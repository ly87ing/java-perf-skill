# Claude Skills

<p align="center">
  <img src="https://img.shields.io/badge/Claude-Skills-blue" alt="Claude Skills">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="MIT License">
  <img src="https://img.shields.io/badge/Version-1.0.0-orange" alt="Version">
</p>

Claude Agent Skills 集合，包含多个可复用的领域特定技能。

## 目录结构

```
claude-skills/
├── performance-troubleshoot/   # 性能问题排查 Skill
│   ├── SKILL.md                # 主文件 - 诊断流程和场景化引导
│   ├── REFERENCE.md            # 参考资料 - 决策树 / 诊断表 / 工具推荐
│   ├── CHECKLIST.md            # 审查检查清单 - 6+1 类分类索引
│   └── TEMPLATE.md             # 文档模板 - 动态指标模板
├── README.md
└── LICENSE
```

## 安装

### 方法 1: 安装到 ~/.claude/skills (推荐，全局生效)

```bash
# 1. 克隆仓库
git clone https://github.com/ly87ing/claude-skills.git
cd claude-skills

# 2. 创建 Claude skills 目录 (如果不存在)
mkdir -p ~/.claude/skills

# 3. 复制 skill 到 Claude 全局目录
cp -r performance-troubleshoot ~/.claude/skills/
```

### 方法 2: 安装到项目目录 (仅对该项目生效)

```bash
# 1. 克隆仓库
git clone https://github.com/ly87ing/claude-skills.git
cd claude-skills

# 2. 复制到目标项目的 .agent/skills 目录
mkdir -p /path/to/your-project/.agent/skills
cp -r performance-troubleshoot /path/to/your-project/.agent/skills/
```

> **注意**: 安装后需要重启 Claude 才能加载新的 Skill。

## 可用 Skills

### [performance-troubleshoot](./performance-troubleshoot/)

性能与资源问题排查 Skill，包含自动化的多轮审查。

**触发方式**: 描述性能问题即可自动触发

```
请帮我排查内存暴涨问题，从 3GB 涨到 16GB...
系统响应很慢，CPU 使用率很高...
消息队列出现大量积压...
```

**适用场景**:

| 问题类型 | 示例 |
|----------|------|
| **响应慢** | 接口延迟高、吞吐低、锁竞争 |
| **CPU问题** | CPU使用率高、负载高、死循环 |
| **内存问题** | 内存暴涨、OOM、GC频繁、泄露 |
| **资源耗尽** | 连接池满、线程池满、句柄不足 |
| **服务不可用** | 宕机、超时、错误率高、雪崩 |
| **消息积压** | 队列积压、消费延迟、背压 |
| **其他** | 自由描述的其他性能问题 |

**功能特性**:

- 渐进式问题诊断 - 3轮对话逐步收集信息
- 智能决策树 - 症状→诊断→处方自动推荐
- 完整检查清单 - 14类 150+ 检查点
- 诊断工具推荐 - arthas, async-profiler, jstack 等
- 反模式警示 - 5个典型错误示例

## 贡献

欢迎提交 Issue 和 Pull Request 来添加新的 Skills！

## 许可证

[MIT License](LICENSE)

## 参考

- [Claude Agent Skills 官方文档](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview)
- [Skills Best Practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
