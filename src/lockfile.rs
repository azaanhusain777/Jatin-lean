//! Lock file parser: understands actual dependencies from package-lock.json / yarn.lock.
//!
//! Parses lock files to build an accurate dependency graph, enabling
//! smarter pruning decisions based on what packages are truly required.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Represents the dependency graph extracted from a lock file.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// All direct dependencies (from package.json)
    pub direct_deps: HashSet<String>,
    /// All transitive dependencies (from lock file)
    pub all_deps: HashSet<String>,
    /// Dependency → version mapping
    pub versions: HashMap<String, String>,
    /// Dependency → its dependencies
    pub dep_tree: HashMap<String, Vec<String>>,
    /// Source of the graph (npm, yarn, pnpm)
    pub source: LockFileType,
}

/// Supported lock file types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LockFileType {
    NpmV2,
    NpmV3,
    Yarn,
    Pnpm,
    Unknown,
}

impl std::fmt::Display for LockFileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockFileType::NpmV2 => write!(f, "npm (v2)"),
            LockFileType::NpmV3 => write!(f, "npm (v3)"),
            LockFileType::Yarn => write!(f, "Yarn"),
            LockFileType::Pnpm => write!(f, "pnpm"),
            LockFileType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Minimal representation of package-lock.json (npm v2/v3)
#[derive(Debug, Deserialize)]
struct NpmLockfile {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: Option<u32>,
    dependencies: Option<HashMap<String, NpmDependency>>,
    packages: Option<HashMap<String, NpmPackageEntry>>,
}

#[derive(Debug, Deserialize)]
struct NpmDependency {
    version: Option<String>,
    #[serde(default)]
    resolved: Option<String>,
    #[serde(default)]
    requires: Option<HashMap<String, String>>,
    #[serde(default)]
    dependencies: Option<HashMap<String, NpmDependency>>,
    #[serde(default)]
    dev: Option<bool>,
    #[serde(default)]
    optional: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct NpmPackageEntry {
    version: Option<String>,
    #[serde(default)]
    resolved: Option<String>,
    #[serde(default)]
    dependencies: Option<HashMap<String, String>>,
    #[serde(default)]
    dev: Option<bool>,
    #[serde(default)]
    optional: Option<bool>,
}

/// Minimal representation of package.json
#[derive(Debug, Deserialize)]
struct PackageJson {
    #[serde(default)]
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies", default)]
    dev_dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "peerDependencies", default)]
    peer_dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "optionalDependencies", default)]
    optional_dependencies: Option<HashMap<String, String>>,
}

impl DependencyGraph {
    /// Build a dependency graph from the project directory.
    pub fn from_project(project_dir: &Path) -> Result<Self> {
        // Read package.json for direct dependencies
        let pkg_json_path = project_dir.join("package.json");
        let direct_deps = if pkg_json_path.exists() {
            let content = fs::read_to_string(&pkg_json_path)?;
            let pkg: PackageJson = serde_json::from_str(&content)?;
            let mut deps = HashSet::new();
            if let Some(d) = &pkg.dependencies {
                deps.extend(d.keys().cloned());
            }
            if let Some(d) = &pkg.dev_dependencies {
                deps.extend(d.keys().cloned());
            }
            if let Some(d) = &pkg.peer_dependencies {
                deps.extend(d.keys().cloned());
            }
            if let Some(d) = &pkg.optional_dependencies {
                deps.extend(d.keys().cloned());
            }
            deps
        } else {
            HashSet::new()
        };

        // Try lock files in order: package-lock.json, yarn.lock, pnpm-lock.yaml
        let npm_lock = project_dir.join("package-lock.json");
        let yarn_lock = project_dir.join("yarn.lock");
        let pnpm_lock = project_dir.join("pnpm-lock.yaml");

        if npm_lock.exists() {
            Self::from_npm_lockfile(&npm_lock, direct_deps)
        } else if yarn_lock.exists() {
            Self::from_yarn_lockfile(&yarn_lock, direct_deps)
        } else if pnpm_lock.exists() {
            // pnpm lock parsing is complex; return a basic graph
            Ok(Self {
                direct_deps: direct_deps.clone(),
                all_deps: direct_deps,
                versions: HashMap::new(),
                dep_tree: HashMap::new(),
                source: LockFileType::Pnpm,
            })
        } else {
            // No lock file found
            Ok(Self {
                direct_deps: direct_deps.clone(),
                all_deps: direct_deps,
                versions: HashMap::new(),
                dep_tree: HashMap::new(),
                source: LockFileType::Unknown,
            })
        }
    }

