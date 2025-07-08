use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio::time::Duration;
use std::collections::HashMap;
use uuid::Uuid;
use anyhow::Result;
use std::net::SocketAddr;
use crate::models::{ClipboardMessage, MessageCache, SyncMetrics};

type Tx = broadcast::Sender<String>;
type PeerMap = Arc<RwLock<HashMap<Uuid, (SocketAddr, tokio::sync::mpsc::UnboundedSender<Message>)>>>;

pub struct WebSocketServer {
    port: u16,
    peers: PeerMap,
    tx: Tx,
    shutdown_tx: broadcast::Sender<()>,
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    message_cache: Arc<RwLock<MessageCache>>,
    clipboard_callback: Arc<RwLock<Option<Box<dyn Fn(String) + Send + Sync>>>>,
    sync_metrics: Arc<RwLock<SyncMetrics>>,
}

impl WebSocketServer {
    pub fn new(port: u16) -> Self {
        let (tx, _) = broadcast::channel(100);
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            port,
            peers: Arc::new(RwLock::new(HashMap::new())),
            tx,
            shutdown_tx,
            server_handle: Arc::new(RwLock::new(None)),
            message_cache: Arc::new(RwLock::new(MessageCache::new())),
            clipboard_callback: Arc::new(RwLock::new(None)),
            sync_metrics: Arc::new(RwLock::new(SyncMetrics::default())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        // Check if already running
        if self.server_handle.read().await.is_some() {
            tracing::warn!("WebSocket server is already running");
            return Ok(());
        }

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to bind WebSocket server to {}: {}", addr, e);
                return Err(e.into());
            }
        };
        tracing::info!("WebSocket server listening on {}", addr);

