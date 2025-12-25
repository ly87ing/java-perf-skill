/**
 * Checklist 数据 - 根据症状返回相关检查项
 */

export interface ChecklistItem {
    id: string;
    title: string;
    items: string[];
}

export const CHECKLIST_DATA: Record<string, ChecklistItem> = {
    '0': {
        id: '0',
        title: '放大效应追踪',
        items: [
            '流量入口排查（Controller, MQ Listener, Schedule Job, WebSocket）',
            '循环内 IO/计算（for/while/stream 内的 DB 查询、RPC、复杂计算）',
            '集合笛卡尔积（嵌套循环 O(N*M)）',
            '广播风暴（单事件触发全量推送）',
            '频繁对象创建（循环内 new 对象、stream.collect）'
        ]
    },
    '1': {
        id: '1',
        title: '锁与并发',
        items: [
            '锁粒度过大（synchronized 方法或大代码块）',
            '锁竞争（高频访问的共享资源）',
            '死锁风险（嵌套锁获取顺序不一致）',
            'CAS 自旋（Atomic 的 do-while 无退避）'
        ]
    },
    '2': {
        id: '2',
        title: 'IO 与阻塞',
        items: [
            '同步 IO（NIO/Netty 线程中混入 JDBC/File IO/同步 HTTP）',
            '长耗时逻辑（Controller 入口未异步化的耗时操作）',
            '资源未关闭（InputStream/Connection 未在 finally 或 try-with-resources 关闭）'
        ]
    },
    '3': {
        id: '3',
        title: '外部调用',
        items: [
            '无超时设置（HTTPClient, Dubbo, DB 连接）',
            '重试风暴（无 Backoff 和 Jitter）',
            '同步串行调用（多下游串行，可改 CompletableFuture 并行）'
        ]
    },
    '4': {
        id: '4',
        title: '资源池管理',
        items: [
            '无界线程池（Executors.newCachedThreadPool）',
            '池资源泄露（获取后未归还）',
            '连接数配置不当（过小等待/过大切换）'
        ]
    },
    '5': {
        id: '5',
        title: '内存与缓存',
        items: [
            '无界缓存（static Map 无 TTL/Size 限制，只增不删）',
            '大对象分配（一次性加载大文件/全量表）',
            'ThreadLocal 泄露（请求结束未 remove()）'
        ]
    },
    '6': {
        id: '6',
        title: '异常处理',
        items: [
            '异常吞没（catch 后仅打印，未抛出/处理）',
            '异常日志爆炸（高频错误路径打印完整堆栈）',
            '异常控制流程（用异常做正常业务流程控制）'
        ]
    },
    '10': {
        id: '10',
        title: '正则表达式',
        items: [
            'Catastrophic Backtracking（嵌套量词 (a+)+ 指数回溯）',
            '反复编译（Pattern.compile 在循环/高频方法中被反复调用）'
        ]
    },
    '11': {
        id: '11',
        title: '响应式编程',
        items: [
            '阻塞操作（map/flatMap 中有 JDBC/RPC 阻塞）',
            '背压丢失（无法处理背压的操作符）'
        ]
    },
    '12': {
        id: '12',
        title: '定时任务',
        items: [
            '任务堆积（执行时间超过调度间隔）',
            '异常中断（未捕获异常导致调度停止）'
        ]
    },
    '13': {
        id: '13',
        title: '数据库',
        items: [
            'N+1 查询（循环内单条查询，应批量 IN 查询）',
            '全表扫描（无索引或索引失效）',
            '深度分页（OFFSET 过大，应改用游标分页）',
            '事务过长（事务内包含 RPC/文件操作）',
            '锁表操作（DDL/大批量 UPDATE 未分批）'
        ]
    },
    '14': {
        id: '14',
        title: 'Java 特定',
        items: [
            'Stream 滥用（短集合用 Stream 增加开销）',
            'BigDecimal 重复创建（应用 BigDecimal.ZERO/ONE 常量）',
            '字符串拼接（循环内 + 拼接，应用 StringBuilder）',
            '反射调用（高频路径使用反射未缓存 Method）',
            '装箱拆箱（Integer/Long 频繁自动装箱）'
        ]
    },
    '15': {
        id: '15',
        title: 'Spring 框架',
        items: [
            '@Async 默认线程池（未配置自定义线程池导致任务堆积）',
            '@Transactional 传播问题（嵌套事务配置不当）',
            'AOP 代理失效（同类方法调用绕过代理）',
            'Bean 循环依赖（延迟初始化导致启动慢）',
            '@Scheduled 单线程（默认单线程导致任务阻塞）'
        ]
    },
    '16': {
        id: '16',
        title: 'Dubbo/RPC',
        items: [
            '超时设置不当（provider/consumer 超时配置冲突）',
            '序列化开销（复杂对象未优化序列化）',
            '线程池满（provider 线程池配置过小）',
            '重试风暴（未配置合理重试策略）',
            '熔断缺失（未使用 Sentinel/Hystrix 限流）'
        ]
    },
    '17': {
        id: '17',
        title: 'MyBatis/ORM',
        items: [
            '一级缓存坑（同 SqlSession 内脏读）',
            '懒加载 N+1（关联查询触发多次 SQL）',
            '批量插入未优化（逐条 insert 应改 batch）',
            '动态 SQL 拼接（foreach 过长导致 SQL 爆炸）',
            'ResultMap 映射开销（复杂嵌套映射性能差）'
        ]
    }
};

