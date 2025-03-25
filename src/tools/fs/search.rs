use anyhow::{Context, Result};
use glob::glob;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct SearchTools;

impl SearchTools {
    pub fn glob_search(pattern: &str) -> Result<Vec<PathBuf>> {
        let entries =
            glob(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;

        let mut matches = Vec::new();
        for entry in entries {
            let path = entry.context("Failed to read glob entry")?;
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
