#!/bin/bash

# ============================================
# Java Perf v4.0.0 (Rust) - 一键安装脚本
# ============================================

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
echo "║  Java Perf v4.0.0 (Rust Radar-Sniper)      ║"
echo "║  零依赖，单二进制                           ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"

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
        echo "   请从源码编译: cd rust-mcp && cargo build --release"
        exit 1
        ;;
esac

# 安装目录
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

# 检查是否有预编译二进制
RELEASE_URL="https://github.com/ly87ing/java-perf-skill/releases/latest/download/$BINARY"

echo -e "${YELLOW}[1/3] 下载二进制文件...${NC}"

if [ -f "$SCRIPT_DIR/releases/$BINARY" ]; then
    # 使用本地文件
    cp "$SCRIPT_DIR/releases/$BINARY" "$INSTALL_DIR/java-perf"
    echo -e "${GREEN}✓ 使用本地二进制${NC}"
elif command -v curl &> /dev/null; then
    # 尝试下载
    if curl -fsSL "$RELEASE_URL" -o "$INSTALL_DIR/java-perf" 2>/dev/null; then
        echo -e "${GREEN}✓ 下载完成${NC}"
    else
        echo -e "${YELLOW}⚠ 下载失败，尝试从源码编译...${NC}"
        # 从源码编译
        if command -v cargo &> /dev/null; then
            cd "$SCRIPT_DIR/rust-mcp"
            cargo build --release
            cp target/release/java-perf "$INSTALL_DIR/java-perf"
            echo -e "${GREEN}✓ 编译完成${NC}"
        else
            echo -e "${RED}❌ 需要安装 Rust: https://rustup.rs${NC}"
            exit 1
        fi
    fi
else
    echo -e "${RED}❌ 需要 curl 或从源码编译${NC}"
    exit 1
fi

chmod +x "$INSTALL_DIR/java-perf"
echo -e "${GREEN}  路径: $INSTALL_DIR/java-perf${NC}"

# 注册 MCP
echo ""
echo -e "${YELLOW}[2/3] 注册 MCP 到 Claude Code...${NC}"

if command -v claude &> /dev/null; then
    # 清理旧注册
    claude mcp remove java-perf -s local 2>/dev/null || true
    claude mcp remove java-perf -s user 2>/dev/null || true
    claude mcp remove java-perf -s project 2>/dev/null || true
    sleep 1
    
    # 注册新的
    claude mcp add java-perf --scope user -- "$INSTALL_DIR/java-perf"
    
    # 验证
    sleep 2
    if claude mcp list 2>&1 | grep -q "java-perf.*Connected"; then
        echo -e "${GREEN}✓ MCP Server 已注册并验证成功${NC}"
    else
        echo -e "${YELLOW}⚠ MCP Server 已注册，可能需要重启 Claude Code${NC}"
    fi
else
    echo -e "${YELLOW}⚠ claude 命令未找到，请手动注册:${NC}"
    echo -e "   claude mcp add java-perf --scope user -- $INSTALL_DIR/java-perf"
fi

# 安装 Skill
echo ""
echo -e "${YELLOW}[3/3] 安装 Skill...${NC}"
SKILL_SOURCE="$SCRIPT_DIR/../skill"
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
echo "优势："
echo -e "  ${GREEN}零依赖${NC}  - 不需要 Node.js"
echo -e "  ${GREEN}1.9MB${NC}   - 单二进制文件"
echo -e "  ${GREEN}~5ms${NC}    - 毫秒级启动"
echo ""
echo "验证安装："
echo -e "  ${YELLOW}claude mcp list${NC}"
echo ""
