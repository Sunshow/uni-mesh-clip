use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use tokio::sync::{RwLock};
use std::collections::HashMap;
use uuid::Uuid;
use anyhow::Result;
use std::net::SocketAddr;
use tokio::sync::broadcast;
use crate::models::ClipboardMessage;

type Tx = broadcast::Sender<String>;
type PeerMap = Arc<RwLock<HashMap<Uuid, (SocketAddr, tokio::sync::mpsc::UnboundedSender<Message>)>>>;

pub struct WebSocketServer {
    port: u16,
    peers: PeerMap,
    tx: Tx,
}

impl WebSocketServer {
    pub fn new(port: u16) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            port,
            peers: Arc::new(RwLock::new(HashMap::new())),
            tx,
        }
    }

    pub async fn start(&self) -> Result<()> {
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

        tokio::spawn(async move {
            while let Ok((stream, addr)) = listener.accept().await {
                tokio::spawn(Self::handle_connection(stream, addr, peers.clone(), tx.clone()));
            }
        });

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