    /// Parse an npm package-lock.json file.
    fn from_npm_lockfile(path: &Path, direct_deps: HashSet<String>) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;

        let lockfile: NpmLockfile =
            serde_json::from_str(&content).with_context(|| "Failed to parse package-lock.json")?;

        let lock_version = lockfile.lockfile_version.unwrap_or(1);
        let source = if lock_version >= 3 {
            LockFileType::NpmV3
        } else {
            LockFileType::NpmV2
        };

        let mut all_deps = HashSet::new();
        let mut versions = HashMap::new();
        let mut dep_tree: HashMap<String, Vec<String>> = HashMap::new();

        // Parse v2 format (dependencies field)
        if let Some(deps) = &lockfile.dependencies {
            parse_npm_deps_recursive(deps, &mut all_deps, &mut versions, &mut dep_tree);
        }

        // Parse v3 format (packages field)
        if let Some(packages) = &lockfile.packages {
            for (key, entry) in packages {
                if key.is_empty() || key == "" {
                    continue; // Skip root package
                }

                // Extract package name from key like "node_modules/lodash"
                let pkg_name = key.strip_prefix("node_modules/").unwrap_or(key);

                // Skip nested node_modules
                if pkg_name.contains("node_modules/") {
                    continue;
                }

                all_deps.insert(pkg_name.to_string());

                if let Some(version) = &entry.version {
                    versions.insert(pkg_name.to_string(), version.clone());
                }

                if let Some(deps) = &entry.dependencies {
                    let dep_names: Vec<String> = deps.keys().cloned().collect();
                    dep_tree.insert(pkg_name.to_string(), dep_names);
                }
            }
        }