// 症状到章节的映射
export const SYMPTOM_TO_SECTIONS: Record<string, string[]> = {
    'memory': ['0', '5', '6', '14'],
    'cpu': ['0', '1', '10', '14'],
    'slow': ['2', '3', '1', '13', '15', '16', '17'],
    'resource': ['4', '5', '15', '16'],
    'backlog': ['0', '11', '12'],
    'gc': ['5', '0', '14']
};

// 症状组合诊断
export const SYMPTOM_COMBINATIONS: Record<string, { diagnosis: string, rootCauses: Array<{ cause: string, probability: number }> }> = {
    'cpu+slow': {
        diagnosis: '锁竞争或计算密集导致 CPU 高同时响应慢',
        rootCauses: [
            { cause: '锁竞争（synchronized/ReentrantLock）', probability: 60 },
            { cause: '正则表达式回溯', probability: 20 },
            { cause: 'CAS 自旋', probability: 15 },
            { cause: '复杂算法', probability: 5 }
        ]
    },
    'cpu+gc': {
        diagnosis: '对象创建过快导致 GC 频繁和 CPU 消耗',
        rootCauses: [
            { cause: '循环内创建大量对象', probability: 50 },
            { cause: 'Stream 滥用', probability: 25 },
            { cause: '字符串拼接', probability: 20 },
            { cause: '装箱拆箱', probability: 5 }
        ]
    },
    'slow+memory': {
        diagnosis: 'GC 暂停或大对象操作导致响应慢和内存高',
        rootCauses: [
            { cause: '大对象分配（全量加载）', probability: 45 },
            { cause: '无界缓存', probability: 30 },
            { cause: 'Full GC 暂停', probability: 20 },
            { cause: '内存泄露', probability: 5 }
        ]
    },
    'slow+resource': {
        diagnosis: '资源池耗尽导致请求等待',
        rootCauses: [
            { cause: '连接池满', probability: 40 },
            { cause: '线程池满', probability: 35 },
            { cause: '下游服务慢', probability: 20 },
            { cause: '资源泄露', probability: 5 }
        ]
    },
    'memory+gc': {
        diagnosis: '内存压力导致频繁 GC',
        rootCauses: [
            { cause: '对象创建风暴', probability: 40 },
            { cause: '内存泄露', probability: 30 },
            { cause: '缓存未限制大小', probability: 25 },
            { cause: 'ThreadLocal 未清理', probability: 5 }
        ]
    },
    'backlog+slow': {
        diagnosis: '消费能力不足导致积压和延迟',
        rootCauses: [
            { cause: '消费者处理慢', probability: 50 },
            { cause: '消费者内有阻塞调用', probability: 30 },
            { cause: '并发度不足', probability: 15 },
            { cause: '消息体过大', probability: 5 }
        ]
    }
};

