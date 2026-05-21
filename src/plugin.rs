//! Plugin system: extensible architecture for custom pruning rules and analyzers.
//!
//! Provides a trait-based plugin API allowing users to write custom analyzers,
//! reporters, and pruning strategies that integrate seamlessly with the core engine.

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::rules::FileCategory;
use crate::scanner::{PruneCandidate, ScanResult};

// ─── Plugin Trait ─────────────────────────────────────────────────────────────

/// A plugin that extends jatin-lean's capabilities.
pub trait Plugin: Send + Sync {
    /// Unique name for this plugin.
    fn name(&self) -> &str;

    /// Version of the plugin.
    fn version(&self) -> &str;

    /// Description of what this plugin does.
    fn description(&self) -> &str;

    /// Called during the scan phase — allows the plugin to add or remove candidates.
    fn on_scan(&self, _candidates: &mut Vec<PruneCandidate>, _root: &Path) -> Result<()> {
        Ok(())
    }

    /// Called before deletion — allows the plugin to veto candidates.
    fn on_pre_delete(&self, _candidates: &[PruneCandidate]) -> Result<Vec<PathBuf>> {
        Ok(Vec::new()) // Return paths to exclude from deletion
    }

    /// Called after deletion — for reporting/logging.
    fn on_post_delete(&self, _deleted: &[PathBuf], _total_bytes: u64) -> Result<()> {
        Ok(())
    }

    /// Custom report generation hook.
    fn generate_report(&self, _scan_result: &ScanResult) -> Result<Option<String>> {
        Ok(None)
    }
}

// ─── Plugin Registry ──────────────────────────────────────────────────────────

/// Registry that manages loaded plugins.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
    enabled: HashMap<String, bool>,
}