        let peers = self.peers.clone();
        let tx = self.tx.clone();
        let message_cache = self.message_cache.clone();
        let clipboard_callback = self.clipboard_callback.clone();
        let sync_metrics = self.sync_metrics.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                tokio::spawn(Self::handle_connection(
                                    stream, 
                                    addr, 
                                    peers.clone(), 
                                    tx.clone(),
                                    message_cache.clone(),
                                    clipboard_callback.clone(),
                                    sync_metrics.clone()
                                ));
                            }
                            Err(e) => {
                                tracing::error!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("WebSocket server shutting down");
                        break;
                    }
                }
            }
        });

        *self.server_handle.write().await = Some(handle);
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping WebSocket server on port {}", self.port);
        
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());
        
        // Wait for server task to finish
        let mut handle_guard = self.server_handle.write().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
            tracing::info!("WebSocket server stopped");
        }
        
        // Clear all peers
        self.peers.write().await.clear();
        
        Ok(())
    }

    pub async fn set_clipboard_callback<F>(&self, callback: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        *self.clipboard_callback.write().await = Some(Box::new(callback));
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        peers: PeerMap,
        tx: Tx,
        message_cache: Arc<RwLock<MessageCache>>,
        clipboard_callback: Arc<RwLock<Option<Box<dyn Fn(String) + Send + Sync>>>>,
        sync_metrics: Arc<RwLock<SyncMetrics>>,
    ) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let peer_id = Uuid::new_v4();
        tracing::info!("New WebSocket connection from {} with id {}", addr, peer_id);

        let (ws_sender, mut ws_receiver) = ws_stream.split();
        let (peer_tx, mut peer_rx) = tokio::sync::mpsc::unbounded_channel();

        // Add peer to the map
        peers.write().await.insert(peer_id, (addr, peer_tx));
        
        // Update connected peers count
        {
            let mut metrics = sync_metrics.write().await;
            metrics.connected_peers = peers.read().await.len() as u32;
        }

        // Spawn task to forward messages from channel to websocket
        let mut ws_sender = ws_sender;
        tokio::spawn(async move {
            while let Some(msg) = peer_rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Subscribe to broadcast messages
        let mut rx = tx.subscribe();

        // Handle incoming messages
        loop {
            tokio::select! {
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            tracing::debug!("Received message from {}: {}", peer_id, text);
                            
                            // Try to parse as ClipboardMessage
                            match serde_json::from_str::<ClipboardMessage>(&text.to_string()) {
                                Ok(clipboard_msg) => {
                                    // Update metrics for received message
                                    {
                                        let mut metrics = sync_metrics.write().await;
                                        metrics.messages_received += 1;
                                        metrics.last_sync_time = Some(chrono::Utc::now());
                                    }
                                    
                                    // Check for duplicate message
                                    let mut cache = message_cache.write().await;
                                    if cache.is_duplicate(&clipboard_msg.id) {
                                        tracing::debug!("Ignoring duplicate message {}", clipboard_msg.id);
                                        continue;
                                    }
                                    
                                    // Add to cache
                                    cache.add_message(clipboard_msg.id);
                                    
                                    // Cleanup old messages if needed
                                    if cache.should_cleanup() {
                                        cache.cleanup_old_messages();
                                    }
                                    drop(cache);
                                    
                                    // Handle clipboard update with retry logic
                                    if let Some(ref content) = clipboard_msg.content {
                                        if let Some(ref callback) = *clipboard_callback.read().await {
                                            tracing::info!("Applying clipboard update from {}: {} chars", peer_id, content.len());
                                            
                                            // Retry clipboard update up to 3 times
                                            let mut retry_count = 0;
                                            let mut success = false;
                                            while retry_count < 3 {
                                                match tokio::time::timeout(Duration::from_secs(2), async {
                                                    callback(content.clone());
                                                }).await {
                                                    Ok(_) => {
                                                        tracing::debug!("Clipboard update successful on attempt {}", retry_count + 1);
                                                        success = true;
                                                        break;
                                                    }
                                                    Err(_) => {
                                                        retry_count += 1;
                                                        tracing::warn!("Clipboard update attempt {} failed, retrying...", retry_count);
                                                        if retry_count < 3 {
                                                            tokio::time::sleep(Duration::from_millis(100 * retry_count as u64)).await;
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            // Update metrics based on success/failure
                                            {
                                                let mut metrics = sync_metrics.write().await;
                                                if success {
                                                    metrics.clipboard_updates_applied += 1;
                                                } else {
                                                    metrics.clipboard_updates_failed += 1;
                                                    tracing::error!("Failed to update clipboard after 3 attempts");
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Broadcast to all other peers
                                    if let Err(e) = tx.send(text.to_string()) {
                                        tracing::error!("Failed to broadcast message: {}", e);
                                        let mut metrics = sync_metrics.write().await;
                                        metrics.messages_failed += 1;
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to parse clipboard message from {}: {}", peer_id, e);
                                    // Still broadcast raw message for compatibility
                                    if let Err(e) = tx.send(text.to_string()) {
                                        tracing::error!("Failed to broadcast message: {}", e);
                                        let mut metrics = sync_metrics.write().await;
                                        metrics.messages_failed += 1;
                                    }
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            tracing::info!("WebSocket connection closed for {}", peer_id);
                            break;
                        }
                        Some(Err(e)) => {
                            tracing::error!("WebSocket error for {}: {}", peer_id, e);
                            break;
                        }
                        _ => {}
                    }
                }
                broadcast_msg = rx.recv() => {
                    if let Ok(msg) = broadcast_msg {
                        // Don't echo back to sender
                        let peers_map = peers.read().await;
                        for (id, (_, peer_tx)) in peers_map.iter() {
                            if *id != peer_id {
                                let _ = peer_tx.send(Message::Text(msg.clone().into()));
                            }
                        }
                    }
                }
            }
        }

        // Remove peer from map on disconnect
        peers.write().await.remove(&peer_id);
        
        // Update connected peers count
        {
            let mut metrics = sync_metrics.write().await;
            metrics.connected_peers = peers.read().await.len() as u32;
        }
        
        Ok(())
    }

    pub async fn broadcast_message(&self, message: ClipboardMessage) -> Result<()> {
        // Add to our own cache to prevent processing our own messages
        {
            let mut cache = self.message_cache.write().await;
            cache.add_message(message.id);
            if cache.should_cleanup() {
                cache.cleanup_old_messages();
            }
        }
        
        let json = serde_json::to_string(&message)?;
        
        // Update metrics for sent message
        {
            let mut metrics = self.sync_metrics.write().await;
            metrics.messages_sent += 1;
            metrics.last_sync_time = Some(chrono::Utc::now());
        }
        
        match self.tx.send(json) {
            Ok(_) => {
                tracing::debug!("Message broadcast successfully");
                Ok(())
            }
            Err(broadcast::error::SendError(_)) => {
                // No receivers, which is normal when no clients are connected
                tracing::debug!("No connected clients to receive message");
                Ok(())
            }
        }
    }

    pub async fn get_connected_peers(&self) -> Vec<(Uuid, SocketAddr)> {
        self.peers.read().await
            .iter()
            .map(|(id, (addr, _))| (*id, *addr))
            .collect()
    }

    pub async fn get_sync_metrics(&self) -> SyncMetrics {
        let mut metrics = self.sync_metrics.read().await.clone();
        metrics.connected_peers = self.peers.read().await.len() as u32;
        metrics
    }
}