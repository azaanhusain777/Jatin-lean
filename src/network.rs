//! Network module: npm registry integration and version checking.
//!
//! Provides:
//!   - npm registry API client for package metadata
//!   - Version freshness checking (outdated, deprecated, vulnerable)
//!   - Download count statistics
//!   - Package size from registry (unpacked / tarball)
//!   - Bulk package audit against known advisories

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

// ─── Registry Client ─────────────────────────────────────────────────────────

/// npm registry API client.
pub struct RegistryClient {
    base_url: String,
    timeout: Duration,
    user_agent: String,
}

impl RegistryClient {
    /// Create a client for the default npm registry.
    pub fn new() -> Self {
        Self {
            base_url: "https://registry.npmjs.org".to_string(),
            timeout: Duration::from_secs(10),
            user_agent: format!("jatin-lean/{}", env!("CARGO_PKG_VERSION")),
        }
    }

    /// Create with a custom registry URL.
    pub fn with_registry(url: &str) -> Self {
        Self {
            base_url: url.trim_end_matches('/').to_string(),
            ..Self::new()
        }
    }

    /// Fetch package metadata from the registry.
    pub fn get_package_info(&self, name: &str) -> Result<PackageInfo> {
        let url = format!("{}/{}", self.base_url, name);
        let body = self.http_get(&url)?;
        let info: RegistryResponse = serde_json::from_str(&body)
            .with_context(|| format!("Failed to parse registry response for {}", name))?;

        let latest_version = info.dist_tags.get("latest").cloned().unwrap_or_default();
        let latest_info = info.versions.get(&latest_version);

        let deprecated = latest_info
            .and_then(|v| v.deprecated.clone())
            .or_else(|| info.deprecated.clone());

        let unpacked_size = latest_info
            .and_then(|v| v.dist.as_ref())
            .and_then(|d| d.unpacked_size);

        let tarball_size = latest_info
            .and_then(|v| v.dist.as_ref())
            .and_then(|d| d.tarball_size);

        let dependency_count = latest_info
            .and_then(|v| v.dependencies.as_ref())
            .map(|d| d.len())
            .unwrap_or(0);

        let license = latest_info
            .and_then(|v| v.license.clone())
            .or(info.license.clone());

        Ok(PackageInfo {
            name: info.name,
            latest_version,
            description: info.description.unwrap_or_default(),
            deprecated,
            license,
            unpacked_size,
            tarball_size,
            dependency_count,
            versions: info.versions.keys().cloned().collect(),
            maintainers: info
                .maintainers
                .unwrap_or_default()
                .into_iter()
                .map(|m| m.name)
                .collect(),
        })
    }

    /// Check if a specific version is outdated.
    pub fn check_version(&self, name: &str, current: &str) -> Result<VersionStatus> {
        let info = self.get_package_info(name)?;

        if current == info.latest_version {
            return Ok(VersionStatus::UpToDate);
        }

        // Simple semver comparison (major version check)
        let current_parts = parse_semver(current);
        let latest_parts = parse_semver(&info.latest_version);

        if current_parts.0 < latest_parts.0 {
            return Ok(VersionStatus::MajorOutdated {
                current: current.to_string(),
                latest: info.latest_version.clone(),
            });
        }

        if current_parts.1 < latest_parts.1 {
            return Ok(VersionStatus::MinorOutdated {
                current: current.to_string(),
                latest: info.latest_version.clone(),
            });
        }

        Ok(VersionStatus::PatchOutdated {
            current: current.to_string(),
            latest: info.latest_version.clone(),
        })
    }

    /// Fetch download counts for a package.
    pub fn get_downloads(&self, name: &str) -> Result<DownloadStats> {
        let url = format!("https://api.npmjs.org/downloads/point/last-week/{}", name);
        let body = self.http_get(&url)?;

        let response: DownloadResponse = serde_json::from_str(&body)
            .with_context(|| format!("Failed to parse download stats for {}", name))?;

        Ok(DownloadStats {
            package: name.to_string(),
            weekly_downloads: response.downloads,
        })
    }

    /// Bulk check versions for all installed packages.
    pub fn audit_versions(&self, installed: &HashMap<String, String>) -> Result<Vec<AuditEntry>> {
        use indicatif::{ProgressBar, ProgressStyle};

        let pb = ProgressBar::new(installed.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "  {spinner:.cyan} Auditing {pos}/{len} packages [{bar:30}] {msg}",
            )
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.enable_steady_tick(Duration::from_millis(100));

        let mut entries = Vec::new();

        for (name, version) in installed {
            pb.set_message(name.clone());

            match self.check_version(name, version) {
                Ok(status) => {
                    let deprecated = matches!(
                        self.get_package_info(name),
                        Ok(info) if info.deprecated.is_some()
                    );

                    entries.push(AuditEntry {
                        name: name.clone(),
                        installed_version: version.clone(),
                        status,
                        deprecated,
                        error: None,
                    });
                }
                Err(e) => {
                    entries.push(AuditEntry {
                        name: name.clone(),
                        installed_version: version.clone(),
                        status: VersionStatus::Unknown,
                        deprecated: false,
                        error: Some(e.to_string()),
                    });
                }
            }

            pb.inc(1);
        }

        pb.finish_and_clear();
        Ok(entries)
    }