impl PluginRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            enabled: HashMap::new(),
        }
    }

    /// Create a registry with all built-in plugins.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(NativeModulePlugin));
        registry.register(Box::new(TypeScriptSourcePlugin));
        registry.register(Box::new(TestFilePlugin));
        registry.register(Box::new(ExampleFilePlugin));
        registry.register(Box::new(BenchmarkFilePlugin));
        registry
    }

    /// Register a plugin.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();
        self.enabled.insert(name, true);
        self.plugins.push(plugin);
    }

    /// Enable or disable a plugin by name.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) {
        if let Some(e) = self.enabled.get_mut(name) {
            *e = enabled;
        }
    }

    /// Get all registered plugins.
    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }

    /// Get active (enabled) plugins.
    pub fn active_plugins(&self) -> Vec<&dyn Plugin> {
        self.plugins
            .iter()
            .filter(|p| *self.enabled.get(p.name()).unwrap_or(&true))
            .map(|p| p.as_ref())
            .collect()
    }

    /// Run all active plugins' scan hooks.
    pub fn run_scan_hooks(&self, candidates: &mut Vec<PruneCandidate>, root: &Path) -> Result<()> {
        for plugin in self.active_plugins() {
            plugin.on_scan(candidates, root)?;
        }
        Ok(())
    }

    /// Run all active plugins' pre-delete hooks and collect exclusions.
    pub fn run_pre_delete_hooks(&self, candidates: &[PruneCandidate]) -> Result<Vec<PathBuf>> {
        let mut exclusions = Vec::new();
        for plugin in self.active_plugins() {
            let excl = plugin.on_pre_delete(candidates)?;
            exclusions.extend(excl);
        }
        Ok(exclusions)
    }

    /// Run all active plugins' post-delete hooks.
    pub fn run_post_delete_hooks(&self, deleted: &[PathBuf], total_bytes: u64) -> Result<()> {
        for plugin in self.active_plugins() {
            plugin.on_post_delete(deleted, total_bytes)?;
        }
        Ok(())
    }

    /// Collect reports from all active plugins.
    pub fn collect_reports(&self, scan_result: &ScanResult) -> Result<Vec<(String, String)>> {
        let mut reports = Vec::new();
        for plugin in self.active_plugins() {
            if let Some(report) = plugin.generate_report(scan_result)? {
                reports.push((plugin.name().to_string(), report));
            }
        }
        Ok(reports)
    }

    /// Get plugin count.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Get active plugin count.
    pub fn active_count(&self) -> usize {
        self.active_plugins().len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

// ─── Built-in Plugins ─────────────────────────────────────────────────────────

/// Plugin that identifies native/compiled modules (.node files, binding.gyp).
pub struct NativeModulePlugin;

impl Plugin for NativeModulePlugin {
    fn name(&self) -> &str {
        "native-modules"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Identifies native/compiled modules and their build artifacts"
    }

    fn on_scan(&self, candidates: &mut Vec<PruneCandidate>, root: &Path) -> Result<()> {
        // Find build directories for native modules
        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(false)
            .max_depth(Some(5))
            .build();

        for entry in walker.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Native build artifacts
            if (name == "build" || name == "Release" || name == "Debug") && path.is_dir() {
                // Check if parent has binding.gyp
                let parent = path.parent().unwrap_or(path);
                if parent.join("binding.gyp").exists() || parent.join("CMakeLists.txt").exists() {
                    // These are build artifacts — mark source files for pruning
                    let build_src = parent.join("src");
                    if build_src.is_dir() {
                        for file_entry in ignore::WalkBuilder::new(&build_src)
                            .hidden(false)
                            .git_ignore(false)
                            .build()
                            .flatten()
                        {
                            if file_entry.file_type().is_some_and(|ft| ft.is_file()) {
                                let file_path = file_entry.path().to_path_buf();
                                let ext =
                                    file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                if matches!(ext, "c" | "cc" | "cpp" | "h" | "hpp" | "gyp" | "gypi")
                                {
                                    if let Ok(meta) = std::fs::metadata(&file_path) {
                                        candidates.push(PruneCandidate {
                                            path: file_path,
                                            size: meta.len(),
                                            category: FileCategory::BuildArtifact,
                                            package_name: parent
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("unknown")
                                                .to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Plugin that identifies TypeScript source files that aren't needed at runtime.
pub struct TypeScriptSourcePlugin;

impl Plugin for TypeScriptSourcePlugin {
    fn name(&self) -> &str {
        "typescript-source"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Identifies TypeScript source files when compiled JS is available"
    }

    fn on_scan(&self, candidates: &mut Vec<PruneCandidate>, root: &Path) -> Result<()> {
        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(false)
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            // .ts file with a corresponding .js file = safe to prune .ts
            if ext == "ts" && !path.to_str().unwrap_or("").ends_with(".d.ts") {
                let js_path = path.with_extension("js");
                if js_path.exists() {
                    if let Ok(meta) = std::fs::metadata(path) {
                        let pkg_name = path
                            .ancestors()
                            .find(|p| p.join("package.json").exists())
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        candidates.push(PruneCandidate {
                            path: path.to_path_buf(),
                            size: meta.len(),
                            category: FileCategory::TypeScriptSource,
                            package_name: pkg_name,
                        });
                    }
                }
            }

            // tsconfig.json files
            if path.file_name().and_then(|n| n.to_str()) == Some("tsconfig.json") {
                if let Ok(meta) = std::fs::metadata(path) {
                    let pkg_name = path
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    candidates.push(PruneCandidate {
                        path: path.to_path_buf(),
                        size: meta.len(),
                        category: FileCategory::CiConfig,
                        package_name: pkg_name,
                    });
                }
            }
        }
        Ok(())
    }
}

/// Plugin that marks test files for pruning.
pub struct TestFilePlugin;

impl Plugin for TestFilePlugin {
    fn name(&self) -> &str {
        "test-files"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Identifies test files and fixtures that aren't needed at runtime"
    }

    fn on_scan(&self, candidates: &mut Vec<PruneCandidate>, root: &Path) -> Result<()> {
        let test_dirs = [
            "test",
            "tests",
            "__tests__",
            "spec",
            "specs",
            "__mocks__",
            "__fixtures__",
        ];

        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(false)
            .max_depth(Some(4))
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }

            let dir_name = entry.file_name().to_str().unwrap_or("");
            if test_dirs.contains(&dir_name) {
                // Walk all files in this test directory
                for file_entry in ignore::WalkBuilder::new(entry.path())
                    .hidden(false)
                    .git_ignore(false)
                    .build()
                    .flatten()
                {
                    if file_entry.file_type().is_some_and(|ft| ft.is_file()) {
                        if let Ok(meta) = std::fs::metadata(file_entry.path()) {
                            let pkg_name = entry
                                .path()
                                .ancestors()
                                .find(|p| p.join("package.json").exists())
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();

                            candidates.push(PruneCandidate {
                                path: file_entry.path().to_path_buf(),
                                size: meta.len(),
                                category: FileCategory::TestAsset,
                                package_name: pkg_name,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Plugin that marks example files for pruning.
pub struct ExampleFilePlugin;

impl Plugin for ExampleFilePlugin {
    fn name(&self) -> &str {
        "example-files"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Identifies example and demo files that aren't needed at runtime"
    }

    fn on_scan(&self, candidates: &mut Vec<PruneCandidate>, root: &Path) -> Result<()> {
        let example_dirs = ["example", "examples", "demo", "demos", "sample", "samples"];

        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(false)
            .max_depth(Some(4))
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }

            let dir_name = entry.file_name().to_str().unwrap_or("").to_lowercase();
            if example_dirs.iter().any(|d| dir_name == *d) {
                for file_entry in ignore::WalkBuilder::new(entry.path())
                    .hidden(false)
                    .git_ignore(false)
                    .build()
                    .flatten()
                {
                    if file_entry.file_type().is_some_and(|ft| ft.is_file()) {
                        if let Ok(meta) = std::fs::metadata(file_entry.path()) {
                            let pkg_name = entry
                                .path()
                                .ancestors()
                                .find(|p| p.join("package.json").exists())
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();

                            candidates.push(PruneCandidate {
                                path: file_entry.path().to_path_buf(),
                                size: meta.len(),
                                category: FileCategory::Documentation,
                                package_name: pkg_name,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Plugin that marks benchmark files for pruning.
pub struct BenchmarkFilePlugin;

impl Plugin for BenchmarkFilePlugin {
    fn name(&self) -> &str {
        "benchmark-files"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Identifies benchmark and performance test files that aren't needed at runtime"
    }

    fn on_scan(&self, candidates: &mut Vec<PruneCandidate>, root: &Path) -> Result<()> {
        let bench_dirs = ["benchmark", "benchmarks", "bench", "perf"];

        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(false)
            .max_depth(Some(4))
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }

            let dir_name = entry.file_name().to_str().unwrap_or("").to_lowercase();
            if bench_dirs.iter().any(|d| dir_name == *d) {
                for file_entry in ignore::WalkBuilder::new(entry.path())
                    .hidden(false)
                    .git_ignore(false)
                    .build()
                    .flatten()
                {
                    if file_entry.file_type().is_some_and(|ft| ft.is_file()) {
                        if let Ok(meta) = std::fs::metadata(file_entry.path()) {
                            let pkg_name = entry
                                .path()
                                .ancestors()
                                .find(|p| p.join("package.json").exists())
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();

                            candidates.push(PruneCandidate {
                                path: file_entry.path().to_path_buf(),
                                size: meta.len(),
                                category: FileCategory::Documentation,
                                package_name: pkg_name,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Print plugin registry information.
pub fn print_plugin_info(registry: &PluginRegistry) {
    use console::style;

    println!();
    println!(
        "  {} {}",
        style("Plugin Registry").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!();

    for plugin in registry.plugins() {
        let is_active = registry
            .active_plugins()
            .iter()
            .any(|p| p.name() == plugin.name());
        let status = if is_active {
            style("● active").green()
        } else {
            style("○ disabled").dim()
        };

        println!(
            "  {} {} v{} — {} [{}]",
            style("▸").dim(),
            style(plugin.name()).white().bold(),
            plugin.version(),
            style(plugin.description()).dim(),
            status,
        );
    }

    println!();
    println!(
        "  {} {}/{} plugins active",
        style("◉").cyan(),
        style(registry.active_count()).white().bold(),
        registry.count(),
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockPlugin;
    impl Plugin for MockPlugin {
        fn name(&self) -> &str {
            "mock"
        }
        fn version(&self) -> &str {
            "0.1.0"
        }
        fn description(&self) -> &str {
            "A test plugin"
        }
    }

    #[test]
    fn test_plugin_registry_new() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_plugin_registry_register() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(MockPlugin));
        assert_eq!(registry.count(), 1);
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn test_plugin_registry_disable() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(MockPlugin));
        registry.set_enabled("mock", false);
        assert_eq!(registry.count(), 1);
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_plugin_registry_with_builtins() {
        let registry = PluginRegistry::with_builtins();
        assert_eq!(registry.count(), 5);
        assert_eq!(registry.active_count(), 5);
    }

    #[test]
    fn test_builtin_plugin_names() {
        let registry = PluginRegistry::with_builtins();
        let names: Vec<&str> = registry.plugins().iter().map(|p| p.name()).collect();
        assert!(names.contains(&"native-modules"));
        assert!(names.contains(&"typescript-source"));
        assert!(names.contains(&"test-files"));
        assert!(names.contains(&"example-files"));
        assert!(names.contains(&"benchmark-files"));
    }
}
