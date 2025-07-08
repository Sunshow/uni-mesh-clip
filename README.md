# UniMesh Clip

Cross-platform LAN clipboard synchronization application built with Tauri 2.0 that enables seamless clipboard sharing between devices on the same local network.

## Features

- 🔄 **Real-time Sync**: Instant clipboard synchronization across devices
- 🔍 **Auto Discovery**: Zero-configuration device discovery via mDNS
- 🔒 **Security**: Optional HMAC authentication for trusted networks
- 🖥️ **Cross-platform**: Windows, macOS, and Linux support
- 📊 **System Tray**: Minimal UI with system tray integration
- ⚡ **High Performance**: Low latency (<100ms) synchronization

## Prerequisites

- Node.js 18+ and npm
- Rust 1.70+
- Platform-specific dependencies:
  - **Windows**: Microsoft Visual Studio C++ Build Tools
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/uni-mesh-clip.git
cd uni-mesh-clip
```

2. Install dependencies:
```bash
npm install
```

3. Copy environment configuration:
```bash
cp .env.example .env
```

4. Build the application:
```bash
npm run tauri build
```

## Development

### Quick Start
Use the convenient development script:
```bash
./dev.sh
```

### Manual Development
Alternatively, use npm scripts:
```bash
# Option 1: Direct Tauri command (from project root)
npm run tauri:dev

# Option 2: Navigate to src-tauri directory
cd src-tauri
npx tauri dev
```

This will start both the Vite dev server and the Tauri application with hot reload.

### Development Notes
- **First run**: Rust dependencies will be downloaded and compiled (may take several minutes)
- **Frontend**: Available at http://localhost:1420
- **WebSocket**: Server runs on ws://localhost:8765
- **Hot reload**: Frontend changes are reflected immediately
- **Rust changes**: Trigger automatic recompilation

### Development Scripts
```bash
npm run dev          # Start frontend dev server only
npm run tauri:dev    # Start full Tauri development environment
npm run tauri:build  # Build production application
npm run typecheck    # TypeScript type checking
npm run lint         # ESLint code quality check
```

## Configuration

Edit `.env` file to customize:

- `WEBSOCKET_PORT`: WebSocket server port (default: 8765)
- `MDNS_SERVICE_NAME`: mDNS service name (default: unimesh-clip)
- `SECURITY_KEY`: Optional shared secret for authentication
- `LOG_LEVEL`: Logging verbosity (trace, debug, info, warn, error)

## Usage

1. **Launch**: Start UniMesh Clip on all devices you want to sync
2. **Discovery**: Devices will automatically discover each other on the LAN
3. **Sync**: Copy text on one device - it appears on all connected devices
4. **Security** (optional): Set the same security key on all devices for authentication

### System Tray

- **Show**: Opens the main window
- **Quit**: Exits the application

### Main Window

- **Status**: Shows current sync status
- **Devices**: Lists discovered devices on the network
- **Settings**: Configure port, security, and preferences

## Architecture

### Backend (Rust)
- WebSocket server for real-time communication
- mDNS for service discovery
- Platform-specific clipboard monitoring
- HMAC-SHA256 message authentication

### Frontend (React/TypeScript)
- Settings management
- Device discovery UI
- Status monitoring
- System tray integration

### Security
- Optional shared secret authentication
- Message deduplication via UUID
- Time window validation
- Input sanitization

## Building for Production

### Windows
```bash
npm run tauri:build -- --target x86_64-pc-windows-msvc
```

### macOS
```bash
npm run tauri:build -- --target universal-apple-darwin
```

### Linux
```bash
npm run tauri:build -- --target x86_64-unknown-linux-gnu
```

## Network Requirements

- Devices must be on the same local network
- mDNS (port 5353) must be allowed
- WebSocket port (default 8765) must be accessible
- Firewall may need configuration for:
  - Incoming WebSocket connections
  - mDNS multicast traffic

## Troubleshooting

### Devices not discovering each other
- Check firewall settings
- Ensure mDNS is not blocked
- Verify devices are on same subnet
- Try manual connection via IP

### Clipboard not syncing
- Check security key matches on all devices
- Verify WebSocket port is not in use
- Check application has clipboard permissions

### High CPU usage
- Adjust clipboard polling interval in settings
- Check for clipboard monitoring loops
- Disable unnecessary device connections

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `npm run test`
5. Submit a pull request

## License

MIT License - see LICENSE file for details

## Acknowledgments

- Built with [Tauri](https://tauri.app/)
- WebSocket implementation using [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)
- Clipboard access via [arboard](https://github.com/1Password/arboard)
- mDNS discovery using [mdns](https://github.com/dylanmckay/mdns)