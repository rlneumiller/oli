use anyhow::{anyhow, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::tools::lsp::protocol::{
    get_initialize_params, NotificationMessage, RequestId, RequestMessage, ResponseMessage,
};

pub struct LspServer {
    process: Child,
    #[allow(dead_code)]
    server_type: String,
    root_path: PathBuf,
    initialized: bool,
    next_request_id: u64,
}

impl LspServer {
    pub fn start_python_server(root_path: &Path) -> Result<Self> {
        eprintln!(
            "Starting Python LSP server (pyright) for path: {}",
            root_path.display()
        );

        // Check if pyright is installed
        let pyright_check = Command::new("sh")
            .arg("-c")
            .arg("command -v pyright")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if pyright_check.is_err() || !pyright_check.unwrap().success() {
            return Err(anyhow!("Python LSP server (pyright) not found. Please install it with 'npm install -g pyright'"));
        }

        let process = Command::new("pyright-langserver")
            .arg("--stdio")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let server = LspServer {
            process,
            server_type: "python".to_string(),
            root_path: root_path.to_path_buf(),
            initialized: false,
            next_request_id: 1,
        };

        Ok(server)
    }

    pub fn start_rust_server(root_path: &Path) -> Result<Self> {
        eprintln!(
            "Starting Rust LSP server (rust-analyzer) for path: {}",
            root_path.display()
        );

        // Check if rust-analyzer is installed
        let rust_analyzer_check = Command::new("sh")
            .arg("-c")
            .arg("command -v rust-analyzer")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if rust_analyzer_check.is_err() || !rust_analyzer_check.unwrap().success() {
            return Err(anyhow!(
                "Rust LSP server (rust-analyzer) not found. Please install it."
            ));
        }

        let process = Command::new("rust-analyzer")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let server = LspServer {
            process,
            server_type: "rust".to_string(),
            root_path: root_path.to_path_buf(),
            initialized: false,
            next_request_id: 1,
        };

        Ok(server)
    }

    pub fn initialize(&mut self) -> Result<ResponseMessage> {
        let params = get_initialize_params(self.root_path.to_str().unwrap_or("."));

        let response = self
            .send_request("initialize", Some(serde_json::to_value(params)?))?
            .ok_or_else(|| anyhow!("Failed to initialize LSP server: no response"))?;

        // Send initialized notification
        self.send_notification("initialized", Some(serde_json::json!({})))?;

        self.initialized = true;
        Ok(response)
    }

    pub fn shutdown(&mut self) -> Result<()> {
        if self.initialized {
            // Send shutdown request
            self.send_request("shutdown", None)?;

            // Send exit notification
            self.send_notification("exit", None)?;
        }

        // Terminate the process
        self.process.kill()?;
        Ok(())
    }

    fn send_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Option<ResponseMessage>> {
        if !self.initialized && method != "initialize" {
            return Err(anyhow!("LSP server not initialized"));
        }

        let id = self.next_request_id;
        self.next_request_id += 1;

        let request = RequestMessage {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(id),
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;
        let content_length = request_json.len();

        let message = format!("Content-Length: {}\r\n\r\n{}", content_length, request_json);

        if let Some(stdin) = self.process.stdin.as_mut() {
            stdin.write_all(message.as_bytes())?;
            stdin.flush()?;
        } else {
            return Err(anyhow!("Failed to get stdin handle"));
        }

        // Read response
        if let Some(stdout) = self.process.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut header = String::new();
            let mut content_length = 0;

            // Read headers
            loop {
                header.clear();
                reader.read_line(&mut header)?;
                if header.trim().is_empty() {
                    break;
                }

                if let Some(stripped) = header.strip_prefix("Content-Length: ") {
                    content_length = stripped.trim().parse::<usize>()?;
                }
            }

            // Read content
            let mut content = vec![0; content_length];
            reader.read_exact(&mut content)?;

            let response: ResponseMessage = serde_json::from_slice(&content)?;
            return Ok(Some(response));
        }

        Ok(None)
    }

    fn send_notification(&mut self, method: &str, params: Option<Value>) -> Result<()> {
        if !self.initialized && method != "initialized" {
            return Err(anyhow!("LSP server not initialized"));
        }

        let notification = NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        let notification_json = serde_json::to_string(&notification)?;
        let content_length = notification_json.len();

        let message = format!(
            "Content-Length: {}\r\n\r\n{}",
            content_length, notification_json
        );

        if let Some(stdin) = self.process.stdin.as_mut() {
            stdin.write_all(message.as_bytes())?;
            stdin.flush()?;
            Ok(())
        } else {
            Err(anyhow!("Failed to get stdin handle"))
        }
    }

    pub fn did_open_text_document(
        &mut self,
        uri: &str,
        language_id: &str,
        version: u32,
        text: &str,
    ) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": version,
                "text": text
            }
        });

        self.send_notification("textDocument/didOpen", Some(params))
    }

    pub fn document_symbol(&mut self, uri: &str) -> Result<Value> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri }
        });

        let response = self
            .send_request("textDocument/documentSymbol", Some(params))?
            .ok_or_else(|| anyhow!("No response from LSP server"))?;

        match response.result {
            Some(result) => Ok(result),
            None => Err(anyhow!("No result in LSP response: {:?}", response.error)),
        }
    }

    pub fn semantic_tokens(&mut self, uri: &str) -> Result<Value> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri }
        });

        let response = self
            .send_request("textDocument/semanticTokens/full", Some(params))?
            .ok_or_else(|| anyhow!("No response from LSP server"))?;

        match response.result {
            Some(result) => Ok(result),
            None => Err(anyhow!("No result in LSP response: {:?}", response.error)),
        }
    }

    pub fn code_lens(&mut self, uri: &str) -> Result<Value> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri }
        });

        let response = self
            .send_request("textDocument/codeLens", Some(params))?
            .ok_or_else(|| anyhow!("No response from LSP server"))?;

        match response.result {
            Some(result) => Ok(result),
            None => Err(anyhow!("No result in LSP response: {:?}", response.error)),
        }
    }

    pub fn definition(&mut self, uri: &str, line: u32, character: u32) -> Result<Value> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        let response = self
            .send_request("textDocument/definition", Some(params))?
            .ok_or_else(|| anyhow!("No response from LSP server"))?;

        match response.result {
            Some(result) => Ok(result),
            None => Err(anyhow!("No result in LSP response: {:?}", response.error)),
        }
    }

    #[allow(dead_code)]
    pub fn get_server_type(&self) -> &str {
        &self.server_type
    }
}

impl Drop for LspServer {
    fn drop(&mut self) {
        if self.initialized {
            let _ = self.shutdown();
        } else {
            let _ = self.process.kill();
        }
    }
}