        Ok(Self {
            direct_deps,
            all_deps,
            versions,
            dep_tree,
            source,
        })
    }

    /// Parse a yarn.lock file (simplified — yarn v1 format).
    fn from_yarn_lockfile(path: &Path, direct_deps: HashSet<String>) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;

        let mut all_deps = HashSet::new();
        let versions = HashMap::new();
        let mut dep_tree: HashMap<String, Vec<String>> = HashMap::new();

        let mut current_pkg: Option<String> = None;
        let mut current_deps: Vec<String> = Vec::new();
        let mut in_dependencies = false;

        for line in content.lines() {
            // Package header like: "lodash@^4.17.21:"
            if !line.starts_with(' ') && !line.starts_with('#') && line.contains('@') {
                // Save previous package
                if let Some(ref pkg) = current_pkg {
                    if !current_deps.is_empty() {
                        dep_tree.insert(pkg.clone(), current_deps.clone());
                    }
                }

                // Parse new package name
                let pkg_name = parse_yarn_package_name(line);
                if let Some(name) = pkg_name {
                    all_deps.insert(name.clone());
                    current_pkg = Some(name);
                    current_deps = Vec::new();
                    in_dependencies = false;
                }
            } else if line.trim() == "dependencies:" {
                in_dependencies = true;
            } else if in_dependencies && line.starts_with("    ") {
                // Dependency entry like: '    lodash "^4.17.21"'
                let trimmed = line.trim();
                if let Some(dep_name) = trimmed.split_whitespace().next() {
                    let clean_name = dep_name.trim_matches('"').to_string();
                    current_deps.push(clean_name);
                }
            } else if !line.starts_with(' ') {
                in_dependencies = false;
            }
        }

        // Save last package
        if let Some(ref pkg) = current_pkg {
            if !current_deps.is_empty() {
                dep_tree.insert(pkg.clone(), current_deps);
            }
        }

        Ok(Self {
            direct_deps,
            all_deps,
            versions,
            dep_tree,
            source: LockFileType::Yarn,
        })
    }

    /// Check if a package is a production dependency.
    pub fn is_production_dep(&self, package_name: &str) -> bool {
        self.all_deps.contains(package_name)
    }

    /// Check if a package is a direct dependency.
    pub fn is_direct_dep(&self, package_name: &str) -> bool {
        self.direct_deps.contains(package_name)
    }

    /// Get the dependency chain to reach a package from roots.
    pub fn dep_chain(&self, target: &str) -> Vec<Vec<String>> {
        let mut chains = Vec::new();
        let mut visited = HashSet::new();
        self.find_chains_recursive(
            &self.direct_deps,
            target,
            &mut Vec::new(),
            &mut chains,
            &mut visited,
        );
        chains
    }

    fn find_chains_recursive(
        &self,
        current_set: &HashSet<String>,
        target: &str,
        current_chain: &mut Vec<String>,
        chains: &mut Vec<Vec<String>>,
        visited: &mut HashSet<String>,
    ) {
        for dep in current_set {
            if visited.contains(dep) {
                continue;
            }
            visited.insert(dep.clone());
            current_chain.push(dep.clone());

            if dep == target {
                chains.push(current_chain.clone());
            } else if let Some(children) = self.dep_tree.get(dep) {
                let child_set: HashSet<String> = children.iter().cloned().collect();
                self.find_chains_recursive(&child_set, target, current_chain, chains, visited);
            }

            current_chain.pop();
            visited.remove(dep);
        }
    }

    /// Get packages that are installed but not in the dependency graph.
    pub fn find_orphaned_packages(&self, node_modules_path: &Path) -> Result<Vec<String>> {
        let mut orphans = Vec::new();

        if let Ok(entries) = fs::read_dir(node_modules_path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_str().unwrap_or("");

                if name_str.starts_with('.') || name_str == ".bin" {
                    continue;
                }

                if name_str.starts_with('@') {
                    // Scoped packages
                    if let Ok(scoped) = fs::read_dir(entry.path()) {
                        for s in scoped.flatten() {
                            let scoped_name =
                                format!("{}/{}", name_str, s.file_name().to_str().unwrap_or(""));
                            if !self.all_deps.contains(&scoped_name) {
                                orphans.push(scoped_name);
                            }
                        }
                    }
                } else if !self.all_deps.contains(name_str) {
                    orphans.push(name_str.to_string());
                }
            }
        }

        orphans.sort();
        Ok(orphans)
    }

    /// Get total number of dependencies.
    pub fn total_deps(&self) -> usize {
        self.all_deps.len()
    }

    /// Get total number of direct dependencies.
    pub fn direct_dep_count(&self) -> usize {
        self.direct_deps.len()
    }

    /// Get transitive-only dependencies (not direct).
    pub fn transitive_only(&self) -> HashSet<&str> {
        self.all_deps
            .iter()
            .filter(|d| !self.direct_deps.contains(*d))
            .map(|d| d.as_str())
            .collect()
    }
}

/// Parse npm dependencies recursively (v2 format).
fn parse_npm_deps_recursive(
    deps: &HashMap<String, NpmDependency>,
    all_deps: &mut HashSet<String>,
    versions: &mut HashMap<String, String>,
    dep_tree: &mut HashMap<String, Vec<String>>,
) {
    for (name, dep) in deps {
        all_deps.insert(name.clone());

        if let Some(version) = &dep.version {
            versions.insert(name.clone(), version.clone());
        }

        if let Some(requires) = &dep.requires {
            let req_names: Vec<String> = requires.keys().cloned().collect();
            dep_tree.insert(name.clone(), req_names);
        }

        // Recurse into nested dependencies
        if let Some(nested) = &dep.dependencies {
            parse_npm_deps_recursive(nested, all_deps, versions, dep_tree);
        }
    }
}

/// Parse a yarn lock entry to extract the package name.
fn parse_yarn_package_name(line: &str) -> Option<String> {
    let cleaned = line.trim_end_matches(':').trim();

    // Handle multiple version specs: "pkg@^1.0.0, pkg@~1.0.0:"
    let first_spec = cleaned.split(',').next()?.trim().trim_matches('"');

    // Split on '@' but handle scoped packages (@scope/name@version)
    if first_spec.starts_with('@') {
        // Scoped package: @scope/name@version
        let without_at = &first_spec[1..];
        if let Some(at_pos) = without_at.find('@') {
            Some(format!("@{}", &without_at[..at_pos]))
        } else {
            Some(first_spec.to_string())
        }
    } else {
        // Regular package: name@version
        first_spec.split('@').next().map(|s| s.to_string())
    }
}

