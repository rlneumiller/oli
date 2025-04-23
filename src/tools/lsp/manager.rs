use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::servers::LspServer;
use crate::tools::lsp::models::{
    CodeLens, DocumentSymbol, Location, LspServerType, Position, SemanticTokens,
};

/// Manager for LSP servers
pub struct LspServerManager {
    servers: Mutex<HashMap<String, LspServer>>,
}

impl Default for LspServerManager {
    fn default() -> Self {
        Self {
            servers: Mutex::new(HashMap::new()),
        }
    }
}

impl LspServerManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create an LSP server for a specific language and workspace
    pub fn get_server(&self, server_type: &LspServerType, workspace_path: &Path) -> Result<String> {
        let mut servers = self
            .servers
            .lock()
            .map_err(|_| anyhow!("Failed to lock servers mutex"))?;

        // Create a unique key for this server combination
        let server_key = format!("{:?}-{}", server_type, workspace_path.display());

        if !servers.contains_key(&server_key) {
            // Start a new server
            let mut server = match server_type {
                LspServerType::Python => LspServer::start_python_server(workspace_path)?,
                LspServerType::Rust => LspServer::start_rust_server(workspace_path)?,
            };

            // Initialize the server
            server.initialize()?;
            servers.insert(server_key.clone(), server);
        }

        Ok(server_key)
    }

    /// Get document symbols for a file
    pub fn document_symbol(
        &self,
        file_path: &str,
        server_type: &LspServerType,
    ) -> Result<Vec<DocumentSymbol>> {
        let path = PathBuf::from(file_path);
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path));
        }

        let workspace_path = self.find_workspace_root(&path)?;
        let server_key = self.get_server(server_type, &workspace_path)?;

        let uri = format!("file://{}", file_path);
        let file_content = fs::read_to_string(file_path)?;
        let language_id = match server_type {
            LspServerType::Python => "python",
            LspServerType::Rust => "rust",
        };

        let mut servers = self
            .servers
            .lock()
            .map_err(|_| anyhow!("Failed to lock servers mutex"))?;
        let server = servers
            .get_mut(&server_key)
            .ok_or_else(|| anyhow!("Server not found: {}", server_key))?;

        // Notify the server about the file
        server.did_open_text_document(&uri, language_id, 1, &file_content)?;

        // Get document symbols
        let result = server.document_symbol(&uri)?;

        // Parse the result
        let symbols: Vec<DocumentSymbol> = serde_json::from_value(result)?;
        Ok(symbols)
    }

    /// Get semantic tokens for a file
    pub fn semantic_tokens(
        &self,
        file_path: &str,
        server_type: &LspServerType,
    ) -> Result<SemanticTokens> {
        let path = PathBuf::from(file_path);
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path));
        }

        let workspace_path = self.find_workspace_root(&path)?;
        let server_key = self.get_server(server_type, &workspace_path)?;

        let uri = format!("file://{}", file_path);
        let file_content = fs::read_to_string(file_path)?;
        let language_id = match server_type {
            LspServerType::Python => "python",
            LspServerType::Rust => "rust",
        };

        let mut servers = self
            .servers
            .lock()
            .map_err(|_| anyhow!("Failed to lock servers mutex"))?;
        let server = servers
            .get_mut(&server_key)
            .ok_or_else(|| anyhow!("Server not found: {}", server_key))?;

        // Notify the server about the file
        server.did_open_text_document(&uri, language_id, 1, &file_content)?;

        // Get semantic tokens
        let result = server.semantic_tokens(&uri)?;

        // Parse the result
        let tokens: SemanticTokens = serde_json::from_value(result)?;
        Ok(tokens)
    }

    /// Get code lenses for a file
    pub fn code_lens(&self, file_path: &str, server_type: &LspServerType) -> Result<Vec<CodeLens>> {
        let path = PathBuf::from(file_path);
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path));
        }

        let workspace_path = self.find_workspace_root(&path)?;
        let server_key = self.get_server(server_type, &workspace_path)?;

        let uri = format!("file://{}", file_path);
        let file_content = fs::read_to_string(file_path)?;
        let language_id = match server_type {
            LspServerType::Python => "python",
            LspServerType::Rust => "rust",
        };

        let mut servers = self
            .servers
            .lock()
            .map_err(|_| anyhow!("Failed to lock servers mutex"))?;
        let server = servers
            .get_mut(&server_key)
            .ok_or_else(|| anyhow!("Server not found: {}", server_key))?;

        // Notify the server about the file
        server.did_open_text_document(&uri, language_id, 1, &file_content)?;

        // Get code lenses
        let result = server.code_lens(&uri)?;

        // Parse the result
        let lenses: Vec<CodeLens> = serde_json::from_value(result)?;
        Ok(lenses)
    }

    /// Get definition for a symbol at a specific position
    pub fn definition(
        &self,
        file_path: &str,
        position: &Position,
        server_type: &LspServerType,
    ) -> Result<Vec<Location>> {
        let path = PathBuf::from(file_path);
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path));
        }

        let workspace_path = self.find_workspace_root(&path)?;
        let server_key = self.get_server(server_type, &workspace_path)?;

        let uri = format!("file://{}", file_path);
        let file_content = fs::read_to_string(file_path)?;
        let language_id = match server_type {
            LspServerType::Python => "python",
            LspServerType::Rust => "rust",
        };

        let mut servers = self
            .servers
            .lock()
            .map_err(|_| anyhow!("Failed to lock servers mutex"))?;
        let server = servers
            .get_mut(&server_key)
            .ok_or_else(|| anyhow!("Server not found: {}", server_key))?;

        // Notify the server about the file
        server.did_open_text_document(&uri, language_id, 1, &file_content)?;

        // Get definition
        let result = server.definition(&uri, position.line, position.character)?;

        // Parse the result
        let locations: Vec<Location> = serde_json::from_value(result)?;
        Ok(locations)
    }

    /// Find the root directory of a workspace
    fn find_workspace_root(&self, file_path: &Path) -> Result<PathBuf> {
        let mut current_dir = file_path
            .parent()
            .ok_or_else(|| anyhow!("Cannot determine parent directory"))?
            .to_path_buf();

        // Look for common project indicators
        loop {
            // Check for Rust project
            if current_dir.join("Cargo.toml").exists() {
                return Ok(current_dir);
            }

            // Check for Python project
            if current_dir.join("pyproject.toml").exists()
                || current_dir.join("setup.py").exists()
                || current_dir.join("requirements.txt").exists()
            {
                return Ok(current_dir);
            }

            // Check for git repository
            if current_dir.join(".git").exists() {
                return Ok(current_dir);
            }

            // Go up one directory
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                // If we reached the root, use the file's directory
                return Ok(file_path
                    .parent()
                    .ok_or_else(|| anyhow!("Cannot determine parent directory"))?
                    .to_path_buf());
            }
        }
    }

    /// Stop all servers
    pub fn stop_all(&self) -> Result<()> {
        let mut servers = self
            .servers
            .lock()
            .map_err(|_| anyhow!("Failed to lock servers mutex"))?;

        for (_, server) in servers.iter_mut() {
            if server.shutdown().is_err() {
                eprintln!("Error shutting down LSP server");
            }
        }

        servers.clear();
        Ok(())
    }
}

impl Drop for LspServerManager {
    fn drop(&mut self) {
        if self.stop_all().is_err() {
            eprintln!("Error stopping LSP servers during manager drop");
        }
    }
}
