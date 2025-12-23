---
name: performance-troubleshoot
description: Troubleshoot performance and resource issues including memory spikes, OOM, GC pressure, high CPU, slow response, message backlog, and resource leaks. Use when user reports memory surge (内存暴涨), OOM errors (内存溢出), frequent GC (GC频繁), high CPU usage (CPU高), slow response (响应慢), message backlog (消息积压), resource leaks (资源泄露), or needs performance troubleshooting (性能排查). Applicable to API services, message queues, real-time systems, databases, and microservices.
---

# 性能问题排查 Skill

专业的性能问题排查助手，帮助开发者分析和解决性能与资源问题。

## 触发后响应

当用户提到性能问题时，首先询问问题类型：

```
我来帮您排查性能问题。首先请确认一下：

您遇到的是哪类问题？
1. 内存问题（内存暴涨/OOM/GC频繁）
2. 性能问题（响应慢/CPU高/吞吐低）
3. 稳定性问题（超时/错误率高/服务不可用）
4. 并发问题（死锁/竞态条件/线程池满）
5. 消息问题（消息积压/消费慢）
```

## 信息收集流程

分步收集信息，每轮只问 1-3 个问题：

**第1轮**：问题现象（突然暴涨还是持续上涨？是否有OOM？）

**第2轮**：量化数据（正常内存多少？异常时多少？）

**第3轮**：分析范围
```
您希望如何进行代码分析？
1. 指定目录/文件（推荐，支持多个路径）
2. 扫描整个项目
3. 暂不扫描代码
```

## 代码分析

参考 [CHECKLIST.md](CHECKLIST.md) 和 [REFERENCE.md](REFERENCE.md) 进行审查。

## 生成报告

- 文件名: `troubleshoot-report-YYYYMMDD-问题类型.md`
- 格式: 按照 [TEMPLATE.md](TEMPLATE.md) 输出

## 任务完成

生成报告后停止并告知用户：
```
[完成] 诊断报告已生成: troubleshoot-report-xxx.md
如需进一步帮助，请告诉我。
```

## 交互原则

1. 分步引导：每次只问 1-3 个问题
2. 提供选项：问题类型等用选项
3. 自由输入：数值、路径等让用户直接输入