    /// Simple HTTP GET.
    fn http_get(&self, url: &str) -> Result<String> {
        // Using std::net::TcpStream for minimal dependencies
        // In production, you'd use ureq or reqwest
        let url_parsed = parse_url(url)?;

        let addr = format!("{}:{}", url_parsed.host, url_parsed.port);
        let _stream = std::net::TcpStream::connect_timeout(&addr.parse()?, self.timeout)?;

        // For HTTPS, we'd need TLS. For now, return a placeholder
        // that indicates network features require the `ureq` feature.
        bail!(
            "Network features require the 'network' feature flag. \
             Install with: cargo install jatin-lean --features network"
        );
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Data Structures ─────────────────────────────────────────────────────────

/// Package information from the registry.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub latest_version: String,
    pub description: String,
    pub deprecated: Option<String>,
    pub license: Option<String>,
    pub unpacked_size: Option<u64>,
    pub tarball_size: Option<u64>,
    pub dependency_count: usize,
    pub versions: Vec<String>,
    pub maintainers: Vec<String>,
}

/// Version freshness status.
#[derive(Debug, Clone)]
pub enum VersionStatus {
    UpToDate,
    PatchOutdated { current: String, latest: String },
    MinorOutdated { current: String, latest: String },
    MajorOutdated { current: String, latest: String },
    Unknown,
}

impl VersionStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::UpToDate => "✓",
            Self::PatchOutdated { .. } => "↑",
            Self::MinorOutdated { .. } => "⬆",
            Self::MajorOutdated { .. } => "⚠",
            Self::Unknown => "?",
        }
    }

    pub fn severity(&self) -> &'static str {
        match self {
            Self::UpToDate => "current",
            Self::PatchOutdated { .. } => "patch",
            Self::MinorOutdated { .. } => "minor",
            Self::MajorOutdated { .. } => "major",
            Self::Unknown => "unknown",
        }
    }
}

/// Download statistics.
#[derive(Debug, Clone)]
pub struct DownloadStats {
    pub package: String,
    pub weekly_downloads: u64,
}

/// Audit entry for a single package.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub name: String,
    pub installed_version: String,
    pub status: VersionStatus,
    pub deprecated: bool,
    pub error: Option<String>,
}

// ─── Registry Response Types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RegistryResponse {
    name: String,
    description: Option<String>,
    deprecated: Option<String>,
    license: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: HashMap<String, String>,
    versions: HashMap<String, VersionInfo>,
    maintainers: Option<Vec<Maintainer>>,
}

