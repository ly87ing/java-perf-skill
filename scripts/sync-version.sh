#!/bin/bash
# 版本同步脚本 - 确保所有文件版本号一致
# 用法: ./scripts/sync-version.sh [new_version]
#       ./scripts/sync-version.sh --check (仅检查不修改)

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# 获取 Cargo.toml 中的版本（作为真实版本源）
get_cargo_version() {
    grep -m1 '^version = ' "$ROOT_DIR/rust/Cargo.toml" | sed 's/version = "\(.*\)"/\1/'
}

# 检查模式
if [[ "$1" == "--check" ]]; then
    CARGO_VERSION=$(get_cargo_version)
    echo "📦 Cargo.toml 版本: $CARGO_VERSION"

    ERRORS=0

    # 检查 README.md
    if grep -q "Version-${CARGO_VERSION}-blue" "$ROOT_DIR/rust/README.md"; then
        echo -e "${GREEN}✓${NC} rust/README.md: $CARGO_VERSION"
    else
        echo -e "${RED}✗${NC} rust/README.md 版本不一致"
        ERRORS=$((ERRORS + 1))
    fi

    # 检查 CHANGELOG.md
    if grep -q "## \[${CARGO_VERSION}\]" "$ROOT_DIR/rust/CHANGELOG.md"; then
        echo -e "${GREEN}✓${NC} rust/CHANGELOG.md: $CARGO_VERSION"
    else
        echo -e "${RED}✗${NC} rust/CHANGELOG.md 缺少 $CARGO_VERSION 条目"
        ERRORS=$((ERRORS + 1))
    fi

    # 检查 SKILL.md (可选 - 如果 SKILL.md 包含版本号则检查)
    if grep -q "(v[0-9]" "$ROOT_DIR/skills/java-perf/SKILL.md" 2>/dev/null; then
        if grep -q "(v${CARGO_VERSION})" "$ROOT_DIR/skills/java-perf/SKILL.md"; then
            echo -e "${GREEN}✓${NC} skills/java-perf/SKILL.md: $CARGO_VERSION"
        else
            echo -e "${RED}✗${NC} skills/java-perf/SKILL.md 版本不一致"
            ERRORS=$((ERRORS + 1))
        fi
    else
        echo -e "${GREEN}✓${NC} skills/java-perf/SKILL.md: (无版本号 - 跳过)"
    fi

    if [[ $ERRORS -gt 0 ]]; then
        echo -e "\n${RED}发现 $ERRORS 处版本不一致${NC}"
        echo "运行 ./scripts/sync-version.sh 自动同步"
        exit 1
    else
        echo -e "\n${GREEN}✓ 所有版本一致: $CARGO_VERSION${NC}"
        exit 0
    fi
fi

# 同步模式
NEW_VERSION="${1:-$(get_cargo_version)}"
echo "🔄 同步版本至: $NEW_VERSION"

# 更新 README.md 标题和徽章
sed -i.bak "s/# Java Perf v[0-9]*\.[0-9]*\.[0-9]*/# Java Perf v${NEW_VERSION}/" "$ROOT_DIR/rust/README.md"
sed -i.bak "s/Version-[0-9]*\.[0-9]*\.[0-9]*-blue/Version-${NEW_VERSION}-blue/" "$ROOT_DIR/rust/README.md"
sed -i.bak "s/> v[0-9]*\.[0-9]*\.[0-9]* 特性/> v${NEW_VERSION} 特性/" "$ROOT_DIR/rust/README.md"

# 更新 SKILL.md
sed -i.bak "s/(v[0-9]*\.[0-9]*\.[0-9]*)/(v${NEW_VERSION})/" "$ROOT_DIR/skills/java-perf/SKILL.md"

# 清理备份文件
find "$ROOT_DIR" -name "*.bak" -delete

echo -e "${GREEN}✓ 版本同步完成: $NEW_VERSION${NC}"
echo ""
echo "已更新文件:"
echo "  - rust/README.md"
echo "  - skills/java-perf/SKILL.md"
echo ""
echo "注意: Cargo.toml 和 CHANGELOG.md 需要手动更新"
