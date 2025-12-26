#!/bin/bash

# ============================================
# Java Performance Diagnostics - 更新脚本
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
echo "║  Java Performance Diagnostics Updater      ║"
echo "║  Java 性能诊断工具 - 更新                   ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"

# 检查是否是 git 仓库
if [ ! -d "$SCRIPT_DIR/.git" ]; then
    echo -e "${RED}❌ 当前目录不是 git 仓库，无法自动更新${NC}"
    echo "   请手动下载最新版本: https://github.com/ly87ing/java-perf-skill"
    exit 1
fi

# 拉取最新代码
echo -e "${YELLOW}[1/4] 拉取最新代码...${NC}"
cd "$SCRIPT_DIR"
git fetch origin
BEHIND=$(git rev-list HEAD..origin/main --count 2>/dev/null || echo "0")

if [ "$BEHIND" = "0" ]; then
    echo -e "${GREEN}✓ 已是最新版本${NC}"
else
    echo -e "${YELLOW}  发现 ${BEHIND} 个新提交，正在更新...${NC}"
    git pull origin main
    echo -e "${GREEN}✓ 代码更新完成${NC}"
fi

# 重新编译 MCP Server
echo ""
echo -e "${YELLOW}[2/4] 重新编译 MCP Server...${NC}"
cd "$SCRIPT_DIR/mcp"
npm install --silent
npm run build --silent
echo -e "${GREEN}✓ MCP Server 编译完成${NC}"

# 更新 Skill
echo ""
echo -e "${YELLOW}[3/4] 更新 Skill...${NC}"
SKILL_SOURCE="$SCRIPT_DIR/skill"
SKILL_TARGET="$HOME/.claude/skills/java-perf"

if [ -d "$SKILL_TARGET" ]; then
    rm -rf "$SKILL_TARGET"
fi
cp -r "$SKILL_SOURCE" "$SKILL_TARGET"
echo -e "${GREEN}✓ Skill 更新完成${NC}"

# 重新注册 MCP（确保使用最新编译版本）
echo ""
echo -e "${YELLOW}[4/5] 重新注册 MCP Server...${NC}"
MCP_PATH="$SCRIPT_DIR/mcp/dist/index.js"

if command -v claude &> /dev/null; then
    # 清理并重新注册
    claude mcp remove java-perf --scope user 2>/dev/null || true
    claude mcp remove java-perf --scope project 2>/dev/null || true
    sleep 1
    claude mcp add java-perf --scope user -- node "$MCP_PATH"
    
    # 验证
    sleep 2
    if claude mcp list 2>&1 | grep -q "java-perf.*Connected"; then
        echo -e "${GREEN}✓ MCP Server 重新注册并验证成功${NC}"
    else
        echo -e "${YELLOW}⚠ MCP Server 已注册，可能需要重启 Claude Code${NC}"
    fi
else
    echo -e "${YELLOW}⚠ 跳过 MCP 注册（claude 命令未找到）${NC}"
fi

# 显示更新日志
echo ""
echo -e "${YELLOW}[5/5] 最近更新日志...${NC}"
git log --oneline -5 2>/dev/null || true

# 完成
echo ""
echo -e "${GREEN}"
echo "╔════════════════════════════════════════════╗"
echo "║           ✓ 更新完成！                     ║"
echo "╚════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""
echo "新功能将在下次调用时生效"
echo ""
