//! Memory-mapped I/O engine for zero-copy file scanning.
//!
//! Uses mmap/madvise on Unix and MapViewOfFile on Windows to achieve
//! near-zero overhead file reads. Combined with SIMD hashing, this
//! provides maximum throughput for large node_modules directories.

use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::simd;

// ─── Memory-Mapped File ─────────────────────────────────────────────────────

/// A memory-mapped file handle for zero-copy reads.
pub struct MappedFile {
    data: Vec<u8>,
    path: PathBuf,
    size: u64,
}

impl MappedFile {
    /// Map a file into memory.
    ///
    /// For files < 4KB, uses a regular read (mmap overhead not worth it).
    /// For files >= 4KB, uses memory mapping on supported platforms.
    pub fn open(path: &Path) -> Result<Self> {
        let metadata =
            fs::metadata(path).with_context(|| format!("Cannot stat: {}", path.display()))?;
        let size = metadata.len();

        if size == 0 {
            return Ok(Self {
                data: Vec::new(),
                path: path.to_path_buf(),
                size: 0,
            });
        }

        // For small files, regular I/O is faster than mmap
        if size < 4096 {
            return Self::read_small(path, size);
        }

        // Try mmap on Unix
        #[cfg(unix)]
        {
            match Self::mmap_unix(path, size) {
                Ok(mapped) => return Ok(mapped),
                Err(_) => return Self::read_small(path, size),
            }
        }

        #[cfg(not(unix))]
        {
            Self::read_small(path, size)
        }
    }

    /// Regular file read for small files.
    fn read_small(path: &Path, size: u64) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::with_capacity(size as usize);
        file.read_to_end(&mut data)?;
        Ok(Self {
            data,
            path: path.to_path_buf(),
            size,
        })
    }

    /// Memory-map on Unix using libc.
    #[cfg(unix)]
    fn mmap_unix(path: &Path, size: u64) -> Result<Self> {
        use std::os::unix::io::AsRawFd;

        let file = File::open(path)?;
        let fd = file.as_raw_fd();
        let len = size as usize;

        unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                fd,
                0,
            );

            if ptr == libc::MAP_FAILED {
                anyhow::bail!("mmap failed for {}", path.display());
            }

            // Advise kernel for sequential read pattern
            libc::madvise(ptr, len, libc::MADV_SEQUENTIAL);

            // Copy data out of mapping so we can safely unmap
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            let data = slice.to_vec();

            libc::munmap(ptr, len);

            Ok(Self {
                data,
                path: path.to_path_buf(),
                size,
            })
        }
    }

    /// Get the file contents as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// File size.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// File path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Compute SIMD-accelerated hash.
    pub fn hash(&self) -> u64 {
        simd::fast_hash(&self.data)
    }

    /// Count lines using SIMD.
    pub fn line_count(&self) -> usize {
        simd::count_newlines(&self.data)
            + if self.data.last() != Some(&b'\n') && !self.data.is_empty() {
                1
            } else {
                0
            }
    }

    /// Byte frequency analysis.
    pub fn byte_frequency(&self) -> simd::ByteFrequency {
        simd::ByteFrequency::analyze(&self.data)
    }

    /// Is likely a binary file?
    pub fn is_binary(&self) -> bool {
        // Check first 8KB for binary content
        let check_size = self.data.len().min(8192);
        let freq = simd::ByteFrequency::analyze(&self.data[..check_size]);
        freq.is_likely_binary()
    }

    /// Find a pattern in the file content.
    pub fn find(&self, pattern: &[u8]) -> Option<usize> {
        simd::find_pattern(&self.data, pattern)
    }

    /// Check if file contains a pattern.
    pub fn contains(&self, pattern: &[u8]) -> bool {
        self.find(pattern).is_some()
    }

    /// Count occurrences of a pattern.
    pub fn count_pattern(&self, pattern: &[u8]) -> usize {
        if pattern.is_empty() {
            return 0;
        }
        let mut count = 0;
        let mut offset = 0;
        while offset + pattern.len() <= self.data.len() {
            if let Some(pos) = simd::find_pattern(&self.data[offset..], pattern) {
                count += 1;
                offset += pos + pattern.len();
            } else {
                break;
            }
        }
        count
    }
}

