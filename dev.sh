#!/bin/bash

# UniMesh Clip Development Script
# This script sets up the correct working directory and launches Tauri dev mode

echo "🚀 Starting UniMesh Clip development environment..."

# Check if we're in the right directory
if [ ! -f "package.json" ] || [ ! -d "src-tauri" ]; then
    echo "❌ Error: Please run this script from the project root directory"
    echo "   Current directory: $(pwd)"
    echo "   Expected files: package.json, src-tauri/"
    exit 1
fi

# Check if dependencies are installed
if [ ! -d "node_modules" ]; then
    echo "📦 Installing Node.js dependencies..."
    npm install
fi

# Check if Rust dependencies are cached
if [ ! -d "src-tauri/target" ]; then
    echo "🦀 First run - Rust dependencies will be downloaded and compiled..."
    echo "   This may take a few minutes..."
fi

echo "🔧 Building and starting application..."
echo "   Frontend: http://localhost:1420"
echo "   WebSocket: ws://localhost:8765"
echo ""
echo "💡 Tip: Use 'npm run tauri:dev' directly if you prefer npm scripts"
echo ""

# Start Tauri development mode
cd src-tauri && npx tauri dev