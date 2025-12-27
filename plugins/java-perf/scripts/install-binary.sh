#!/bin/bash

# ============================================
# Java Perf v8.0.0 - Binary Installation Script
# ============================================
# Called by SessionStart hook when plugin session starts.
# Uses CLAUDE_PLUGIN_ROOT environment variable.
# Idempotent: only installs if binary is missing.

# Exit on undefined variables (but not on error, for idempotent behavior)
set -u

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Install directory
INSTALL_DIR="$HOME/.local/bin"
BINARY_PATH="$INSTALL_DIR/java-perf"

# Idempotent check: if binary exists and is executable, skip installation
if [ -x "$BINARY_PATH" ]; then
    exit 0
fi

echo -e "${YELLOW}Installing java-perf binary...${NC}"

# Detect platform
PLATFORM=$(uname -s)
ARCH=$(uname -m)

case "$PLATFORM-$ARCH" in
    Darwin-arm64)
        BINARY="java-perf-darwin-arm64"
        ;;
    Darwin-x86_64)
        BINARY="java-perf-darwin-x64"
        ;;
    Linux-x86_64)
        BINARY="java-perf-linux-x64"
        ;;
    *)
        echo -e "${RED}❌ Unsupported platform: $PLATFORM-$ARCH${NC}" >&2
        echo "   Please build from source: cd rust && cargo build --release" >&2
        exit 1
        ;;
esac

mkdir -p "$INSTALL_DIR"

# GitHub Release URL
REPO="ly87ing/java-perf-skill"
RELEASE_URL="https://github.com/$REPO/releases/latest/download/$BINARY"

# Get plugin root from environment variable (set by Claude Code)
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-}"

# Try installation methods in order
INSTALLED=false

# 1. Check for local compiled binary (development mode)
if [ -n "$PLUGIN_ROOT" ] && [ -f "$PLUGIN_ROOT/rust/target/release/java-perf" ]; then
    cp "$PLUGIN_ROOT/rust/target/release/java-perf" "$BINARY_PATH"
    chmod +x "$BINARY_PATH"
    echo -e "${GREEN}✓ Installed from local build${NC}"
    INSTALLED=true
fi

# 2. Try downloading from GitHub Release
if [ "$INSTALLED" = "false" ] && command -v curl &> /dev/null; then
    if curl -fsSL "$RELEASE_URL" -o "$BINARY_PATH.tmp" 2>/dev/null; then
        chmod +x "$BINARY_PATH.tmp"
        mv "$BINARY_PATH.tmp" "$BINARY_PATH"
        echo -e "${GREEN}✓ Downloaded from GitHub Release${NC}"
        INSTALLED=true
    else
        rm -f "$BINARY_PATH.tmp"
    fi
fi

# 3. Try building from source
if [ "$INSTALLED" = "false" ] && [ -n "$PLUGIN_ROOT" ] && command -v cargo &> /dev/null; then
    echo "  Building from source..."
    if [ -d "$PLUGIN_ROOT/rust" ]; then
        (cd "$PLUGIN_ROOT/rust" && cargo build --release)
        cp "$PLUGIN_ROOT/rust/target/release/java-perf" "$BINARY_PATH"
        chmod +x "$BINARY_PATH"
        echo -e "${GREEN}✓ Built from source${NC}"
        INSTALLED=true
    fi
fi

if [ "$INSTALLED" = "false" ]; then
    echo -e "${RED}❌ Installation failed${NC}" >&2
    echo "   Please install Rust (https://rustup.rs) or download manually" >&2
    exit 1
fi

# Check PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}⚠ Add to PATH: export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
fi

echo -e "${GREEN}✓ java-perf installed${NC}"