// 快速诊断表
export const QUICK_DIAGNOSIS: Record<string, { causes: string[], patterns: string[], tools: string[] }> = {
    'memory': {
        causes: ['对象创建风暴', '资源泄露', '无界缓存'],
        patterns: ['对象池', '生命周期管理', 'TTL/Size 限制'],
        tools: ['jmap -histo:live', 'MAT (Memory Analyzer)', 'async-profiler -e alloc']
    },
    'cpu': {
        causes: ['死循环', '正则回溯', '锁竞争', 'CAS 自旋'],
        patterns: ['算法优化', '锁分段', '退避机制'],
        tools: ['async-profiler -e cpu', 'perf top', 'arthas profiler']
    },
    'slow': {
        causes: ['IO阻塞', '锁竞争', '下游慢', '串行调用'],
        patterns: ['异步化', '熔断', '缓存', '并行调用'],
        tools: ['arthas trace', 'Jaeger/Zipkin', 'async-profiler -e wall']
    },
    'resource': {
        causes: ['连接池/线程池满', '句柄泄露', '无界队列'],
        patterns: ['资源复用', '背压', '有界队列'],
        tools: ['jstack', 'lsof -p', 'arthas thread']
    },
    'backlog': {
        causes: ['消费慢', '突发流量', '处理能力不足'],
        patterns: ['批量消费', '并行消费', '限流'],
        tools: ['MQ 控制台', 'arthas watch', 'Prometheus metrics']
    },
    'gc': {
        causes: ['对象分配速率高', '大对象', '内存泄露'],
        patterns: ['减少对象创建', '对象复用', '堆外内存'],
        tools: ['jstat -gcutil', 'GC 日志分析', 'async-profiler -e alloc']
    }
};

// 反模式速查
export const ANTI_PATTERNS = [
    { name: '锁内IO', bad: 'synchronized { httpClient.get() }', good: '锁外获取，锁内只写' },
    { name: '循环创建对象', bad: 'for() { new StringBuilder() }', good: '复用对象' },
    { name: '无界队列', bad: 'Executors.newFixedThreadPool', good: '有界队列 + 拒绝策略' },
    { name: '缓存穿透', bad: 'if (cache==null) db.query()', good: '加锁防穿透' },
    { name: 'N+1 查询', bad: 'for(u:users) dao.get(u.id)', good: '批量查询 IN (ids)' },
    { name: '消息重复消费', bad: '无幂等处理', good: '幂等 key + 去重表' },
    { name: '消费者阻塞', bad: 'consumer 内同步 RPC', good: '异步处理 + 本地队列' },
    { name: 'Stream 短集合', bad: 'list.stream().filter().collect()', good: 'for 循环（<10 元素）' },
    { name: '深度分页', bad: 'LIMIT 10 OFFSET 10000', good: 'WHERE id > lastId LIMIT 10' }
];

// 报告模板
export const REPORT_TEMPLATE = `# 性能问题诊断报告

> **日期**: YYYY-MM-DD  
> **项目**: [项目名称]  
> **症状**: [内存/CPU/响应慢/资源耗尽/消息积压]

---

## 一、问题总览

| 优先级 | 问题 | 位置 | 影响 |
|--------|------|------|------|
| 🔴 P0 | [问题描述] | \`File.java:123\` | [影响描述] |
| 🟠 P1 | [问题描述] | \`File.java:456\` | [影响描述] |

---

## 二、问题详情与解决方案

### 问题 1: [问题名称]

**位置**: \`path/to/File.java:123\`  
**放大倍数**: N × M = 总放大

#### 问题代码
\`\`\`java
// 问题描述
[问题代码]
\`\`\`

#### 解决方案
\`\`\`java
// ✅ 优化后
[优化代码]
\`\`\`

**预期效果**: [量化描述]

---

## 三、行动清单

- [ ] **P0 紧急**: [具体修复操作]
- [ ] **P1 重要**: [具体修复操作]
`;

