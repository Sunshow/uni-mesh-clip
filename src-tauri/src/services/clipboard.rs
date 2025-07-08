use arboard::Clipboard;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use anyhow::Result;

pub struct ClipboardMonitor {
    clipboard: Arc<Mutex<Clipboard>>,
    last_content: Arc<Mutex<Option<String>>>,
}

impl ClipboardMonitor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            clipboard: Arc::new(Mutex::new(Clipboard::new()?)),
            last_content: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn start_monitoring<F>(&self, on_change: F) -> Result<()>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let clipboard = self.clipboard.clone();
        let last_content = self.last_content.clone();
        
        let mut interval = interval(Duration::from_millis(500));
        
        loop {
            interval.tick().await;
            
            let mut clipboard = clipboard.lock().await;
            if let Ok(text) = clipboard.get_text() {
                let mut last = last_content.lock().await;
                
                if last.as_ref() != Some(&text) {
                    *last = Some(text.clone());
                    on_change(text);
                }
            }
        }
    }

    pub async fn set_clipboard(&self, content: String) -> Result<()> {
        let mut clipboard = self.clipboard.lock().await;
        clipboard.set_text(content)?;
        Ok(())
    }
}