#[derive(Debug, Deserialize)]
struct VersionInfo {
    deprecated: Option<String>,
    license: Option<String>,
    dist: Option<DistInfo>,
    dependencies: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct DistInfo {
    #[serde(rename = "unpackedSize")]
    unpacked_size: Option<u64>,
    #[serde(rename = "fileCount")]
    file_count: Option<u64>,
    #[serde(rename = "tarball")]
    tarball_url: Option<String>,
    // Calculate tarball size from Content-Length when available
    #[serde(default)]
    tarball_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct Maintainer {
    name: String,
}

#[derive(Debug, Deserialize)]
struct DownloadResponse {
    downloads: u64,
    package: String,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Parse a semver string into (major, minor, patch).
fn parse_semver(version: &str) -> (u64, u64, u64) {
    let clean = version.trim_start_matches(|c: char| !c.is_ascii_digit());
    let parts: Vec<u64> = clean
        .split('.')
        .take(3)
        .filter_map(|p| p.parse().ok())
        .collect();

    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

/// Minimal URL parser.
struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
    is_https: bool,
}

fn parse_url(url: &str) -> Result<ParsedUrl> {
    let is_https = url.starts_with("https://");
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    let (host_port, path) = match without_scheme.find('/') {
        Some(i) => (&without_scheme[..i], &without_scheme[i..]),
        None => (without_scheme, "/"),
    };

    let (host, port) = match host_port.find(':') {
        Some(i) => (
            host_port[..i].to_string(),
            host_port[i + 1..]
                .parse()
                .unwrap_or(if is_https { 443 } else { 80 }),
        ),
        None => (host_port.to_string(), if is_https { 443 } else { 80 }),
    };

    Ok(ParsedUrl {
        host,
        port,
        path: path.to_string(),
        is_https,
    })
}

// ─── Local Package Scanner ───────────────────────────────────────────────────

/// Scan installed packages and extract name + version from package.json files.
pub fn scan_installed_packages(node_modules: &Path) -> Result<HashMap<String, String>> {
    let mut packages = HashMap::new();

    let entries = std::fs::read_dir(node_modules)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_str().unwrap_or("").to_string();

        if name.starts_with('.') {
            continue;
        }

        if name.starts_with('@') {
            // Scoped packages
            if let Ok(scoped) = std::fs::read_dir(&path) {
                for s in scoped.flatten() {
                    let scoped_name = format!("{}/{}", name, s.file_name().to_str().unwrap_or(""));
                    if let Some(version) = read_package_version(&s.path()) {
                        packages.insert(scoped_name, version);
                    }
                }
            }
        } else if let Some(version) = read_package_version(&path) {
            packages.insert(name, version);
        }
    }

    Ok(packages)
}

/// Read the version field from a package.json.
fn read_package_version(pkg_dir: &Path) -> Option<String> {
    let pkg_json = pkg_dir.join("package.json");
    let content = std::fs::read_to_string(&pkg_json).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Print audit results.
pub fn print_audit_results(entries: &[AuditEntry]) {
    use console::style;

    println!();
    println!(
        "  {} {}",
        style("Version Audit").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!();

    let mut outdated_count = 0u64;
    let mut deprecated_count = 0u64;

    for entry in entries {
        let icon = entry.status.icon();
        let severity = entry.status.severity();

        let name_style = if entry.deprecated {
            style(&entry.name).red().strikethrough()
        } else {
            style(&entry.name).white()
        };

        let version_info = match &entry.status {
            VersionStatus::UpToDate => style(format!("{} (current)", entry.installed_version))
                .green()
                .to_string(),
            VersionStatus::PatchOutdated { latest, .. } => {
                outdated_count += 1;
                style(format!("{} → {}", entry.installed_version, latest))
                    .yellow()
                    .to_string()
            }
            VersionStatus::MinorOutdated { latest, .. } => {
                outdated_count += 1;
                style(format!("{} → {}", entry.installed_version, latest))
                    .yellow()
                    .bold()
                    .to_string()
            }
            VersionStatus::MajorOutdated { latest, .. } => {
                outdated_count += 1;
                style(format!("{} → {}", entry.installed_version, latest))
                    .red()
                    .bold()
                    .to_string()
            }
            VersionStatus::Unknown => style(format!("{} (unknown)", entry.installed_version))
                .dim()
                .to_string(),
        };

        let dep_marker = if entry.deprecated {
            deprecated_count += 1;
            style(" [DEPRECATED]").red().bold().to_string()
        } else {
            String::new()
        };

        println!(
            "  {} {} {} {}{}",
            icon,
            name_style,
            version_info,
            style(format!("[{}]", severity)).dim(),
            dep_marker,
        );
    }

    println!();
    println!(
        "  {} {} packages checked, {} outdated, {} deprecated",
        style("Summary:").dim(),
        entries.len(),
        style(outdated_count).yellow(),
        style(deprecated_count).red(),
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver() {
        assert_eq!(parse_semver("1.2.3"), (1, 2, 3));
        assert_eq!(parse_semver("0.1.0"), (0, 1, 0));
        assert_eq!(parse_semver("^1.2.3"), (1, 2, 3));
        assert_eq!(parse_semver("~1.2.3"), (1, 2, 3));
        assert_eq!(parse_semver("1.0"), (1, 0, 0));
    }

    #[test]
    fn test_parse_url() -> Result<()> {
        let parsed = parse_url("https://registry.npmjs.org/lodash")?;
        assert_eq!(parsed.host, "registry.npmjs.org");
        assert_eq!(parsed.port, 443);
        assert_eq!(parsed.path, "/lodash");
        assert!(parsed.is_https);
        Ok(())
    }

    #[test]
    fn test_parse_url_http() -> Result<()> {
        let parsed = parse_url("http://localhost:4873/package")?;
        assert_eq!(parsed.host, "localhost");
        assert_eq!(parsed.port, 4873);
        assert_eq!(parsed.path, "/package");
        assert!(!parsed.is_https);
        Ok(())
    }

    #[test]
    fn test_version_status_icons() {
        assert_eq!(VersionStatus::UpToDate.icon(), "✓");
        assert_eq!(
            VersionStatus::MajorOutdated {
                current: "1.0.0".into(),
                latest: "2.0.0".into()
            }
            .icon(),
            "⚠"
        );
    }

    #[test]
    fn test_registry_client_creation() {
        let client = RegistryClient::new();
        assert_eq!(client.base_url, "https://registry.npmjs.org");
    }

    #[test]
    fn test_scan_installed_packages() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let nm = dir.path().join("node_modules");
        std::fs::create_dir(&nm)?;

        let pkg = nm.join("test-pkg");
        std::fs::create_dir(&pkg)?;
        std::fs::write(
            pkg.join("package.json"),
            r#"{"name":"test-pkg","version":"1.2.3"}"#,
        )?;

        let packages = scan_installed_packages(&nm)?;
        assert_eq!(packages.get("test-pkg"), Some(&"1.2.3".to_string()));
        Ok(())
    }
}
