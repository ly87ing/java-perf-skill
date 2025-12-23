# Claude Skill: Performance Troubleshoot

<p align="center">
  <img src="https://img.shields.io/badge/Claude-Skill-blue" alt="Claude Skill">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="MIT License">
  <img src="https://img.shields.io/badge/Version-1.0.0-orange" alt="Version">
</p>

一个用于分析和解决性能与资源问题的 Claude Skill，包含自动化的多轮审查，确保方案安全、正确、健壮。

## ✨ 功能特性

- 🔍 **渐进式问题诊断** - 3轮对话逐步收集信息
- 🌳 **智能决策树** - 症状→诊断→处方自动推荐
- 📋 **完整检查清单** - 14类 150+ 检查点
- 🛠️ **诊断工具推荐** - arthas, async-profiler, jstack 等
- ❌ **反模式警示** - 5个典型错误示例
- 📊 **输出格式规范** - 7项强制要求，图文并茂

## 🚀 快速开始

### 安装

1. 克隆仓库到本地：
```bash
git clone https://github.com/ly87ing/claude-skill-performance-troubleshoot.git
```

2. 将 Skill 复制到您的项目中：
```bash
cp -r claude-skill-performance-troubleshoot/.agent/skills/performance-troubleshoot your-project/.agent/skills/
```

### 使用

在与 Claude 对话时，只需描述您的性能问题：

```
请帮我排查内存暴涨问题，从 3GB 涨到 16GB...

系统响应很慢，CPU 使用率很高...

消息队列出现大量积压...
```

Claude 会自动触发此 Skill 并引导您完成问题分析。

## 📁 文件结构

```
performance-troubleshoot/
├── SKILL.md        # 主文件 - 诊断流程和优化模式
├── CHECKLIST.md    # 审查检查清单 - 150+ 检查点
└── TEMPLATE.md     # 文档模板 - 输出格式规范
```

## 🎯 适用场景

| 问题类型 | 示例 |
|----------|------|
| **内存问题** | 内存暴涨、OOM、GC 频繁 |
| **性能问题** | 响应慢、CPU 高、吞吐低 |
| **并发问题** | 死锁、竞态条件、线程池满 |
| **稳定性问题** | 超时、错误率高、服务不可用 |
| **消息问题** | 消息积压、消费慢 |

## 📊 优化模式

Skill 包含 7 类 40+ 优化模式：

1. **性能优化** - 请求合并、结果缓存、批量处理
2. **锁竞争优化** - 锁分段、读写锁、无锁设计
3. **故障处理** - 熔断器、重试、降级
4. **流量控制** - 限流、背压、负载均衡
5. **Actor 模式** - 消息传递、监督策略
6. **长连接管理** - 心跳、重连、广播优化
7. **资源突增防护** - 对象池、预分配

## 🛠️ 诊断工具

| 问题类型 | 推荐工具 |
|----------|----------|
| 内存问题 | jmap, MAT, VisualVM |
| CPU 问题 | async-profiler, arthas |
| 线程问题 | jstack, arthas |
| GC 问题 | GCViewer, GCEasy |

## 📝 输出示例

使用此 Skill 后，您将获得包含以下内容的完整方案：

1. ✅ 问题分析图 (Mermaid)
2. ✅ 解决方案表格
3. ✅ 方案设计图
4. ✅ 完整代码实现
5. ✅ 预期效果 (量化)
6. ✅ 验证方法

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

[MIT License](LICENSE)

## 🙏 鸣谢

- [Anthropic Claude](https://anthropic.com) - AI 平台
- [Claude Skills](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview) - 技能框架
