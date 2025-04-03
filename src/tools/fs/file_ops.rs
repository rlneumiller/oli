use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::diff::DiffTools;

pub struct FileOps;

impl FileOps {
    pub fn read_file(path: &Path) -> Result<String> {
        let mut file =
            File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        Ok(content)
    }

    pub fn read_file_with_line_numbers(path: &Path) -> Result<String> {
        let content = Self::read_file(path)?;
        let numbered_content = content
            .lines()
            .enumerate()
            .map(|(i, line)| format!("{:4} | {}", i + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(numbered_content)
    }

    pub fn read_file_lines(path: &Path, offset: usize, limit: Option<usize>) -> Result<String> {
        let content = Self::read_file(path)?;
        let lines: Vec<&str> = content.lines().collect();
        let start = offset.min(lines.len());
        let end = match limit {
            Some(limit) => (start + limit).min(lines.len()),
            None => lines.len(),
        };

        let numbered_content = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:4} | {}", i + start + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(numbered_content)
    }

    pub fn generate_write_diff(path: &Path, content: &str) -> Result<(String, bool)> {
        // Check if file exists to determine if this is an update or new file
        let is_new_file = !path.exists();

        let old_content = if is_new_file {
            String::new()
        } else {
            Self::read_file(path)?
        };

        // Generate a diff
        let diff_lines = DiffTools::generate_diff(&old_content, content);
        let formatted_diff = DiffTools::format_diff(&diff_lines, &path.display().to_string())?;

        Ok((formatted_diff, is_new_file))
    }

    pub fn write_file(path: &Path, content: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let mut file = File::create(path)
            .with_context(|| format!("Failed to create file: {}", path.display()))?;
        file.write_all(content.as_bytes())
            .with_context(|| format!("Failed to write to file: {}", path.display()))?;
        Ok(())
    }

    pub fn write_file_with_diff(path: &Path, content: &str) -> Result<String> {
        let (diff, _) = Self::generate_write_diff(path, content)?;
        Self::write_file(path, content)?;
        Ok(diff)
    }

    pub fn generate_edit_diff(
        path: &Path,
        old_string: &str,
        new_string: &str,
    ) -> Result<(String, String)> {
        let content = Self::read_file(path)?;

        // Count occurrences to ensure we're replacing a unique string
        let occurrences = content.matches(old_string).count();
        if occurrences == 0 {
            anyhow::bail!("The string to replace was not found in the file");
        }
        if occurrences > 1 {
            anyhow::bail!("The string to replace appears multiple times in the file ({}). Please provide more context to ensure a unique match.", occurrences);
        }

        let new_content = content.replace(old_string, new_string);

        // Generate a diff
        let diff_lines = DiffTools::generate_diff(&content, &new_content);
        let formatted_diff = DiffTools::format_diff(&diff_lines, &path.display().to_string())?;

        Ok((new_content, formatted_diff))
    }

    pub fn edit_file(path: &Path, old_string: &str, new_string: &str) -> Result<String> {
        let (new_content, diff) = Self::generate_edit_diff(path, old_string, new_string)?;
        Self::write_file(path, &new_content)?;
        Ok(diff)
    }

    pub fn list_directory(path: &Path) -> Result<Vec<PathBuf>> {
        let entries = fs::read_dir(path)
            .with_context(|| format!("Failed to read directory: {}", path.display()))?;

        let mut paths = Vec::new();
        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            paths.push(entry.path());
        }

        // Sort by name
        paths.sort();

        Ok(paths)
    }

    #[allow(dead_code)]
    pub fn create_directory(path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_file_info(path: &Path) -> Result<String> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        let file_type = if metadata.is_dir() {
            "Directory"
        } else if metadata.is_file() {
            "File"
        } else {
            "Unknown"
        };

        let size = metadata.len();
        let modified = metadata
            .modified()
            .context("Failed to get modification time")?;

        let info = format!(
            "Path: {}\nType: {}\nSize: {} bytes\nModified: {:?}",
            path.display(),
            file_type,
            size,
            modified
        );

        Ok(info)
    }
}
