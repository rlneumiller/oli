use crate::inference;
use crate::models::{get_available_models, ModelConfig};
use anyhow::{Context, Result};
use dirs::home_dir;
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

#[derive(Debug, PartialEq)]
pub enum AppState {
    Setup,
    Error(String),
    Chat,
}

pub struct App {
    pub state: AppState,
    pub input: String,
    pub messages: Vec<String>,
    pub download_progress: Option<(u64, u64)>,
    pub selected_model: usize,
    pub available_models: Vec<ModelConfig>,
    pub inference: Option<inference::ModelSession>,
    pub download_active: bool,
    pub error_message: Option<String>,
    pub debug_messages: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::Setup,
            input: String::new(),
            messages: vec![],
            download_progress: None,
            selected_model: 0,
            available_models: get_available_models(),
            inference: None,
            download_active: false,
            error_message: None,
            debug_messages: true,
        }
    }

    // Get current selected model config
    pub fn current_model(&self) -> &ModelConfig {
        &self.available_models[self.selected_model]
    }

    pub fn models_dir() -> Result<PathBuf> {
        Ok(home_dir()
            .context("Failed to find home directory")?
            .join(".oli")
            .join("models"))
    }

    pub fn setup_models(&mut self, tx: mpsc::Sender<String>) -> Result<()> {
        if self.debug_messages {
            self.messages.push("DEBUG: setup_models called".into());
        }

        self.error_message = None;
        // Initialize download state
        self.download_active = true;

        let model_name = self.current_model().name.clone();
        let model_file_name = self.current_model().file_name.clone();
        let model_primary_url = self.current_model().primary_url.clone();
        let model_fallback_url = self.current_model().fallback_url.clone();

        self.messages
            .push(format!("Setting up model: {}", model_name));

        let models_dir = Self::models_dir()?;
        std::fs::create_dir_all(&models_dir)?;

        let model_path = models_dir.join(&model_file_name);
        if model_path.exists() {
            if self.debug_messages {
                self.messages
                    .push(format!("DEBUG: Model file exists at {:?}", model_path));
            }

            match self.verify_model(&model_path) {
                Ok(()) => match self.load_model(&model_path) {
                    Ok(()) => {
                        if self.debug_messages {
                            self.messages
                                .push("DEBUG: Model loaded successfully".into());
                        }
                        tx.send("setup_complete".into())?;
                    }
                    Err(e) => {
                        self.handle_error(format!("Failed to load model: {}", e));
                        tx.send("setup_failed".into())?;
                    }
                },
                Err(e) => {
                    self.handle_error(format!("Invalid model file: {}", e));
                    std::fs::remove_file(&model_path).ok();
                    self.download_active = true;
                    self.messages
                        .push("Starting download after validation failure...".into());
                    self.download_model_with_path(
                        tx.clone(),
                        &model_path,
                        &model_primary_url,
                        &model_fallback_url,
                    )?;
                }
            }
            return Ok(());
        }

        if self.debug_messages {
            self.messages
                .push(format!("DEBUG: Model file does not exist, downloading..."));
        }

        self.download_active = true;
        self.messages
            .push(format!("Starting download of {}...", model_name));
        self.download_model_with_path(tx, &model_path, &model_primary_url, &model_fallback_url)
    }

    fn download_model_with_path(
        &mut self,
        tx: mpsc::Sender<String>,
        path: &Path,
        primary_url: &str,
        fallback_url: &str,
    ) -> Result<()> {
        if self.debug_messages {
            self.messages
                .push(format!("DEBUG: Downloading to {:?}", path));
            self.messages
                .push(format!("DEBUG: download_active={}", self.download_active));
        }
        self.download_file(primary_url, fallback_url, path, tx)
    }

    // Add methods to change selected model
    pub fn select_next_model(&mut self) {
        self.selected_model = (self.selected_model + 1) % self.available_models.len();
    }

    pub fn select_prev_model(&mut self) {
        self.selected_model = if self.selected_model == 0 {
            self.available_models.len() - 1
        } else {
            self.selected_model - 1
        };
    }

    fn handle_error(&mut self, message: String) {
        self.error_message = Some(message.clone());
        self.messages.push(format!("Error: {}", message));
    }

    fn verify_model(&self, path: &Path) -> Result<()> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];
        if file.read_exact(&mut magic).is_err() {
            anyhow::bail!("Failed to read magic bytes - file may be corrupted");
        }

        if &magic != b"GGUF" {
            let content = std::fs::read(path)?;
            if content.len() < 50 {
                anyhow::bail!(
                    "File too small to be a valid model ({}bytes)",
                    content.len()
                );
            }

            if content.starts_with(b"<html") || content.starts_with(b"<!DOCTYPE") {
                anyhow::bail!("Received HTML page instead of model file");
            }

            anyhow::bail!(
                "Invalid GGUF format (magic: {:?} or '{}')",
                magic,
                String::from_utf8_lossy(&magic)
            );
        }
        Ok(())
    }

    fn download_file(
        &mut self,
        primary_url: &str,
        fallback_url: &str,
        path: &Path,
        tx: mpsc::Sender<String>,
    ) -> Result<()> {
        let primary_url = primary_url.to_string();
        let fallback_url = fallback_url.to_string();
        let path = path.to_path_buf();
        let tx_clone = tx.clone();
        
        // Ensure download_active is set to true
        self.download_active = true;

        std::thread::spawn(move || {
            let download_result = || -> Result<(), String> {
                match Self::attempt_download(&primary_url, &path, &tx_clone) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        tx_clone
                            .send(format!("retry:First download attempt failed: {}", e))
                            .ok();

                        match Self::attempt_download(&fallback_url, &path, &tx_clone) {
                            Ok(()) => return Ok(()),
                            Err(e2) => {
                                return Err(format!(
                                    "Both download attempts failed. Primary: {}, Fallback: {}",
                                    e, e2
                                ));
                            }
                        }
                    }
                }
            }();

            match download_result {
                Ok(()) => {
                    // Send a success message
                    tx_clone.send("status:Download successful, verifying file...".into()).unwrap();
                    
                    // Verify after download completes
                    match Self::verify_static(&path) {
                        Ok(()) => {
                            tx_clone.send("status:File verified successfully".into()).unwrap();
                            tx_clone.send("download_complete".into()).unwrap()
                        },
                        Err(e) => tx_clone.send(format!("error:{}", e)).unwrap(),
                    }
                }
                Err(e) => tx_clone.send(format!("error:{}", e)).unwrap(),
            }
        });

        Ok(())
    }

    fn attempt_download(url: &str, path: &Path, tx: &mpsc::Sender<String>) -> Result<(), String> {
        // Send an initial message to indicate download is starting
        tx.send(format!("download_started:{}", url))
            .map_err(|e| format!("Channel error: {}", e))?;

        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .timeout(Duration::from_secs(300)) // Longer timeout for large files (5 min)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("Client build failed: {}", e))?;

        // Notify about connection attempt
        tx.send(format!("status:Connecting to {}...", url))
            .map_err(|e| format!("Channel error: {}", e))?;

        let mut response = client
            .get(url)
            .header(reqwest::header::ACCEPT, "*/*")
            .send()
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {} for URL {}", response.status(), url));
        }

        // Get the content length to track progress
        let total_size = response.content_length().unwrap_or(0);
        tx.send(format!("status:Downloading {}MB file...", total_size / 1_000_000))
            .map_err(|e| format!("Channel error: {}", e))?;
            
        let mut file = File::create(path).map_err(|e| format!("File creation failed: {}", e))?;

        // Initial progress
        tx.send(format!("progress:0:{}", total_size))
            .map_err(|e| format!("Channel error: {}", e))?;

        // Create a buffer for reading chunks
        let mut buffer = [0; 8192]; // 8KB buffer
        let mut downloaded: u64 = 0;
        let mut last_progress_time = std::time::Instant::now();
        
        // Read and write in chunks to show progress
        loop {
            match response.read(&mut buffer) {
                Ok(0) => break, // End of file
                Ok(n) => {
                    file.write_all(&buffer[..n])
                        .map_err(|e| format!("Write error: {}", e))?;
                    
                    downloaded += n as u64;
                    
                    // Update progress at most every 500ms to avoid flooding
                    let now = std::time::Instant::now();
                    if now.duration_since(last_progress_time).as_millis() > 500 {
                        tx.send(format!("progress:{}:{}", downloaded, total_size))
                            .map_err(|e| format!("Channel error: {}", e))?;
                        last_progress_time = now;
                    }
                }
                Err(e) => return Err(format!("Download error: {}", e)),
            }
        }

        // Final progress update
        tx.send(format!("progress:{}:{}", downloaded, total_size))
            .map_err(|e| format!("Channel error: {}", e))?;

        // Ensure file is written to disk
        file.sync_all()
            .map_err(|e| format!("File sync error: {}", e))?;

        Ok(())
    }

    fn verify_static(path: &Path) -> Result<()> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];

        if let Err(e) = file.read_exact(&mut magic) {
            anyhow::bail!("Failed to read file header: {}", e);
        }

        if &magic != b"GGUF" {
            let magic_str = String::from_utf8_lossy(&magic);
            anyhow::bail!("Invalid model file (magic: {:?} or '{}')", magic, magic_str);
        }
        Ok(())
    }

    pub fn load_model(&mut self, model_path: &Path) -> Result<()> {
        let n_gpu_layers = self.current_model().n_gpu_layers;
        let inference = inference::ModelSession::new(model_path, n_gpu_layers)?;
        self.inference = Some(inference);
        self.messages.push("Model loaded successfully!".into());
        self.download_active = false;
        self.state = AppState::Chat;
        Ok(())
    }

    pub fn query_model(&mut self, prompt: &str) -> Result<String> {
        self.inference
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Model not loaded"))
            .and_then(|inference| inference.generate(prompt))
    }
}
