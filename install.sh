#!/bin/bash

# ============================================
# Java Performance Diagnostics - 一键安装脚本
# ============================================

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════╗"
echo "║  Java Performance Diagnostics Installer    ║"
echo "║  Java 性能诊断工具 - 一键安装               ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"

# 检查 Node.js
echo -e "${YELLOW}[1/4] 检查 Node.js 环境...${NC}"
if ! command -v node &> /dev/null; then
    echo -e "${RED}❌ Node.js 未安装，请先安装 Node.js${NC}"
    echo "   安装方式: brew install node 或 访问 https://nodejs.org"
    exit 1
fi
NODE_VERSION=$(node -v)
echo -e "${GREEN}✓ Node.js ${NODE_VERSION} 已安装${NC}"

# 检查 npm
if ! command -v npm &> /dev/null; then
    echo -e "${RED}❌ npm 未安装${NC}"
    exit 1
fi
echo -e "${GREEN}✓ npm $(npm -v) 已安装${NC}"

# 编译 MCP Server
echo ""
echo -e "${YELLOW}[2/4] 编译 MCP Server...${NC}"
cd "$SCRIPT_DIR/mcp"
npm install --silent
npm run build --silent
echo -e "${GREEN}✓ MCP Server 编译完成${NC}"

# 获取 MCP 路径
MCP_PATH="$SCRIPT_DIR/mcp/dist/index.js"
echo -e "${GREEN}  路径: ${MCP_PATH}${NC}"

# 注册 MCP 到 Claude（用户级，全局生效）
echo ""
echo -e "${YELLOW}[3/4] 注册 MCP 到 Claude Code（用户级）...${NC}"

# 检查 claude 命令
if command -v claude &> /dev/null; then
    # 先尝试移除旧的（忽略错误）
    claude mcp remove java-perf --scope user 2>/dev/null || true
    
    # 添加新的（用户级，全局生效）
    claude mcp add java-perf --scope user -- node "$MCP_PATH"
    echo -e "${GREEN}✓ MCP Server 已注册到 Claude Code（用户级）${NC}"
else
    echo -e "${YELLOW}⚠ claude 命令未找到，请手动注册 MCP:${NC}"
    echo -e "   claude mcp add java-perf --scope user -- node ${MCP_PATH}"
fi

# 安装 Skill
echo ""
echo -e "${YELLOW}[4/4] 安装 Skill...${NC}"
SKILL_SOURCE="$SCRIPT_DIR/skill"
SKILL_TARGET="$HOME/.claude/skills/java-perf"

# 创建目标目录
mkdir -p "$HOME/.claude/skills"

# 复制 Skill
rm -rf "$SKILL_TARGET" 2>/dev/null || true
cp -r "$SKILL_SOURCE" "$SKILL_TARGET"
echo -e "${GREEN}✓ Skill 已安装到 ${SKILL_TARGET}${NC}"

# 完成
echo ""
echo -e "${GREEN}"
echo "╔════════════════════════════════════════════╗"
echo "║           ✓ 安装完成！                     ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""
echo "使用方式："
echo "  在 Claude Code 中描述你的性能问题，例如："
echo ""
echo -e "  ${BLUE}帮我分析一下内存暴涨的问题...${NC}"
echo -e "  ${BLUE}系统响应很慢，CPU占用很高...${NC}"
echo -e "  ${BLUE}消息队列出现大量积压...${NC}"
echo ""
echo "验证安装："
echo -e "  ${YELLOW}claude mcp list${NC}  # 查看已安装的 MCP"
echo ""
