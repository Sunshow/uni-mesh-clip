# INITIAL.md

## FEATURE:

Cross-platform LAN clipboard synchronization application built with Tauri 2.0 that enables seamless clipboard sharing between devices on the same local network.

### Core Components:
- **WebSocket Server**: Local WebSocket service for clipboard data broadcasting and receiving
- **mDNS Discovery**: Automatic service discovery and publishing using multicast DNS
- **Clipboard Monitor**: Real-time clipboard change detection and synchronization
- **Security Layer**: Optional shared secret key for device authentication
- **Configuration UI**: User-friendly interface for service settings and device management

### Key Features:
- **Cross-platform Support**: Windows, macOS, and Linux compatibility
- **Automatic Discovery**: Zero-configuration device discovery via mDNS
- **Real-time Sync**: Instant clipboard synchronization across connected devices
- **Deduplication**: UUID-based message deduplication to prevent loops
- **Security**: Optional shared secret authentication for trusted device networks
- **Configurable**: Customizable WebSocket port, security settings, and sync preferences

## TECHNICAL ARCHITECTURE:

### Backend (Rust):
- **WebSocket Server**: Using `tokio-tungstenite` for WebSocket communication
- **mDNS Service**: Using `mdns` crate for service discovery and publishing
- **Clipboard Integration**: Platform-specific clipboard APIs via `arboard` or similar
- **Message Handling**: UUID-based deduplication and secure message routing
- **Configuration Management**: Persistent settings storage

### Frontend (TypeScript/React):
- **Settings Interface**: Port configuration, security key management
- **Device Discovery**: Real-time display of discovered devices
- **Status Monitoring**: Connection status and sync activity
- **Security Management**: Trusted device list and authentication settings

### Communication Protocol:
```json
{
  "id": "uuid-v4",
  "type": "clipboard_update",
  "content": "clipboard_data",
  "timestamp": "iso8601",
  "signature": "hmac_sha256_optional"
}
```

## EXAMPLES:

The `examples/` folder contains reference implementations and patterns:
- `examples/websocket_server.rs` - WebSocket server setup and message handling
- `examples/mdns_discovery.rs` - Service discovery and publishing patterns
- `examples/clipboard_monitor.rs` - Clipboard change detection
- `examples/ui_components/` - Frontend component examples for settings and device management

Don't copy these examples directly, but use them as inspiration for best practices in Tauri 2.0 development, async Rust patterns, and secure communication protocols.

## DOCUMENTATION:

- Tauri 2.0 Documentation: https://v2.tauri.app/
- tokio-tungstenite: https://docs.rs/tokio-tungstenite/
- mdns crate: https://docs.rs/mdns/
- arboard (clipboard): https://docs.rs/arboard/

## OTHER CONSIDERATIONS:

### Security Requirements:
- Optional HMAC-SHA256 message authentication using shared secret
- Input validation for all clipboard data
- Rate limiting to prevent spam/abuse
- Secure storage of configuration data

### Configuration Management:
- `.env.example` with default settings and required environment variables
- Persistent configuration storage using Tauri's built-in store
- Runtime configuration updates without restart

### Setup Requirements:
- README with comprehensive setup instructions
- Platform-specific installation notes
- Network configuration guidelines (firewall, mDNS requirements)
- Security best practices documentation

### Project Structure:
```
src-tauri/
├── src/
│   ├── commands/          # Tauri command handlers
│   ├── services/          # WebSocket, mDNS, clipboard services
│   ├── models/           # Data structures and types
│   ├── utils/            # Helper functions and utilities
│   └── main.rs
├── Cargo.toml
└── tauri.conf.json

src/
├── components/           # React components
├── hooks/               # Custom React hooks
├── types/               # TypeScript type definitions
├── utils/               # Frontend utilities
└── main.tsx

tests/
├── unit/                # Unit tests for Rust and TypeScript
├── integration/         # Integration tests
└── e2e/                 # End-to-end tests
```

### Environment Variables:
- `WEBSOCKET_PORT` - Default WebSocket server port
- `MDNS_SERVICE_NAME` - Service name for mDNS discovery
- `SECURITY_KEY` - Optional shared secret for device authentication
- `LOG_LEVEL` - Application logging level

### Platform Considerations:
- Windows: Handle clipboard format variations and permissions
- macOS: Sandbox compatibility and accessibility permissions
- Linux: X11/Wayland clipboard integration
- Network: mDNS availability and firewall configuration