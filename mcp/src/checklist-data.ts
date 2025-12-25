/**
 * Checklist 数据 - 根据症状返回相关检查项
 */

// 详细检查项
export interface DetailedItem {
    desc: string;           // 检查项描述
    verify?: string;        // 验证命令
    threshold?: string;     // 告警阈值
    fix?: string;           // 快速修复建议
    why?: string;           // 原理说明（为什么会有这个问题）
    ref?: string;           // 延伸阅读链接
}

// 章节定义
export interface ChecklistSection {
    id: string;
    title: string;
    priority: 'P0' | 'P1' | 'P2';  // P0=紧急, P1=重要, P2=改进
    items: DetailedItem[];
}

// 向后兼容的简单接口
export interface ChecklistItem {
    id: string;
    title: string;
    items: string[];
}

export const CHECKLIST_DATA: Record<string, ChecklistSection> = {
    '0': {
        id: '0',
        title: '放大效应追踪',
        priority: 'P0',
        items: [
            { desc: '流量入口排查（Controller, MQ Listener, Schedule Job, WebSocket）', verify: 'arthas: trace *Controller *', threshold: 'QPS > 1000 关注', why: '入口是性能问题的放大器，1个入口的慢操作会被流量放大N倍' },
            { desc: '循环内 IO/计算（for/while/stream 内的 DB 查询、RPC）', verify: 'grep -n "for.*\\{" | 检查内部是否有 dao/rpc 调用', fix: '批量查询替代循环查询', why: '循环100次 x 每次10ms = 1秒，这是最常见的性能杀手' },
            { desc: '集合笛卡尔积（嵌套循环 O(N*M)）', verify: '搜索嵌套 for 循环', threshold: 'N*M > 10000 需优化', why: '时间复杂度爆炸：100x100=1万次，应该用 Map 降到 O(N+M)' },
            { desc: '广播风暴（单事件触发全量推送）', verify: '检查 @EventListener/@KafkaListener 处理逻辑', why: '1条消息触发推送给10万用户，瞬间产生10万次IO' },
            { desc: '频繁对象创建（循环内 new 对象）', verify: 'async-profiler -e alloc', fix: '对象池/复用', why: '频繁 new 导致 GC 压力，Young GC 频繁会影响吞吐' }
        ]
    },
    '1': {
        id: '1',
        title: '锁与并发',
        priority: 'P0',
        items: [
            { desc: '锁粒度过大（synchronized 方法或大代码块）', verify: 'jstack | grep -A 20 "BLOCKED"', fix: '细化锁粒度/读写锁', why: '大锁让并发变串行，N个线程只能1个执行，CPU利用率低' },
            { desc: '锁竞争（高频访问的共享资源）', verify: 'arthas: monitor -c 5 锁方法', threshold: '等待时间 > 100ms', why: '线程等锁时处于 BLOCKED 状态，无法执行任何工作' },
            { desc: '死锁风险（嵌套锁获取顺序不一致）', verify: 'jstack | grep "deadlock"', why: '线程A持有锁1等锁2，线程B持有锁2等锁1，永远等待' },
            { desc: 'CAS 自旋（Atomic 的 do-while 无退避）', verify: '搜索 AtomicInteger/AtomicLong 使用处', fix: 'LongAdder 替代', why: '高竞争下 CAS 频繁失败重试，CPU 空转浪费' }
        ]
    },
    '2': {
        id: '2',
        title: 'IO 与阻塞',
        priority: 'P0',
        items: [
            { desc: '同步 IO（NIO/Netty 线程中混入阻塞操作）', verify: '检查 EventLoop 线程内是否有 JDBC/File IO', why: 'EventLoop 线程被阻塞后，该线程上的所有连接都无法处理' },
            { desc: '长耗时逻辑（Controller 入口未异步化）', verify: 'arthas: trace 入口方法', threshold: '耗时 > 500ms 需异步', why: '一个线程被长操作占用，线程池有效并发度下降' },
            { desc: '资源未关闭（InputStream/Connection 未 close）', verify: 'lsof -p PID | wc -l', threshold: '句柄 > 10000 告警', fix: 'try-with-resources', why: '资源泄露导致句柄耗尽，新连接无法建立' }
        ]
    },
    '3': {
        id: '3',
        title: '外部调用',
        priority: 'P1',
        items: [
            { desc: '无超时设置（HTTPClient, Dubbo, DB 连接）', verify: '搜索 timeout/connectTimeout 配置', fix: '统一配置超时 3-5s', why: '无超时的请求可能永久等待，占用线程资源' },
            { desc: '重试风暴（无 Backoff 和 Jitter）', verify: '检查 @Retry/@Retryable 配置', why: '同时重试导致下游服务雪崩，需指数退避+随机抨动' },
            { desc: '同步串行调用（多下游串行）', verify: 'arthas: trace 检查调用链', fix: 'CompletableFuture 并行', why: '串行调用 A+B+C = 300ms，并行 = max(A,B,C) = 100ms' }
        ]
    },
    '4': {
        id: '4',
        title: '资源池管理',
        priority: 'P0',
        items: [
            { desc: '无界线程池（Executors.newCachedThreadPool）', verify: 'arthas: thread -n 10', threshold: '线程 > 200 告警', fix: 'ThreadPoolExecutor 有界', why: '无界池遇到流量洪峰无限创建线程，耗尽系统资源后 OOM' },
            { desc: '池资源泄露（获取后未归还）', verify: 'jstack | grep pool', fix: 'finally 归还', why: '每次请求泄露1个连接，池很快被占满，新请求永远等待' },
            { desc: '连接数配置不当', verify: 'show processlist (MySQL)', threshold: '活跃连接 > 80% 池大小', why: '池太小导致排队等待，池太大导致数据库压力和上下文切换' }
        ]
    },
    '5': {
        id: '5',
        title: '内存与缓存',
        priority: 'P0',
        items: [
            { desc: '无界缓存（static Map 无 TTL/Size 限制）', verify: 'jmap -histo:live | head -20', fix: 'Caffeine/Guava Cache', why: '只增不删的缓存是内存泄露，最终导致 OOM' },
            { desc: '大对象分配（一次性加载大文件/全量表）', verify: 'MAT 分析 Dominator Tree', threshold: '单对象 > 10MB 关注', why: '大对象直接进入老年代，触发 Full GC 导致长时间停顿' },
            { desc: 'ThreadLocal 泄露（请求结束未 remove）', verify: '搜索 ThreadLocal 未配对 remove()', fix: 'finally 中 remove()', why: '线程池复用线程，ThreadLocal 不清理导致内存累积和业务数据混乱' }
        ]
    },
    '6': {
        id: '6',
        title: '异常处理',
        priority: 'P2',
        items: [
            { desc: '异常吞没（catch 后仅打印）', verify: '搜索 catch.*\\{.*e.printStackTrace', why: '异常被吞掉导致问题难以追溯和修复' },
            { desc: '异常日志爆炸（高频打印完整堆栈）', verify: '日志文件大小增长速率', threshold: '日志 > 1GB/天 关注', why: '频繁打印堆栈消耗 CPU 和磁盘 IO' },
            { desc: '异常控制流程（用异常做业务控制）', verify: '搜索 catch 中的业务逻辑', why: '异常开销大（栈堆栈捕获），不应用于正常流程' }
        ]
    },
    '10': {
        id: '10',
        title: '正则表达式',
        priority: 'P1',
        items: [
            { desc: 'Catastrophic Backtracking（嵌套量词如 (a+)+）', verify: '搜索 Pattern.compile，检查正则复杂度', why: '恶意输入可触发指数级回溯，单次匹配耗时可达分钟' },
            { desc: '反复编译（循环内 Pattern.compile）', verify: '搜索 Pattern.compile 出现位置', fix: '静态常量预编译', why: '正则编译开销大，循环 1000 次 = 1000 次编译开销' }
        ]
    },
    '11': {
        id: '11',
        title: '响应式编程',
        priority: 'P1',
        items: [
            { desc: '阻塞操作（Mono/Flux 中有阻塞调用）', verify: '搜索 .block()/.toFuture().get()', fix: 'subscribeOn(Schedulers.boundedElastic())', why: '响应式线程池很小，阻塞会卡死整个应用' },
            { desc: '背压丢失（无法处理背压的操作符）', verify: '检查 onBackpressure 策略', why: '不处理背压会导致内存溢出或数据丢失' }
        ]
    },
    '12': {
        id: '12',
        title: '定时任务',
        priority: 'P1',
        items: [
            { desc: '任务堆积（执行时间超过调度间隔）', verify: '日志检查任务开始/结束时间', threshold: '执行时间 > 间隔时间', why: '任务越积越多，最终耗尽线程和内存' },
            { desc: '异常中断（未捕获异常导致调度停止）', verify: '检查 @Scheduled 方法的异常处理', fix: 'try-catch 包裹', why: '未捕获异常会导致定时任务永久停止' }
        ]
    },
    '13': {
        id: '13',
        title: '数据库',
        priority: 'P0',
        items: [
            { desc: 'N+1 查询（循环内单条查询）', verify: '开启 SQL 日志，观察重复 SQL', fix: 'IN 批量查询', why: '循环 100 次 = 100 次网络往返，批量查询只需 1 次' },
            { desc: '全表扫描（无索引或索引失效）', verify: 'EXPLAIN SELECT ...', threshold: 'type=ALL 需优化', why: '全表扫描时间复杂度 O(N)，索引是 O(logN)' },
            { desc: '深度分页（OFFSET 过大）', verify: '搜索 LIMIT.*OFFSET', fix: 'WHERE id > lastId', why: 'OFFSET 10000 需跳过 1 万行，游标分页直接定位' },
            { desc: '事务过长（事务内包含 RPC）', verify: '检查 @Transactional 方法内容', fix: '事务拆分', why: '长事务持有连接和锁，影响并发' },
            { desc: '锁表操作（大批量 UPDATE）', verify: 'show processlist', fix: '分批处理', why: '一次更新 10 万行会锁表，分批 1000 行可避免' }
        ]
    },
    '14': {
        id: '14',
        title: 'Java 特定',
        priority: 'P2',
        items: [
            { desc: 'Stream 滥用（短集合用 Stream）', verify: 'async-profiler 热点分析', threshold: '集合 < 10 用 for', fix: 'for 循环替代', why: 'Stream 创建中间对象开销大，小集合不值得' },
            { desc: 'BigDecimal 重复创建', verify: '搜索 new BigDecimal', fix: 'BigDecimal.ZERO/ONE', why: '重复创建常用值浪费内存' },
            { desc: '字符串拼接（循环内 + 拼接）', verify: '搜索循环内字符串 +', fix: 'StringBuilder', why: '每次 + 创建新对象，循环 N 次 = N 个临时对象' },
            { desc: '反射调用（高频路径未缓存 Method）', verify: '搜索 getMethod/invoke', fix: '缓存 Method 对象', why: '反射每次查找方法开销大，缓存后快 10 倍' },
            { desc: '装箱拆箱（Integer/Long 频繁自动装箱）', verify: 'async-profiler -e alloc', fix: '原始类型', why: '装箱创建对象，拆箱调用方法，在循环中开销明显' }
        ]
    },
    '15': {
        id: '15',
        title: 'Spring 框架',
        priority: 'P1',
        items: [
            { desc: '@Async 默认线程池', verify: '检查 TaskExecutor 配置', fix: '自定义 ThreadPoolTaskExecutor', why: '默认线程池无界，高并发下任务堆积OOM' },
            { desc: '@Transactional 传播问题', verify: '检查嵌套事务配置', why: '传播属性配置错误导致事务未生效或意外回滚' },
            { desc: 'AOP 代理失效（同类方法调用）', verify: '检查 this.method() 调用', fix: 'AopContext.currentProxy()', why: 'this 调用绕过代理，事务/缓存注解失效' },
            { desc: 'Bean 循环依赖', verify: '启动日志检查 circular reference', fix: '@Lazy 注解', why: '循环依赖导致启动慢或失败' },
            { desc: '@Scheduled 单线程', verify: '检查 SchedulingConfigurer', fix: '配置线程池', why: '默认单线程，一个慢任务阻塞所有定时任务' }
        ]
    },
    '16': {
        id: '16',
        title: 'Dubbo/RPC',
        priority: 'P1',
        items: [
            { desc: '超时设置不当', verify: '检查 dubbo:reference timeout', fix: 'provider > consumer', why: 'consumer 超时短于 provider 导致重复请求' },
            { desc: '序列化开销', verify: '检查传输对象大小', threshold: '> 1MB 需优化', why: '大对象序列化耗 CPU，传输耗带宽' },
            { desc: '线程池满', verify: 'arthas: thread | grep dubbo', threshold: '活跃 > 80% 告警', why: '线程池满导致新请求被拒绝' },
            { desc: '重试风暴', verify: '检查 retries 配置', fix: '幂等接口才重试', why: '非幂等接口重试导致数据重复' },
            { desc: '熔断缺失', verify: '检查 Sentinel/Hystrix 配置', why: '无熔断时下游故障会拖垮上游' }
        ]
    },
    '17': {
        id: '17',
        title: 'MyBatis/ORM',
        priority: 'P1',
        items: [
            { desc: '一级缓存坑（同 SqlSession 内脏读）', verify: '检查事务边界', why: '同事务内读取到未提交的修改' },
            { desc: '懒加载 N+1', verify: '开启 SQL 日志', fix: 'fetchType=eager 或 JOIN', why: '访问关联对象触发额外 SQL' },
            { desc: '批量插入未优化', verify: '搜索循环 insert', fix: 'foreach batch 插入', why: '循环 insert 每次建连，batch 一次搞定' },
            { desc: '动态 SQL 过长', verify: '检查 foreach 元素数量', threshold: '> 1000 需分批', why: 'SQL 太长导致解析慢或超限' },
            { desc: 'ResultMap 映射开销', verify: '检查复杂嵌套映射', why: '复杂映射反射开销大' }
        ]
    },
    '18': {
        id: '18',
        title: '放大效应进阶',
        priority: 'P0',
        items: [
            { desc: '惊群效应（缓存失效时 N 线程同时查库）', verify: '搜索 cache.get 后直接 db.query，无锁保护', fix: 'Mutex/Singleflight 或分布式锁', why: '1000 并发 x 缓存失效 = 1000 次 DB 查询，应该只允许 1 个线程查' },
            { desc: '扇出放大（1 请求调 N 个下游）', verify: '统计单接口内 RPC/HTTP 调用数', threshold: '扇出 > 5 需关注', fix: '并行调用 + 超时控制', why: '串行调用 10 个下游各 100ms = 1s，并行只需 100ms' },
            { desc: '排队放大（任务堆积导致等待时间 > 处理时间）', verify: 'arthas: thread 检查线程池队列大小', threshold: '队列 > 100 需关注', why: '处理 10ms 但排队 1s，用户感知是 1.01s' },
            { desc: '热点 Key 放大（分片不均导致单点压力）', verify: '检查 Redis/DB 分片 key 分布', fix: '加随机后缀分散', why: '100 万请求打到同一分片，该分片成为瓶颈' },
            { desc: '超时放大（超时配置过长占用资源）', verify: '搜索 timeout 配置 > 10s', fix: '超时 3-5s，快速失败', why: '超时 30s = 线程被占 30s，10 个慢请求耗尽线程池' },
            { desc: '连接放大（每请求新建连接）', verify: '搜索 new HttpClient/new Connection', fix: '使用连接池', why: 'TCP 握手 + TLS 握手 = 100-500ms，连接池复用只需 1ms' }
        ]
    },
    '19': {
        id: '19',
        title: '级联故障防护',
        priority: 'P0',
        items: [
            { desc: '舱壁隔离缺失（核心与非核心共用线程池）', verify: '检查是否所有业务用同一个线程池', fix: '按业务域隔离线程池', why: '非核心慢接口拖垮线程池 → 核心接口也无法处理' },
            { desc: '过载保护缺失（无 Load Shedding）', verify: '检查是否有 CPU/Memory 阈值保护', fix: 'Sentinel/自定义过载保护', why: '系统满载还接受请求 → 雪崩' },
            { desc: '入口限流缺失（无 QPS 限制）', verify: '搜索 @RateLimiter/Sentinel 配置', fix: 'Guava RateLimiter/Sentinel', why: '突发流量直接打到后端 → 压垮系统' },
            { desc: '快速失败缺失（超时不中断）', verify: '检查 Future.get 是否有超时', fix: 'get(timeout) + cancel(true)', why: '下游超时但任务不中断 → 资源持续被占用' },
            { desc: '熔断缺失（下游故障持续调用）', verify: '检查 Hystrix/Resilience4j/Sentinel 配置', fix: '配置熔断器', why: '下游挂了还持续调用 → 放大故障 + 阻塞调用方' },
            { desc: 'DNS 缓存缺失（每次请求 DNS 解析）', verify: '检查 JVM DNS 缓存配置', fix: 'networkaddress.cache.ttl=60', why: '每次 DNS 解析 10-100ms，缓存后 0ms' }
        ]
    }
};

// 症状到章节的映射
export const SYMPTOM_TO_SECTIONS: Record<string, string[]> = {
    'memory': ['0', '5', '6', '14', '18'],
    'cpu': ['0', '1', '10', '14', '18'],
    'slow': ['2', '3', '1', '13', '15', '16', '17', '18', '19'],
    'resource': ['4', '5', '15', '16', '18', '19'],
    'backlog': ['0', '11', '12', '18'],
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

