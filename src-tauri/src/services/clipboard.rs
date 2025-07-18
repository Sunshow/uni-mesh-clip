use arboard::Clipboard;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration, timeout};
use anyhow::Result;

pub struct ClipboardMonitor {
    clipboard: Arc<Mutex<Clipboard>>,
    last_content: Arc<Mutex<Option<String>>>,
    sync_in_progress: Arc<Mutex<bool>>,
}

impl ClipboardMonitor {
    pub async fn new() -> Result<Self> {
        tracing::info!("Initializing clipboard monitor...");
        
        // Add timeout to prevent hanging on permission requests
        let clipboard_result = timeout(Duration::from_secs(5), async {
            tokio::task::spawn_blocking(|| Clipboard::new()).await
        }).await;
        
        let clipboard = match clipboard_result {
            Ok(join_result) => match join_result {
                Ok(clipboard_result) => match clipboard_result {
                    Ok(clipboard) => clipboard,
                    Err(e) => return Err(anyhow::anyhow!("Failed to initialize clipboard: {}", e)),
                },
                Err(e) => return Err(anyhow::anyhow!("Failed to spawn clipboard task: {}", e)),
            },
            Err(_) => return Err(anyhow::anyhow!("Clipboard initialization timed out - this usually means permission is required")),
        };
        
        tracing::info!("Clipboard monitor initialized successfully");
        Ok(Self {
            clipboard: Arc::new(Mutex::new(clipboard)),
            last_content: Arc::new(Mutex::new(None)),
            sync_in_progress: Arc::new(Mutex::new(false)),
        })
    }

    pub async fn start_monitoring<F>(&self, on_change: F) -> Result<()>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let clipboard = self.clipboard.clone();
        let last_content = self.last_content.clone();
        let sync_in_progress = self.sync_in_progress.clone();
        let on_change = Arc::new(on_change);
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(500));
            
            loop {
                interval.tick().await;
                
                // Skip monitoring if sync is in progress
                if *sync_in_progress.lock().await {
                    continue;
                }
                
                let mut clipboard = clipboard.lock().await;
                match clipboard.get_text() {
                    Ok(text) => {
                        let mut last = last_content.lock().await;
                        
                        if last.as_ref() != Some(&text) && !text.is_empty() {
                            *last = Some(text.clone());
                            drop(last);
                            drop(clipboard);
                            on_change(text);
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Failed to get clipboard text: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }

    pub async fn set_clipboard(&self, content: String) -> Result<()> {
        // Set sync in progress to prevent triggering our own change detection
        *self.sync_in_progress.lock().await = true;
        
        let result = {
            let mut clipboard = self.clipboard.lock().await;
            
            // Retry clipboard operation up to 3 times
            let mut retry_count = 0;
            loop {
                match clipboard.set_text(&content) {
                    Ok(_) => {
                        tracing::debug!("Clipboard set successfully on attempt {}", retry_count + 1);
                        break Ok(());
                    }
                    Err(e) => {
                        retry_count += 1;
                        if retry_count >= 3 {
                            tracing::error!("Failed to set clipboard after {} attempts: {}", retry_count, e);
                            break Err(anyhow::anyhow!("Failed to set clipboard: {}", e));
                        } else {
                            tracing::warn!("Clipboard set attempt {} failed, retrying: {}", retry_count, e);
                            drop(clipboard);
                            tokio::time::sleep(Duration::from_millis(50 * retry_count as u64)).await;
                            clipboard = self.clipboard.lock().await;
                        }
                    }
                }
            }
        };
        
        // Update our last_content to prevent detection on success
        if result.is_ok() {
            *self.last_content.lock().await = Some(content);
        }
        
        // Brief delay to ensure clipboard is set before re-enabling monitoring
        tokio::time::sleep(Duration::from_millis(100)).await;
        *self.sync_in_progress.lock().await = false;
        
        result
    }
}