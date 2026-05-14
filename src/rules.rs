//! Heuristic ruleset for identifying non-essential files in node_modules.
//!
//! Defines patterns for strict junk, development bloat, build leftovers,
//! and source map files that can be safely removed.

use regex::RegexSet;
use std::path::Path;

/// Categories of files that can be pruned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    /// Documentation files (README, CHANGELOG, LICENSE, etc.)
    Documentation,
    /// Test assets (test/, __tests__/, *.test.js, etc.)
    TestAsset,
    /// Build artifacts (*.c, *.cpp, *.o, Makefile, binding.gyp, etc.)
    BuildArtifact,
    /// Source maps (*.js.map, *.css.map)
    SourceMap,
    /// CI/CD configuration files (.travis.yml, circle.yml, etc.)
    CiConfig,
    /// TypeScript source files (*.ts, *.tsx — only declarations are needed at runtime)
    TypeScriptSource,
    /// Example / demo files
    Example,
}

impl FileCategory {
    /// Returns a human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            FileCategory::Documentation => "Documentation",
            FileCategory::TestAsset => "Test-Asset",
            FileCategory::BuildArtifact => "Build-Artifact",
            FileCategory::CiConfig => "CI-Config",
            FileCategory::SourceMap => "Source-Map",
            FileCategory::TypeScriptSource => "TS-Source",
            FileCategory::Example => "Example",
        }
    }

    /// Returns a risk level (0 = no risk, 1 = low, 2 = medium).
    pub fn risk_level(&self) -> u8 {
        match self {
            FileCategory::Documentation => 0,
            FileCategory::CiConfig => 0,
            FileCategory::TestAsset => 0,
            FileCategory::SourceMap => 1,
            FileCategory::BuildArtifact => 1,
            FileCategory::TypeScriptSource => 2,
            FileCategory::Example => 0,
        }
    }
}

/// File patterns organized by category.
pub struct PruneRules {
    /// Documentation file patterns (checked by filename)
    pub doc_files: Vec<String>,
    /// Documentation directories
    pub doc_dirs: Vec<String>,
    /// Test directories
    pub test_dirs: Vec<String>,
    /// Test file extensions/patterns (regex)
    pub test_file_regex: RegexSet,
    /// Build artifact extensions
    pub build_extensions: Vec<String>,
    /// Build artifact filenames
    pub build_files: Vec<String>,
    /// Build artifact directories
    pub build_dirs: Vec<String>,
    /// Source map extensions
    pub map_extensions: Vec<String>,
    /// CI/CD config files
    pub ci_files: Vec<String>,
    /// CI/CD directories
    pub ci_dirs: Vec<String>,
    /// Example directories
    pub example_dirs: Vec<String>,
    /// TypeScript source extensions (NOT .d.ts)
    pub ts_source_extensions: Vec<String>,
}

impl Default for PruneRules {
    fn default() -> Self {
        Self::new()
    }
}

impl PruneRules {
    pub fn new() -> Self {
        Self::new_with_config(None)
    }
    
