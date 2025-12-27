#!/bin/bash

# ============================================
# Java Perf v6.0.0 (Rust) - 手动安装脚本
# ============================================
#
# Plugin 模式：推荐使用 /plugin install java-perf
# 此脚本用于手动安装（开发或离线场景）
#
# 用法:
#   ./install.sh

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════╗"
echo "║  Java Perf v6.0.0 (Plugin Mode)           ║"
echo "║  或使用 /plugin install java-perf          ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""

# 检测平台
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
        echo -e "${RED}❌ 不支持的平台: $PLATFORM-$ARCH${NC}"
        echo "   请从源码编译: cd rust && cargo build --release"
        exit 1
        ;;
esac

# 安装目录
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

# GitHub Release URL
REPO="ly87ing/dev-skills"
RELEASE_URL="https://github.com/$REPO/releases/latest/download/$BINARY"

echo -e "${YELLOW}[1/2] 安装二进制文件...${NC}"

if [ -f "$SCRIPT_DIR/rust/target/release/java-perf" ]; then
    # 优先使用本地刚刚编译的
    cp "$SCRIPT_DIR/rust/target/release/java-perf" "$INSTALL_DIR/java-perf"
    echo -e "${GREEN}✓ 使用本地编译版本${NC}"
elif [ -f "$SCRIPT_DIR/releases/$BINARY" ]; then
    # 使用本地 release
    cp "$SCRIPT_DIR/releases/$BINARY" "$INSTALL_DIR/java-perf"
    echo -e "${GREEN}✓ 使用本地二进制${NC}"
elif command -v curl &> /dev/null; then
    # 尝试下载
    echo "  尝试从 GitHub Release 下载..."
    if curl -fsSL "$RELEASE_URL" -o "$INSTALL_DIR/java-perf" 2>/dev/null; then
        echo -e "${GREEN}✓ 下载完成${NC}"
    else
        echo -e "${YELLOW}⚠ 下载失败，尝试从源码编译...${NC}"
        # 从源码编译
        if command -v cargo &> /dev/null; then
            echo "  正在编译..."
            cd "$SCRIPT_DIR/rust"
            cargo build --release
            cp target/release/java-perf "$INSTALL_DIR/java-perf"
            # 恢复目录
            cd "$SCRIPT_DIR"
            echo -e "${GREEN}✓ 编译完成${NC}"
        else
            echo -e "${RED}❌ 下载失败且未安装 Rust (https://rustup.rs)${NC}"
            exit 1
        fi
    fi
else
    echo -e "${RED}❌ 需要 curl 或从源码编译${NC}"
    exit 1
fi

chmod +x "$INSTALL_DIR/java-perf"
echo -e "${GREEN}  路径: $INSTALL_DIR/java-perf${NC}"

# 检查 PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo -e "${YELLOW}⚠ 请将以下路径添加到 PATH:${NC}"
    echo -e "   export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

# 安装 Skill
echo ""
echo -e "${YELLOW}[2/2] 安装 Skill...${NC}"
SKILL_SOURCE="$SCRIPT_DIR/skills/java-perf"
SKILL_TARGET="$HOME/.claude/skills/java-perf"

mkdir -p "$HOME/.claude/skills"
rm -rf "$SKILL_TARGET" 2>/dev/null || true
if [ -d "$SKILL_SOURCE" ]; then
    cp -r "$SKILL_SOURCE" "$SKILL_TARGET"
    echo -e "${GREEN}✓ Skill 已安装${NC}"
else
    echo -e "${YELLOW}⚠ Skill 目录未找到${NC}"
fi

# 完成
echo ""
echo -e "${GREEN}"
echo "╔════════════════════════════════════════════╗"
echo "║           ✓ 安装完成！                     ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""
echo "使用方式："
echo -e "  ${YELLOW}java-perf scan --path ./src${NC}    # 扫描项目"
echo -e "  ${YELLOW}java-perf status${NC}               # 检查引擎状态"
echo ""