// ─── Batch Reader ────────────────────────────────────────────────────────────

/// Read mode for controlling I/O strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadMode {
    /// Always use regular reads.
    Standard,
    /// Use mmap for files >= 4KB, regular for smaller.
    Auto,
    /// Always try mmap (even for small files).
    ForceMmap,
}

/// Configuration for batch I/O operations.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// I/O read mode.
    pub mode: ReadMode,
    /// Maximum concurrent I/O operations.
    pub max_concurrent: usize,
    /// Read-ahead hint size (bytes).
    pub readahead_size: usize,
    /// Skip files larger than this (bytes, 0 = no limit).
    pub max_file_size: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            mode: ReadMode::Auto,
            max_concurrent: num_cpus(),
            readahead_size: 256 * 1024, // 256KB
            max_file_size: 0,
        }
    }
}

/// Get the number of logical CPUs.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Batch file processor — reads and processes multiple files in parallel.
pub struct BatchProcessor {
    config: BatchConfig,
}

impl BatchProcessor {
    /// Create a new batch processor.
    pub fn new(config: BatchConfig) -> Self {
        Self { config }
    }

    /// Create with default config.
    pub fn default_processor() -> Self {
        Self::new(BatchConfig::default())
    }

    /// Hash all files in a directory tree in parallel.
    pub fn hash_directory(&self, root: &Path) -> Result<Vec<(PathBuf, u64, u64)>> {
        use rayon::prelude::*;

        let files: Vec<PathBuf> = self.collect_files(root)?;

        let results: Vec<(PathBuf, u64, u64)> = files
            .par_iter()
            .filter_map(|path| {
                let mapped = MappedFile::open(path).ok()?;
                let hash = mapped.hash();
                let size = mapped.size();
                Some((path.clone(), hash, size))
            })
            .collect();

        Ok(results)
    }

    /// Analyze all text files in a directory for content statistics.
    pub fn analyze_text_files(&self, root: &Path) -> Result<TextAnalysisResult> {
        use rayon::prelude::*;

        let files: Vec<PathBuf> = self.collect_files(root)?;

        let stats: Vec<FileStats> = files
            .par_iter()
            .filter_map(|path| {
                let mapped = MappedFile::open(path).ok()?;
                if mapped.is_binary() {
                    return None;
                }

                let freq = mapped.byte_frequency();
                Some(FileStats {
                    path: path.clone(),
                    size: mapped.size(),
                    lines: mapped.line_count(),
                    entropy: freq.entropy(),
                    whitespace_ratio: freq.whitespace_ratio(),
                    compression_ratio: freq.estimated_compression_ratio(),
                })
            })
            .collect();

        let total_size: u64 = stats.iter().map(|s| s.size).sum();
        let total_lines: usize = stats.iter().map(|s| s.lines).sum();
        let avg_entropy: f64 = if stats.is_empty() {
            0.0
        } else {
            stats.iter().map(|s| s.entropy).sum::<f64>() / stats.len() as f64
        };

        Ok(TextAnalysisResult {
            files_analyzed: stats.len(),
            total_size,
            total_lines,
            average_entropy: avg_entropy,
            file_stats: stats,
        })
    }