    pub fn new_with_config(config: Option<crate::config::Config>) -> Self {
        let mut rules = Self {
            // ── Documentation ──────────────────────────────
            doc_files: vec![
                "README.md",
                "README",
                "README.txt",
                "README.markdown",
                "readme.md",
                "readme.markdown",
                "CHANGELOG.md",
                "CHANGELOG",
                "CHANGELOG.txt",
                "changelog.md",
                "CHANGES.md",
                "CHANGES",
                "HISTORY.md",
                "HISTORY",
                "AUTHORS",
                "AUTHORS.md",
                "CONTRIBUTORS",
                "CONTRIBUTORS.md",
                "CONTRIBUTING.md",
                "CODE_OF_CONDUCT.md",
                "SECURITY.md",
                "TODO.md",
                "TODO",
                "NOTICE",
                "NOTICE.md",
            ].into_iter().map(String::from).collect(),
            doc_dirs: vec!["docs", "doc", ".github"].into_iter().map(String::from).collect(),

            // ── Test Assets ───────────────────────────────
            test_dirs: vec![
                "test",
                "tests",
                "spec",
                "specs",
                "__tests__",
                "__test__",
                "__mocks__",
                "__snapshots__",
                "fixtures",
                "test-fixtures",
                "coverage",
                ".nyc_output",
            ].into_iter().map(String::from).collect(),
            test_file_regex: RegexSet::new([
                r"\.test\.[jt]sx?$",
                r"\.spec\.[jt]sx?$",
                r"\.test\.mjs$",
                r"\.spec\.mjs$",
                r"jest\.config\.[jt]s$",
                r"jest\.config\.mjs$",
                r"jest\.setup\.[jt]s$",
                r"karma\.conf\.[jt]s$",
                r"mocha\..+$",
                r"\.mocharc\..+$",
                r"\.nycrc",
                r"nyc\.config\.[jt]s$",
                r"\.coveralls\.yml$",
            ])
            .expect("Invalid regex in test patterns"),

            // ── Build Artifacts ───────────────────────────
            build_extensions: vec![
                ".c", ".cpp", ".cc", ".cxx", ".h", ".hpp", ".hh", ".o", ".obj", ".a", ".lib",
                ".gyp", ".gypi",
            ].into_iter().map(String::from).collect(),
            build_files: vec![
                "Makefile",
                "makefile",
                "GNUmakefile",
                "CMakeLists.txt",
                "binding.gyp",
                "Gruntfile.js",
                "Gulpfile.js",
                "gulpfile.js",
                "webpack.config.js",
                "webpack.config.ts",
                "rollup.config.js",
                "rollup.config.mjs",
                "tsconfig.json",
                "tslint.json",
                ".eslintrc",
                ".eslintrc.js",
                ".eslintrc.json",
                ".eslintrc.yml",
                ".eslintignore",
                ".prettierrc",
                ".prettierrc.js",
                ".prettierrc.json",
                ".prettierignore",
                ".babelrc",
                ".babelrc.js",
                "babel.config.js",
                "babel.config.json",
                ".editorconfig",
                ".jshintrc",
                ".npmignore",
            ].into_iter().map(String::from).collect(),
            build_dirs: vec!["build", "obj"].into_iter().map(String::from).collect(),

            // ── Source Maps ───────────────────────────────
            map_extensions: vec![".js.map", ".css.map", ".mjs.map"].into_iter().map(String::from).collect(),

            // ── CI/CD Config ──────────────────────────────
            ci_files: vec![
                ".travis.yml",
                "circle.yml",
                "appveyor.yml",
                ".appveyor.yml",
                "Jenkinsfile",
                ".gitlab-ci.yml",
                "azure-pipelines.yml",
                "codecov.yml",
                ".codecov.yml",
            ].into_iter().map(String::from).collect(),
            ci_dirs: vec![".circleci", ".travis"].into_iter().map(String::from).collect(),

            // ── Examples ──────────────────────────────────
            example_dirs: vec!["example", "examples", "demo", "demos", "sample", "samples"].into_iter().map(String::from).collect(),

            // ── TypeScript Sources ────────────────────────
            ts_source_extensions: vec![".ts", ".tsx"].into_iter().map(String::from).collect(),
        };
        
        // Apply custom config if provided
        if let Some(cfg) = config {
            if cfg.override_defaults {
                // Replace defaults with config
                if !cfg.doc_files.is_empty() {
                    rules.doc_files = cfg.doc_files.clone();
                }
                if !cfg.doc_dirs.is_empty() {
                    rules.doc_dirs = cfg.doc_dirs.clone();
                }
                if !cfg.test_dirs.is_empty() {
                    rules.test_dirs = cfg.test_dirs.clone();
                }
                if !cfg.build_extensions.is_empty() {
                    rules.build_extensions = cfg.build_extensions.clone();
                }
                if !cfg.build_files.is_empty() {
                    rules.build_files = cfg.build_files.clone();
                }
                if !cfg.build_dirs.is_empty() {
                    rules.build_dirs = cfg.build_dirs.clone();
                }
                if !cfg.map_extensions.is_empty() {
                    rules.map_extensions = cfg.map_extensions.clone();
                }
                if !cfg.ci_files.is_empty() {
                    rules.ci_files = cfg.ci_files.clone();
                }
                if !cfg.ci_dirs.is_empty() {
                    rules.ci_dirs = cfg.ci_dirs.clone();
                }
                if !cfg.example_dirs.is_empty() {
                    rules.example_dirs = cfg.example_dirs.clone();
                }
                if !cfg.ts_source_extensions.is_empty() {
                    rules.ts_source_extensions = cfg.ts_source_extensions.clone();
                }
            } else {
                // Extend defaults with config
                for item in &cfg.doc_files {
                    if !rules.doc_files.contains(item) {
                        rules.doc_files.push(item.clone());
                    }
                }
                for item in &cfg.doc_dirs {
                    if !rules.doc_dirs.contains(item) {
                        rules.doc_dirs.push(item.clone());
                    }
                }
                for item in &cfg.test_dirs {
                    if !rules.test_dirs.contains(item) {
                        rules.test_dirs.push(item.clone());
                    }
                }
                for item in &cfg.build_extensions {
                    if !rules.build_extensions.contains(item) {
                        rules.build_extensions.push(item.clone());
                    }
                }
                for item in &cfg.build_files {
                    if !rules.build_files.contains(item) {
                        rules.build_files.push(item.clone());
                    }
                }
                for item in &cfg.build_dirs {
                    if !rules.build_dirs.contains(item) {
                        rules.build_dirs.push(item.clone());
                    }
                }
                for item in &cfg.map_extensions {
                    if !rules.map_extensions.contains(item) {
                        rules.map_extensions.push(item.clone());
                    }
                }
                for item in &cfg.ci_files {
                    if !rules.ci_files.contains(item) {
                        rules.ci_files.push(item.clone());
                    }
                }
                for item in &cfg.ci_dirs {
                    if !rules.ci_dirs.contains(item) {
                        rules.ci_dirs.push(item.clone());
                    }
                }
                for item in &cfg.example_dirs {
                    if !rules.example_dirs.contains(item) {
                        rules.example_dirs.push(item.clone());
                    }
                }
                for item in &cfg.ts_source_extensions {
                    if !rules.ts_source_extensions.contains(item) {
                        rules.ts_source_extensions.push(item.clone());
                    }
                }
            }
        }
        
        rules
    }

