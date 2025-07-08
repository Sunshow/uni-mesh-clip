# PLANNING.md

## Project Overview
**Name**: UniMesh Clip  
**Purpose**: Cross-platform LAN clipboard synchronization application  
**Framework**: Tauri 2.0 with React/TypeScript frontend and Rust backend

## Architecture

### Core Architecture Principles
1. **Separation of Concerns**: Backend handles networking/clipboard, frontend handles UI/configuration
2. **Event-Driven**: Use Tauri's event system for real-time updates
3. **Security First**: Optional HMAC authentication, input validation
4. **Zero Configuration**: Works out of the box with sensible defaults

### Backend Architecture (Rust)

#### Service Layer
- **WebSocketService**: Manages WebSocket server and client connections
- **MdnsService**: Handles device discovery and service publishing
- **ClipboardService**: Monitors and updates system clipboard
- **SecurityService**: Manages authentication and message signing

#### Data Flow
1. ClipboardService detects change → Creates ClipboardMessage
2. SecurityService signs message (if enabled)
3. WebSocketService broadcasts to all connected devices
4. Receiving devices validate and update their clipboards

#### Key Patterns
- **Actor Model**: Each service runs independently with message passing
- **Error Handling**: Result<T, E> with custom error types
- **Async/Await**: Tokio runtime for concurrent operations

### Frontend Architecture (TypeScript/React)

#### Component Structure
- **App**: Main container, manages global state
- **StatusIndicator**: Shows sync status
- **DeviceDiscovery**: Lists discovered devices
- **Settings**: Configuration management

#### State Management
- Local component state for UI
- Tauri commands for backend communication
- Persistent settings via Tauri store plugin

### Communication Protocol

#### Message Format
```json
{
  "id": "uuid-v4",
  "type": "clipboard_update",
  "content": "clipboard_data",
  "timestamp": "2024-01-01T00:00:00Z",
  "signature": "base64_hmac_signature"
}
```

#### Security
- HMAC-SHA256 signature using shared secret
- Message deduplication via UUID
- Time window validation (±5 minutes)

## Technology Stack

### Backend
- **Tauri 2.0**: Application framework
- **tokio**: Async runtime
- **tokio-tungstenite**: WebSocket implementation
- **mdns**: Service discovery
- **arboard**: Clipboard access
- **serde**: Serialization
- **tracing**: Logging

### Frontend
- **React 18**: UI framework
- **TypeScript**: Type safety
- **Vite**: Build tool
- **@tauri-apps/api**: Tauri integration

## Code Organization

### File Structure
```
src-tauri/
├── src/
│   ├── commands/      # Tauri command handlers
│   ├── services/      # Core business logic
│   ├── models/        # Data structures
│   ├── utils/         # Helper functions
│   └── main.rs        # Application entry
src/
├── components/        # React components
├── hooks/            # Custom React hooks
├── types/            # TypeScript types
├── utils/            # Frontend utilities
└── main.tsx          # Frontend entry
```

### Naming Conventions
- **Rust**: snake_case for functions, PascalCase for types
- **TypeScript**: camelCase for functions, PascalCase for types/components
- **Files**: kebab-case for multi-word files

## Development Guidelines

### Code Style
- Run `cargo fmt` for Rust formatting
- Run `npm run lint` for TypeScript linting
- Keep functions under 50 lines
- Use descriptive variable names

### Testing Strategy
- Unit tests for individual services
- Integration tests for command handlers
- E2E tests for critical user flows

### Security Considerations
- Validate all clipboard content
- Sanitize HTML/rich text
- Rate limit incoming messages
- Use secure key storage

## Performance Targets
- Clipboard sync latency < 100ms
- CPU usage < 1% idle
- Memory usage < 50MB baseline
- Support 10+ concurrent devices

## Future Enhancements
1. Rich text/HTML clipboard support
2. File transfer capabilities
3. Clipboard history
4. Cloud relay option
5. Mobile companion apps