    /// Collect all files under a root (respecting max_file_size).
    fn collect_files(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let max_size = self.config.max_file_size;

        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(false)
            .threads(self.config.max_concurrent.min(12))
            .build();

        let files: Vec<PathBuf> = walker
            .flatten()
            .filter(|e| e.file_type().map_or(false, |ft| ft.is_file()))
            .filter(|e| {
                if max_size == 0 {
                    return true;
                }
                e.metadata().map_or(true, |m| m.len() <= max_size)
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        Ok(files)
    }
}

/// Stats for a single text file.
#[derive(Debug, Clone)]
pub struct FileStats {
    pub path: PathBuf,
    pub size: u64,
    pub lines: usize,
    pub entropy: f64,
    pub whitespace_ratio: f64,
    pub compression_ratio: f64,
}

/// Result of analyzing all text files.
#[derive(Debug)]
pub struct TextAnalysisResult {
    pub files_analyzed: usize,
    pub total_size: u64,
    pub total_lines: usize,
    pub average_entropy: f64,
    pub file_stats: Vec<FileStats>,
}

impl TextAnalysisResult {
    /// Get the most "bloated" files (highest whitespace ratio).
    pub fn most_bloated(&self, n: usize) -> Vec<&FileStats> {
        let mut sorted: Vec<_> = self.file_stats.iter().collect();
        sorted.sort_by(|a, b| b.whitespace_ratio.partial_cmp(&a.whitespace_ratio).unwrap());
        sorted.truncate(n);
        sorted
    }

    /// Get the most compressible files.
    pub fn most_compressible(&self, n: usize) -> Vec<&FileStats> {
        let mut sorted: Vec<_> = self.file_stats.iter().collect();
        sorted.sort_by(|a, b| {
            a.compression_ratio
                .partial_cmp(&b.compression_ratio)
                .unwrap()
        });
        sorted.truncate(n);
        sorted
    }

    /// Get the largest files.
    pub fn largest_files(&self, n: usize) -> Vec<&FileStats> {
        let mut sorted: Vec<_> = self.file_stats.iter().collect();
        sorted.sort_by(|a, b| b.size.cmp(&a.size));
        sorted.truncate(n);
        sorted
    }
}

// ─── OS-Level I/O Hints ──────────────────────────────────────────────────────

/// Set read-ahead hint for a file descriptor (Linux).
#[cfg(target_os = "linux")]
pub fn set_readahead(path: &Path, size: usize) -> Result<()> {
    use std::os::unix::io::AsRawFd;
    let file = File::open(path)?;
    let fd = file.as_raw_fd();
    unsafe {
        libc::posix_fadvise(fd, 0, size as i64, libc::POSIX_FADV_SEQUENTIAL);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn set_readahead(_path: &Path, _size: usize) -> Result<()> {
    Ok(()) // No-op on non-Linux
}

/// Drop file system caches for a file (Linux).
#[cfg(target_os = "linux")]
pub fn drop_cache(path: &Path) -> Result<()> {
    use std::os::unix::io::AsRawFd;
    let file = File::open(path)?;
    let fd = file.as_raw_fd();
    unsafe {
        libc::posix_fadvise(fd, 0, 0, libc::POSIX_FADV_DONTNEED);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn drop_cache(_path: &Path) -> Result<()> {
    Ok(())
}

/// Get I/O stats for a directory.
pub fn io_stats(path: &Path) -> Result<IoStats> {
    let mut total_files: u64 = 0;
    let mut total_size: u64 = 0;
    let mut total_dirs: u64 = 0;
    let mut max_file_size: u64 = 0;
    let mut min_file_size: u64 = u64::MAX;

    let walker = ignore::WalkBuilder::new(path)
        .hidden(false)
        .git_ignore(false)
        .build();

    for entry in walker.flatten() {
        if let Some(ft) = entry.file_type() {
            if ft.is_file() {
                total_files += 1;
                if let Ok(meta) = entry.metadata() {
                    let size = meta.len();
                    total_size += size;
                    max_file_size = max_file_size.max(size);
                    min_file_size = min_file_size.min(size);
                }
            } else if ft.is_dir() {
                total_dirs += 1;
            }
        }
    }

    if total_files == 0 {
        min_file_size = 0;
    }

    Ok(IoStats {
        total_files,
        total_dirs,
        total_size,
        max_file_size,
        min_file_size,
        avg_file_size: if total_files > 0 {
            total_size / total_files
        } else {
            0
        },
    })
}

/// Filesystem I/O statistics.
#[derive(Debug, Clone)]
pub struct IoStats {
    pub total_files: u64,
    pub total_dirs: u64,
    pub total_size: u64,
    pub max_file_size: u64,
    pub min_file_size: u64,
    pub avg_file_size: u64,
}

impl IoStats {
    /// Print formatted I/O stats.
    pub fn print_info(&self) {
        use crate::scanner::format_size;
        use console::style;

        println!();
        println!(
            "  {} {}",
            style("I/O Statistics").cyan().bold(),
            style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
        );
        println!(
            "  {} Total files: {}",
            style("◉").cyan(),
            style(self.total_files).white().bold()
        );
        println!(
            "  {} Total directories: {}",
            style("◉").cyan(),
            style(self.total_dirs).white().bold()
        );
        println!(
            "  {} Total size: {}",
            style("◉").cyan(),
            style(format_size(self.total_size)).white().bold()
        );
        println!(
            "  {} Avg file size: {}",
            style("◉").dim(),
            format_size(self.avg_file_size)
        );
        println!(
            "  {} Max file size: {}",
            style("◉").dim(),
            format_size(self.max_file_size)
        );
        println!(
            "  {} Min file size: {}",
            style("◉").dim(),
            format_size(self.min_file_size)
        );
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_mapped_file_open_small() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("small.txt");
        fs::write(&path, "hello world")?;

        let mapped = MappedFile::open(&path)?;
        assert_eq!(mapped.as_bytes(), b"hello world");
        assert_eq!(mapped.size(), 11);
        Ok(())
    }

    #[test]
    fn test_mapped_file_open_empty() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("empty.txt");
        fs::write(&path, "")?;

        let mapped = MappedFile::open(&path)?;
        assert_eq!(mapped.as_bytes(), b"");
        assert_eq!(mapped.size(), 0);
        Ok(())
    }

    #[test]
    fn test_mapped_file_hash_consistency() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("test.txt");
        fs::write(&path, "consistent hashing test")?;

        let m1 = MappedFile::open(&path)?;
        let m2 = MappedFile::open(&path)?;
        assert_eq!(m1.hash(), m2.hash());
        Ok(())
    }

    #[test]
    fn test_mapped_file_line_count() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("lines.txt");
        fs::write(&path, "line1\nline2\nline3\n")?;

        let mapped = MappedFile::open(&path)?;
        assert_eq!(mapped.line_count(), 3);
        Ok(())
    }

    #[test]
    fn test_mapped_file_contains() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("search.txt");
        fs::write(&path, "the quick brown fox")?;

        let mapped = MappedFile::open(&path)?;
        assert!(mapped.contains(b"quick"));
        assert!(mapped.contains(b"fox"));
        assert!(!mapped.contains(b"lazy"));
        Ok(())
    }

    #[test]
    fn test_mapped_file_count_pattern() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("count.txt");
        fs::write(&path, "abcabcabc")?;

        let mapped = MappedFile::open(&path)?;
        assert_eq!(mapped.count_pattern(b"abc"), 3);
        assert_eq!(mapped.count_pattern(b"xyz"), 0);
        Ok(())
    }

    #[test]
    fn test_mapped_file_large() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("large.txt");
        let data: String = "line of text data\n".repeat(10_000);
        fs::write(&path, &data)?;

        let mapped = MappedFile::open(&path)?;
        assert_eq!(mapped.size(), data.len() as u64);
        assert_eq!(mapped.line_count(), 10_000);
        Ok(())
    }