    /// Classify a file path into a category, or None if it should be kept.
    ///
    /// The `rel_path` should be relative to the package directory within node_modules.
    pub fn classify(&self, rel_path: &Path) -> Option<FileCategory> {
        let file_name = rel_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // ── Safety: never touch .bin or dotfiles (except .github) ──
        for component in rel_path.components() {
            if let Some(s) = component.as_os_str().to_str() {
                if s == ".bin" || s == "node_modules" {
                    return None;
                }
                // Allow .github to be deleted (it's in ci_dirs/doc_dirs)
                if s.starts_with('.') && s != ".github" && s != ".circleci" && s != ".travis" {
                    return None;
                }
            }
        }

        // ── Check directories in path ──
        for component in rel_path.components() {
            let dir_name = component.as_os_str().to_str().unwrap_or("");

            if self.test_dirs.iter().any(|d| d == dir_name) {
                return Some(FileCategory::TestAsset);
            }
            if self.doc_dirs.iter().any(|d| d == dir_name) {
                if dir_name == ".github" {
                    return Some(FileCategory::CiConfig);
                }
                return Some(FileCategory::Documentation);
            }
            if self.ci_dirs.iter().any(|d| d == dir_name) {
                return Some(FileCategory::CiConfig);
            }
            if self.example_dirs.iter().any(|d| d == dir_name) {
                return Some(FileCategory::Example);
            }
            // build dirs — but only if not the package root build
            if self.build_dirs.iter().any(|d| d == dir_name) {
                return Some(FileCategory::BuildArtifact);
            }
        }

        // ── Check filenames (documentation) ──
        if self.doc_files.iter().any(|f| f == file_name) {
            return Some(FileCategory::Documentation);
        }

        // ── Check CI config files ──
        if self.ci_files.iter().any(|f| f == file_name) {
            return Some(FileCategory::CiConfig);
        }

        // ── Check build artifact filenames ──
        if self.build_files.iter().any(|f| f == file_name) {
            return Some(FileCategory::BuildArtifact);
        }

        // ── Check extensions ──
        let path_str = rel_path.to_str().unwrap_or("");

        // Source maps (check before general extension checks since .js.map contains .map)
        for ext in &self.map_extensions {
            if path_str.ends_with(ext) {
                return Some(FileCategory::SourceMap);
            }
        }

        // Build artifact extensions
        for ext in &self.build_extensions {
            if file_name.ends_with(ext) {
                return Some(FileCategory::BuildArtifact);
            }
        }

        // Test file patterns (regex)
        if self.test_file_regex.is_match(file_name) {
            return Some(FileCategory::TestAsset);
        }

        // TypeScript sources (but NOT .d.ts declaration files)
        if !file_name.ends_with(".d.ts") && !file_name.ends_with(".d.tsx") {
            for ext in &self.ts_source_extensions {
                if file_name.ends_with(ext) {
                    return Some(FileCategory::TypeScriptSource);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_readme_classified_as_documentation() {
        let rules = PruneRules::new();
        let path = PathBuf::from("README.md");
        assert_eq!(rules.classify(&path), Some(FileCategory::Documentation));
    }

    #[test]
    fn test_test_dir_classified() {
        let rules = PruneRules::new();
        let path = PathBuf::from("__tests__/foo.js");
        assert_eq!(rules.classify(&path), Some(FileCategory::TestAsset));
    }

    #[test]
    fn test_source_map_classified() {
        let rules = PruneRules::new();
        let path = PathBuf::from("dist/bundle.js.map");
        assert_eq!(rules.classify(&path), Some(FileCategory::SourceMap));
    }

    #[test]
    fn test_dotbin_never_deleted() {
        let rules = PruneRules::new();
        let path = PathBuf::from(".bin/somefile");
        assert_eq!(rules.classify(&path), None);
    }

    #[test]
    fn test_dts_files_kept() {
        let rules = PruneRules::new();
        let path = PathBuf::from("index.d.ts");
        assert_eq!(rules.classify(&path), None);
    }

    #[test]
    fn test_ts_source_classified() {
        let rules = PruneRules::new();
        let path = PathBuf::from("src/utils.ts");
        assert_eq!(rules.classify(&path), Some(FileCategory::TypeScriptSource));
    }
}