/// Print dependency graph summary to the terminal.
pub fn print_dep_graph_summary(graph: &DependencyGraph, node_modules_path: &Path) {
    use crate::scanner::format_number;
    use console::style;

    println!();
    println!(
        "  {} {}",
        style("Dependency Graph Analysis").cyan().bold(),
        style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
    );
    println!();

    println!(
        "  {} Lock file type: {}",
        style("◉").cyan(),
        style(&graph.source).white().bold()
    );
    println!(
        "  {} Direct dependencies: {}",
        style("◉").cyan(),
        style(format_number(graph.direct_dep_count() as u64))
            .white()
            .bold()
    );
    println!(
        "  {} Total dependencies: {}",
        style("◉").cyan(),
        style(format_number(graph.total_deps() as u64))
            .white()
            .bold()
    );

    let transitive = graph.transitive_only();
    println!(
        "  {} Transitive dependencies: {}",
        style("◉").yellow(),
        style(format_number(transitive.len() as u64))
            .yellow()
            .bold()
    );

    // Find orphaned packages
    if let Ok(orphans) = graph.find_orphaned_packages(node_modules_path) {
        if !orphans.is_empty() {
            println!(
                "  {} Orphaned packages: {}",
                style("◉").red(),
                style(format_number(orphans.len() as u64)).red().bold()
            );
            println!();
            println!(
                "  {} {}",
                style("Orphaned Packages").white().bold(),
                style("─────────────────────────").dim()
            );
            for orphan in orphans.iter().take(15) {
                println!("  {} {}", style("▸").dim(), style(orphan).yellow());
            }
            if orphans.len() > 15 {
                println!("  {} ...and {} more", style("→").dim(), orphans.len() - 15);
            }
        }
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yarn_package_name_simple() {
        assert_eq!(
            parse_yarn_package_name("lodash@^4.17.21:"),
            Some("lodash".to_string())
        );
    }

    #[test]
    fn test_parse_yarn_package_name_scoped() {
        assert_eq!(
            parse_yarn_package_name("\"@babel/core@^7.0.0\":"),
            Some("@babel/core".to_string())
        );
    }

    #[test]
    fn test_parse_yarn_package_name_multiple() {
        assert_eq!(
            parse_yarn_package_name("lodash@^4.17.21, lodash@~4.17.0:"),
            Some("lodash".to_string())
        );
    }

    #[test]
    fn test_dependency_graph_basic() {
        let graph = DependencyGraph {
            direct_deps: HashSet::from(["react".into(), "lodash".into()]),
            all_deps: HashSet::from([
                "react".into(),
                "lodash".into(),
                "loose-envify".into(),
                "js-tokens".into(),
            ]),
            versions: HashMap::new(),
            dep_tree: HashMap::from([
                ("react".into(), vec!["loose-envify".into()]),
                ("loose-envify".into(), vec!["js-tokens".into()]),
            ]),
            source: LockFileType::NpmV3,
        };

        assert_eq!(graph.direct_dep_count(), 2);
        assert_eq!(graph.total_deps(), 4);
        assert!(graph.is_production_dep("loose-envify"));
        assert!(graph.is_direct_dep("react"));
        assert!(!graph.is_direct_dep("js-tokens"));
    }

    #[test]
    fn test_transitive_only() {
        let graph = DependencyGraph {
            direct_deps: HashSet::from(["react".into()]),
            all_deps: HashSet::from(["react".into(), "loose-envify".into()]),
            versions: HashMap::new(),
            dep_tree: HashMap::new(),
            source: LockFileType::NpmV3,
        };

        let transitive = graph.transitive_only();
        assert!(transitive.contains("loose-envify"));
        assert!(!transitive.contains("react"));
    }

    #[test]
    fn test_lock_file_type_display() {
        assert_eq!(format!("{}", LockFileType::NpmV3), "npm (v3)");
        assert_eq!(format!("{}", LockFileType::Yarn), "Yarn");
        assert_eq!(format!("{}", LockFileType::Pnpm), "pnpm");
    }
}