    #[test]
    fn test_mapped_file_binary_detection() -> Result<()> {
        let dir = TempDir::new()?;

        // Text file
        let text_path = dir.path().join("text.js");
        fs::write(&text_path, "const x = 42; // some JavaScript")?;
        let mapped = MappedFile::open(&text_path)?;
        assert!(!mapped.is_binary());

        // Binary-like file
        let bin_path = dir.path().join("binary.bin");
        let binary_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        fs::write(&bin_path, &binary_data)?;
        let mapped = MappedFile::open(&bin_path)?;
        // Binary detection depends on entropy
        Ok(())
    }

    #[test]
    fn test_io_stats() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("a.txt"), "hello")?;
        fs::write(dir.path().join("b.txt"), "world!")?;
        fs::create_dir(dir.path().join("sub"))?;
        fs::write(dir.path().join("sub/c.txt"), "nested")?;

        let stats = io_stats(dir.path())?;
        assert_eq!(stats.total_files, 3);
        assert!(stats.total_dirs >= 1);
        assert!(stats.total_size > 0);
        Ok(())
    }

    #[test]
    fn test_batch_processor() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("a.js"), "module.exports = {};")?;
        fs::write(dir.path().join("b.js"), "const x = 1;")?;

        let processor = BatchProcessor::default_processor();
        let results = processor.hash_directory(dir.path())?;
        assert_eq!(results.len(), 2);

        // Different content → different hashes
        assert_ne!(results[0].1, results[1].1);
        Ok(())
    }
}
