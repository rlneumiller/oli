use anyhow::{Context, Result};
use glob::glob;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub struct SearchTools;

impl SearchTools {
    // Check if path should be ignored based on common patterns
    fn is_ignored_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Common directories to ignore
        let ignored_dirs = [
            "/target/",
            "/node_modules/",
            "/.git/",
            "/dist/",
            "/build/",
            "/.cache/",
            "/coverage/",
            "/.next/",
            "/.nuxt/",
            "/venv/",
            "/.venv/",
            "/env/",
            "/__pycache__/",
            "/out/",
            "/bin/",
            "/obj/",
        ];

        // Check if the path contains any of the ignored directories
        for dir in &ignored_dirs {
            if path_str.contains(dir) {
                return true;
            }
        }

        // Ignore common generated or binary files
        let ignored_extensions = [
            ".pyc",
            ".pyo",
            ".so",
            ".o",
            ".a",
            ".lib",
            ".dll",
            ".exe",
            ".jar",
            ".war",
            ".ear",
            ".class",
            ".min.js",
            ".min.css",
            ".map",
            ".bundle.js",
            ".swp",
            ".swo",
            ".db",
            ".sqlite",
            ".sqlite3",
            ".lock",
            ".log",
            ".tmp",
            ".temp",
            ".bak",
        ];

        // First check exact file extensions
        if let Some(extension) = path.extension() {
            let ext = format!(".{}", extension.to_string_lossy());
            if ignored_extensions.contains(&ext.as_str()) {
                return true;
            }
        }

        // Then check path suffixes for special cases like minified files
        let path_str = path.to_string_lossy();
        if path_str.ends_with(".min.js")
            || path_str.ends_with(".min.css")
            || path_str.ends_with(".bundle.js")
            || path_str.ends_with(".map")
        {
            return true;
        }

        false
    }

    pub fn glob_search(pattern: &str) -> Result<Vec<PathBuf>> {
        let entries =
            glob(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;

        let mut matches = Vec::new();
        for entry in entries {
            let path = entry.context("Failed to read glob entry")?;

            // Skip ignored paths
            if Self::is_ignored_path(&path) {
                continue;
            }

            matches.push(path);
        }

        // Sort by last modified time (most recent first)
        matches.sort_by(|a, b| {
            let a_modified = std::fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_modified = std::fs::metadata(b).and_then(|m| m.modified()).ok();
            b_modified.cmp(&a_modified)
        });

        Ok(matches)
    }

    pub fn glob_search_in_dir(dir: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
        let dir_str = dir.to_string_lossy();
        let full_pattern = format!("{}/{}", dir_str, pattern);
        Self::glob_search(&full_pattern)
    }

    // Helper function for WalkDir to skip ignored directories
    fn should_skip_dir(entry: &DirEntry) -> bool {
        let path = entry.path();

        if entry.file_type().is_dir() {
            let file_name = entry.file_name().to_string_lossy();

            // Skip common directories that should be ignored
            let ignored_dirs = [
                "target",
                "node_modules",
                ".git",
                "dist",
                "build",
                ".cache",
                "coverage",
                ".next",
                ".nuxt",
                "venv",
                ".venv",
                "env",
                "__pycache__",
                "out",
                "bin",
                "obj",
            ];

            if ignored_dirs.contains(&file_name.as_ref()) {
                return true;
            }
        }

        // Use the general path ignoring function for files
        if entry.file_type().is_file() && Self::is_ignored_path(path) {
            return true;
        }

        false
    }

    pub fn grep_search(
        pattern: &str,
        include_pattern: Option<&str>,
        search_dir: Option<&Path>,
    ) -> Result<Vec<(PathBuf, usize, String)>> {
        let regex =
            Regex::new(pattern).with_context(|| format!("Invalid regex pattern: {}", pattern))?;

        let dir = search_dir.unwrap_or_else(|| Path::new("."));

        let include_regex = match include_pattern {
            Some(pattern) => {
                let pattern = pattern.replace("*.{", "*.").replace("}", "|*."); // Convert *.{ts,tsx} to *.ts|*.tsx
                let parts: Vec<&str> = pattern.split('|').collect();
                let regex_parts: Vec<String> = parts
                    .iter()
                    .map(|p| format!("({})", glob_to_regex(p)))
                    .collect();
                let joined = regex_parts.join("|");
                Some(Regex::new(&joined).with_context(|| {
                    format!("Invalid include pattern: {}", include_pattern.unwrap())
                })?)
            }
            None => None,
        };

        let mut matches = Vec::new();

        for entry in WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| !Self::should_skip_dir(e))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();

            // Skip if doesn't match include pattern
            if let Some(ref include_regex) = include_regex {
                if !include_regex.is_match(&path.to_string_lossy()) {
                    continue;
                }
            }

            // Skip binary files and other files that are typically not text
            if let Some(extension) = path.extension() {
                let ext = extension.to_string_lossy();
                let binary_extensions = [
                    "pyc", "pyo", "so", "o", "a", "lib", "dll", "exe", "jar", "war", "ear",
                    "class", "db", "sqlite", "sqlite3",
                ];
                if binary_extensions.contains(&ext.as_ref()) {
                    continue;
                }
            }

            // Check for minified JS/CSS files and other special cases that require full path check
            let path_str = path.to_string_lossy();
            if path_str.ends_with(".min.js")
                || path_str.ends_with(".min.css")
                || path_str.ends_with(".bundle.js")
                || path_str.ends_with(".map")
            {
                continue;
            }

            // Try to open file
            if let Ok(file) = File::open(path) {
                let reader = BufReader::new(file);
                for (line_num, line_result) in reader.lines().enumerate() {
                    if let Ok(line) = line_result {
                        if regex.is_match(&line) {
                            matches.push((path.to_path_buf(), line_num + 1, line.clone()));
                        }
                    }
                }
            }
        }

        // Sort by last modified time (most recent first)
        matches.sort_by(|a, b| {
            let a_modified = std::fs::metadata(&a.0).and_then(|m| m.modified()).ok();
            let b_modified = std::fs::metadata(&b.0).and_then(|m| m.modified()).ok();
            b_modified.cmp(&a_modified)
        });

        Ok(matches)
    }
}

fn glob_to_regex(glob_pattern: &str) -> String {
    let mut regex_pattern = String::new();

    for c in glob_pattern.chars() {
        match c {
            '*' => regex_pattern.push_str(".*"),
            '?' => regex_pattern.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex_pattern.push('\\');
                regex_pattern.push(c);
            }
            _ => regex_pattern.push(c),
        }
    }

    format!("^{}$", regex_pattern)
}
