//! Configuration file support for custom pruning rules.
//!
//! Allows users to customize what gets deleted via a rules.toml file.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Configuration structure matching rules.toml format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Whether to completely override default rules instead of merging
    #[serde(default)]
    pub override_defaults: bool,

    /// Documentation file patterns
    #[serde(default)]
    pub doc_files: Vec<String>,

    /// Documentation directories
    #[serde(default)]
    pub doc_dirs: Vec<String>,

    /// Test directories
    #[serde(default)]
    pub test_dirs: Vec<String>,

    /// Test file patterns (regex)
    #[serde(default)]
    pub test_patterns: Vec<String>,

    /// Build artifact extensions
    #[serde(default)]
    pub build_extensions: Vec<String>,

    /// Build artifact filenames
    #[serde(default)]
    pub build_files: Vec<String>,

    /// Build artifact directories
    #[serde(default)]
    pub build_dirs: Vec<String>,

    /// Source map extensions
    #[serde(default)]
    pub map_extensions: Vec<String>,

    /// CI/CD config files
    #[serde(default)]
    pub ci_files: Vec<String>,

    /// CI/CD directories
    #[serde(default)]
    pub ci_dirs: Vec<String>,

    /// Example directories
    #[serde(default)]
    pub example_dirs: Vec<String>,

    /// TypeScript source extensions
    #[serde(default)]
    pub ts_source_extensions: Vec<String>,

    /// Additional patterns to exclude (never delete)
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Additional patterns to include (always delete)
    #[serde(default)]
    pub include_patterns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            override_defaults: false,
            doc_files: vec![],
            doc_dirs: vec![],
            test_dirs: vec![],
            test_patterns: vec![],
            build_extensions: vec![],
            build_files: vec![],
            build_dirs: vec![],
            map_extensions: vec![],
            ci_files: vec![],
            ci_dirs: vec![],
            example_dirs: vec![],
            ts_source_extensions: vec![],
            exclude_patterns: vec![],
            include_patterns: vec![],
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Try to load config from multiple locations in order:
    /// 1. --config <path> (if provided)
    /// 2. ./jatin-lean.toml
    /// 3. ./.jatin-lean.toml
    /// 4. ~/.config/jatin-lean/rules.toml
    pub fn load(custom_path: Option<&Path>, project_dir: &Path) -> Result<Option<Self>> {
        // 1. Custom path provided via CLI
        if let Some(path) = custom_path {
            if path.exists() {
                println!(
                    "  {} Loading config from: {}",
                    console::style("◉").cyan(),
                    console::style(path.display()).dim()
                );
                return Ok(Some(Self::from_file(path)?));
            } else {
                anyhow::bail!("Config file not found: {}", path.display());
            }
        }

        // 2. ./jatin-lean.toml
        let local_config = project_dir.join("jatin-lean.toml");
        if local_config.exists() {
            println!(
                "  {} Loading config from: {}",
                console::style("◉").cyan(),
                console::style("jatin-lean.toml").dim()
            );
            return Ok(Some(Self::from_file(&local_config)?));
        }

        // 3. ./.jatin-lean.toml
        let hidden_config = project_dir.join(".jatin-lean.toml");
        if hidden_config.exists() {
            println!(
                "  {} Loading config from: {}",
                console::style("◉").cyan(),
                console::style(".jatin-lean.toml").dim()
            );
            return Ok(Some(Self::from_file(&hidden_config)?));
        }

        // 4. ~/.config/jatin-lean/rules.toml
        if let Some(home) = dirs::home_dir() {
            let global_config = home.join(".config").join("jatin-lean").join("rules.toml");
            if global_config.exists() {
                println!(
                    "  {} Loading global config from: {}",
                    console::style("◉").cyan(),
                    console::style("~/.config/jatin-lean/rules.toml").dim()
                );
                return Ok(Some(Self::from_file(&global_config)?));
            }
        }

        // No config found, use defaults
        Ok(None)
    }

    /// Generate a sample config file
    pub fn generate_sample() -> String {
        r#"# jatin-lean configuration file
# Customize what gets deleted from node_modules

# If true, ignores all built-in rules and only uses the ones defined here.
# If false, these rules are added to the built-in defaults.
override_defaults = false

# Documentation files (exact filenames)
doc_files = [
    "README.md",
    "CHANGELOG.md",
    "CONTRIBUTING.md",
    "LICENSE",
]

# Documentation directories
doc_dirs = [
    "docs",
    "doc",
    ".github",
]

# Test directories
test_dirs = [
    "test",
    "tests",
    "__tests__",
    "spec",
    "specs",
]

# Test file patterns (regex)
test_patterns = [
    "\\.test\\.[jt]sx?$",
    "\\.spec\\.[jt]sx?$",
]

# Build artifact extensions
build_extensions = [
    ".c",
    ".cpp",
    ".o",
    ".gyp",
]

# Build artifact filenames
build_files = [
    "Makefile",
    "binding.gyp",
    "tsconfig.json",
    ".eslintrc",
]

# Build artifact directories
build_dirs = [
    "build",
]

# Source map extensions
map_extensions = [
    ".js.map",
    ".css.map",
]

# CI/CD config files
ci_files = [
    ".travis.yml",
    "circle.yml",
    "appveyor.yml",
]

# CI/CD directories
ci_dirs = [
    ".circleci",
    ".travis",
]

# Example directories
example_dirs = [
    "example",
    "examples",
    "demo",
    "demos",
]

# TypeScript source extensions (NOT .d.ts)
ts_source_extensions = [
    ".ts",
    ".tsx",
]

# Exclude patterns (never delete these)
exclude_patterns = [
    # "important-file.js",
    # "keep-this-dir/",
]

# Include patterns (always delete these)
include_patterns = [
    # "*.backup",
    # "temp/",
]
"#
        .to_string()
    }

    /// Create an example config file at the specified path
    pub fn create_example(path: &Path) -> Result<()> {
        let sample = Self::generate_sample();
        fs::write(path, sample)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(!config.override_defaults);
        assert!(config.doc_files.is_empty());
        assert!(config.test_dirs.is_empty());
    }

    #[test]
    fn test_config_generate_sample() {
        let sample = Config::generate_sample();
        assert!(sample.contains("override_defaults"));
        assert!(sample.contains("doc_files"));
        assert!(sample.contains("test_dirs"));
        assert!(sample.contains("build_extensions"));
    }

    #[test]
    fn test_config_create_example() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("test-config.toml");

        Config::create_example(&config_path)?;

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path)?;
        assert!(content.contains("jatin-lean configuration file"));

        Ok(())
    }

    #[test]
    fn test_config_from_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("test.toml");

        let toml_content = r#"
override_defaults = true
doc_files = ["CUSTOM_README.md"]
test_dirs = ["custom_tests"]
"#;
        fs::write(&config_path, toml_content)?;

        let config = Config::from_file(&config_path)?;
        assert!(config.override_defaults);
        assert_eq!(config.doc_files, vec!["CUSTOM_README.md"]);
        assert_eq!(config.test_dirs, vec!["custom_tests"]);

        Ok(())
    }

    #[test]
    fn test_config_load_custom_path() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("custom.toml");

        Config::create_example(&config_path)?;

        let loaded = Config::load(Some(&config_path), temp_dir.path())?;
        assert!(loaded.is_some());

        Ok(())
    }

    #[test]
    fn test_config_load_local() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("jatin-lean.toml");

        Config::create_example(&config_path)?;

        let loaded = Config::load(None, temp_dir.path())?;
        assert!(loaded.is_some());

        Ok(())
    }

    #[test]
    fn test_config_load_none() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let loaded = Config::load(None, temp_dir.path())?;
        assert!(loaded.is_none());

        Ok(())
    }

    #[test]
    fn test_config_invalid_path() {
        let result = Config::load(Some(Path::new("/nonexistent/config.toml")), Path::new("."));
        assert!(result.is_err());
    }
}
