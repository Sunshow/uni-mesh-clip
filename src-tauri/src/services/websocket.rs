use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use std::collections::HashMap;
use uuid::Uuid;
use anyhow::Result;
use std::net::SocketAddr;
use crate::models::ClipboardMessage;

type Tx = broadcast::Sender<String>;
type PeerMap = Arc<RwLock<HashMap<Uuid, (SocketAddr, tokio::sync::mpsc::UnboundedSender<Message>)>>>;

pub struct WebSocketServer {
    port: u16,
    peers: PeerMap,
    tx: Tx,
    shutdown_tx: broadcast::Sender<()>,
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
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
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                tokio::spawn(Self::handle_connection(stream, addr, peers.clone(), tx.clone()));
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

    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        peers: PeerMap,
        tx: Tx,
    ) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let peer_id = Uuid::new_v4();
        tracing::info!("New WebSocket connection from {} with id {}", addr, peer_id);

        let (ws_sender, mut ws_receiver) = ws_stream.split();
        let (peer_tx, mut peer_rx) = tokio::sync::mpsc::unbounded_channel();

        // Add peer to the map
        peers.write().await.insert(peer_id, (addr, peer_tx));

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
                            // Broadcast to all other peers
                            if let Err(e) = tx.send(text.to_string()) {
                                tracing::error!("Failed to broadcast message: {}", e);
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
        Ok(())
    }

    pub async fn broadcast_message(&self, message: ClipboardMessage) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        let _ = self.tx.send(json);
        Ok(())
    }

    pub async fn get_connected_peers(&self) -> Vec<(Uuid, SocketAddr)> {
        self.peers.read().await
            .iter()
            .map(|(id, (addr, _))| (*id, *addr))
            .collect()
    }
}