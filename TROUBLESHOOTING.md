# Development Troubleshooting Guide

## Common Issues and Solutions

### 1. "could not find Cargo.toml" Error

**Problem**: 
```
error: could not find `Cargo.toml` in `/Users/username/project` or any parent directory
```

**Solution**: 
Use the updated npm scripts that navigate to the correct directory:
```bash
npm run tauri:dev     # Instead of: npx tauri dev
npm run tauri:build   # Instead of: npx tauri build
```

Or use the development script:
```bash
./dev.sh
```

### 2. "channel closed" Error on Startup

**Problem**:
```
ERROR Failed to broadcast clipboard update: Failed to broadcast message: channel closed
```

**Status**: ✅ **FIXED** - This error has been resolved in the latest code.

**Explanation**: This occurred when the application tried to broadcast clipboard content before any WebSocket clients were connected. The error is now handled gracefully and only logged at debug level.

### 3. WebSocket "address already in use" Error

**Problem**:
```
Failed to bind WebSocket server to 127.0.0.1:8765: Address already in use
```

**Solutions**:
1. **Stop existing process**: Kill any running instances of the app
2. **Change port**: Edit the port in your configuration
3. **Wait for cleanup**: The improved shutdown logic now properly releases ports

### 4. Clipboard Permission Issues (macOS)

**Problem**:
```
Clipboard initialization timed out - this usually means permission is required
```

**Solutions**:
1. Grant clipboard access to Terminal/your IDE in System Preferences → Security & Privacy → Privacy → Accessibility
2. The app will continue to work for network sync even without clipboard access
3. Restart the application after granting permissions

### 5. mDNS Discovery Not Working

**Problem**: Devices don't appear in discovery list

**Solutions**:
1. **Firewall**: Ensure mDNS (port 5353) is allowed
2. **Network**: Verify devices are on the same subnet
3. **Debug logs**: Check console for mDNS-related errors
4. **Sample devices**: In debug mode, sample devices are added for testing

### 6. Slow First Compilation

**Problem**: Initial `cargo` compilation takes a very long time

**Explanation**: This is normal for the first run. Rust compiles all dependencies from source.

**Solutions**:
1. Be patient - subsequent builds are much faster
2. Use `cargo check` for faster syntax checking during development
3. Consider using `sccache` for distributed compilation caching

### 7. Hot Reload Not Working

**Problem**: Changes don't reflect in the running application

**Solutions**:
1. **Frontend changes**: Should reload automatically via Vite
2. **Rust changes**: Trigger automatic recompilation
3. **Configuration changes**: Usually require a full restart
4. **Clear cache**: Delete `target/` directory and restart

## Development Best Practices

### Code Quality
```bash
# Run before committing
npm run typecheck    # Check TypeScript
npm run lint         # ESLint check
cargo clippy         # Rust linting
cargo fmt           # Rust formatting
```

### Debugging
1. **Rust logs**: Use `tracing::info!`, `tracing::error!`, etc.
2. **Frontend logs**: Standard `console.log` works
3. **Tauri DevTools**: Available in development builds
4. **Log levels**: Set via environment or configuration

### Performance
1. **Development builds** are slower than release builds
2. **Clipboard polling** runs every 500ms - normal CPU usage
3. **Memory usage** should be <50MB in normal operation
4. **Network traffic** is minimal (only clipboard changes)

## Getting Help

1. **Check logs**: Console output often shows the root cause
2. **Tauri docs**: https://tauri.app/v2/guides/
3. **GitHub issues**: Search existing issues first
4. **Discord**: Tauri community Discord server

## Useful Commands

```bash
# Full clean and restart
rm -rf target/ node_modules/
npm install
./dev.sh

# Check compilation without running
cargo check

# View dependency tree
cargo tree

# Update dependencies
cargo update
npm update

# Build for specific platform
npm run tauri:build -- --target x86_64-unknown-linux-gnu
```