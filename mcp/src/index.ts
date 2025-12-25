#!/usr/bin/env node

/**
 * Performance Troubleshoot MCP Server
 * 
 * 提供 Java 性能问题排查的 MCP 工具
 */

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';
import {
    CHECKLIST_DATA,
    SYMPTOM_TO_SECTIONS,
    QUICK_DIAGNOSIS,
    ANTI_PATTERNS,
    REPORT_TEMPLATE,
    SYMPTOM_COMBINATIONS,
    type ChecklistItem
} from './checklist-data.js';

// ========== 常量定义 ==========
const VALID_SYMPTOMS = ['memory', 'cpu', 'slow', 'resource', 'backlog', 'gc'] as const;
const SYMPTOM_DESCRIPTIONS = {
    memory: '内存暴涨/OOM',
    cpu: 'CPU使用率高',
    slow: '响应慢/超时',
    resource: '资源耗尽(连接池/线程池)',
    backlog: '消息积压',
    gc: 'GC频繁'
} as const;

// ========== 服务器初始化 ==========
const server = new McpServer({
    name: 'java-perf',
    version: '1.0.0'
});

// ========== 工具定义 ==========

/**
 * 工具 1: get_checklist
 * 根据症状返回相关的检查项列表
 */
server.tool(
    'get_checklist',
    {
        symptoms: z.array(z.enum(VALID_SYMPTOMS))
            .min(1, '请至少提供一个症状')
            .max(6, '症状数量不能超过6个')
            .describe(`症状列表。可选值: ${Object.entries(SYMPTOM_DESCRIPTIONS).map(([k, v]) => `${k}(${v})`).join(', ')}`),

        includeDetails: z.boolean()
            .default(true)
            .describe('是否返回详细信息（验证命令、阈值等），默认 true'),

        priorityFilter: z.enum(['all', 'P0', 'P1', 'P2'])
            .default('all')
            .describe('按优先级过滤: all(全部), P0(紧急), P1(重要), P2(改进)')
    },
    async ({ symptoms, includeDetails, priorityFilter }) => {
        // 收集相关章节
        const sectionIds = new Set<string>();
        for (const symptom of symptoms) {
            const sections = SYMPTOM_TO_SECTIONS[symptom] || [];
            sections.forEach(id => sectionIds.add(id));
        }

        // 构建检查项列表
        const checklist: Array<{
            id: string;
            title: string;
            priority: string;
            itemCount: number;
            items?: Array<{
                desc: string;
                verify?: string;
                threshold?: string;
                fix?: string;
            }>;
        }> = [];

        for (const id of sectionIds) {
            const section = CHECKLIST_DATA[id];
            if (section) {
                // 优先级过滤
                if (priorityFilter !== 'all' && section.priority !== priorityFilter) {
                    continue;
                }

                checklist.push({
                    id: section.id,
                    title: section.title,
                    priority: section.priority,
                    itemCount: section.items.length,
                    ...(includeDetails ? { items: section.items } : {})
                });
            }
        }

        // 按优先级排序: P0 > P1 > P2
        checklist.sort((a, b) => {
            const order = { 'P0': 0, 'P1': 1, 'P2': 2 };
            return (order[a.priority as keyof typeof order] || 99) - (order[b.priority as keyof typeof order] || 99);
        });

        return {
            content: [{
                type: 'text' as const,
                text: JSON.stringify({
                    success: true,
                    query: { symptoms, includeDetails, priorityFilter },
                    data: {
                        sectionCount: checklist.length,
                        totalItems: checklist.reduce((sum, s) => sum + s.itemCount, 0),
                        prioritySummary: {
                            P0: checklist.filter(c => c.priority === 'P0').length,
                            P1: checklist.filter(c => c.priority === 'P1').length,
                            P2: checklist.filter(c => c.priority === 'P2').length
                        },
                        checklist
                    }
                }, null, 2)
            }]
        };
    }
);

/**
 * 工具 2: get_diagnosis
 * 获取症状的快速诊断参考
 */
server.tool(
    'get_diagnosis',
    {
        symptom: z.enum(['memory', 'cpu', 'slow', 'resource', 'backlog', 'gc'])
            .describe('单个症状类型'),

        includeAntiPatterns: z.boolean()
            .default(true)
            .describe('是否包含相关反模式示例')
    },
    async ({ symptom, includeAntiPatterns }) => {
        const diagnosis = QUICK_DIAGNOSIS[symptom];

        if (!diagnosis) {
            return {
                content: [{
                    type: 'text' as const,
                    text: JSON.stringify({
                        success: false,
                        error: `未知症状: ${symptom}`,
                        validSymptoms: Object.keys(QUICK_DIAGNOSIS)
                    }, null, 2)
                }]
            };
        }

        // 获取相关反模式
        const antiPatternMap: Record<string, string[]> = {
            memory: ['循环创建对象', '无界队列'],
            cpu: ['锁内IO'],
            slow: ['锁内IO', 'N+1 查询', '深度分页'],
            resource: ['无界队列', '缓存穿透'],
            backlog: ['消息重复消费', '消费者阻塞'],
            gc: ['循环创建对象', 'Stream 短集合']
        };

        const relevantPatterns = includeAntiPatterns
            ? ANTI_PATTERNS.filter(p => antiPatternMap[symptom]?.includes(p.name))
            : [];

        return {
            content: [{
                type: 'text' as const,
                text: JSON.stringify({
                    success: true,
                    query: { symptom, includeAntiPatterns },
                    data: {
                        symptom,
                        symptomDescription: SYMPTOM_DESCRIPTIONS[symptom as keyof typeof SYMPTOM_DESCRIPTIONS] || symptom,
                        possibleCauses: diagnosis.causes,
                        recommendedPatterns: diagnosis.patterns,
                        diagnosticTools: diagnosis.tools,
                        relatedSections: SYMPTOM_TO_SECTIONS[symptom] || [],
                        ...(includeAntiPatterns && relevantPatterns.length > 0
                            ? { antiPatterns: relevantPatterns }
                            : {})
                    }
                }, null, 2)
            }]
        };
    }
);

/**
 * 工具 2.5: get_combined_diagnosis
 * 多症状组合诊断
 */
server.tool(
    'get_combined_diagnosis',
    {
        symptoms: z.array(z.enum(['memory', 'cpu', 'slow', 'resource', 'backlog', 'gc']))
            .min(2, '请提供至少2个症状进行组合诊断')
            .max(3, '最多支持3个症状组合')
            .describe('症状列表（2-3个）')
    },
    async ({ symptoms }) => {
        // 生成所有可能的症状对组合
        const pairs: string[] = [];
        for (let i = 0; i < symptoms.length; i++) {
            for (let j = i + 1; j < symptoms.length; j++) {
                const sorted = [symptoms[i], symptoms[j]].sort();
                pairs.push(sorted.join('+'));
            }
        }

        // 查找匹配的组合诊断
        const matchedDiagnoses = pairs
            .map(pair => {
                const diagnosis = SYMPTOM_COMBINATIONS[pair];
                return diagnosis ? { combination: pair, ...diagnosis } : null;
            })
            .filter(Boolean);

        // 合并相关章节
        const allSections = new Set<string>();
        symptoms.forEach(s => {
            (SYMPTOM_TO_SECTIONS[s] || []).forEach(id => allSections.add(id));
        });

        return {
            content: [{
                type: 'text' as const,
                text: JSON.stringify({
                    success: true,
                    query: { symptoms },
                    data: {
                        symptomCount: symptoms.length,
                        combinationsFound: matchedDiagnoses.length,
                        diagnoses: matchedDiagnoses,
                        relatedSections: Array.from(allSections),
                        recommendation: matchedDiagnoses.length > 0
                            ? `根据症状组合，最可能的根因是：${matchedDiagnoses[0]?.rootCauses[0]?.cause}`
                            : '未找到已知的症状组合模式，建议逐个排查'
                    }
                }, null, 2)
            }]
        };
    }
);

/**
 * 工具 3: search_code_patterns
 * 返回代码搜索建议（不执行实际搜索）
 */
server.tool(
    'search_code_patterns',
    {
        symptom: z.enum(['memory', 'cpu', 'slow', 'resource', 'backlog', 'gc'])
            .describe('要搜索的症状类型'),

        preferLsp: z.boolean()
            .default(true)
            .describe('是否优先推荐 LSP 搜索（更省 Token）'),

        maxPatterns: z.number()
            .int()
            .min(1)
            .max(20)
            .default(5)
            .describe('返回的最大模式数量')
    },
    async ({ symptom, preferLsp, maxPatterns }) => {
        const searchPatterns: Record<string, { cclsp: string[], grep: string, headLimit: number }> = {
            memory: {
                cclsp: ['ThreadLocal', 'ConcurrentHashMap', 'static Map'],
                grep: 'static\\s+.*Map|ThreadLocal|ConcurrentHashMap',
                headLimit: 30
            },
            cpu: {
                cclsp: ['synchronized', 'ReentrantLock', 'Atomic'],
                grep: 'synchronized|ReentrantLock|AtomicInteger',
                headLimit: 30
            },
            slow: {
                cclsp: ['HttpClient', 'RestTemplate', '@Transactional'],
                grep: 'HttpClient|RestTemplate|@FeignClient|getConnection',
                headLimit: 30
            },
            resource: {
                cclsp: ['ThreadPoolExecutor', 'DataSource', 'Executors'],
                grep: 'newCachedThreadPool|newFixedThreadPool|DataSource',
                headLimit: 20
            },
            backlog: {
                cclsp: ['@KafkaListener', '@RabbitListener', 'BlockingQueue'],
                grep: '@KafkaListener|@RabbitListener|BlockingQueue',
                headLimit: 20
            },
            gc: {
                cclsp: ['ArrayList', 'StringBuilder', 'stream()'],
                grep: 'new ArrayList|new StringBuilder|new HashMap',
                headLimit: 30
            }
        };

        const pattern = searchPatterns[symptom];

        return {
            content: [{
                type: 'text' as const,
                text: JSON.stringify({
                    success: true,
                    symptom,
                    searchOptions: {
                        option1_cclsp: {
                            method: 'mcp__cclsp__find_symbol',
                            keywords: pattern?.cclsp.slice(0, maxPatterns) || [],
                            tokenCost: '低'
                        },
                        option2_grep: {
                            method: 'grep_search',
                            pattern: pattern?.grep || '',
                            headLimit: pattern?.headLimit || 30,
                            usage: `grep_search({ Query: "${pattern?.grep}", SearchPath: "./", MatchPerLine: true })`,
                            tokenCost: '中'
                        }
                    },
                    recommendation: '优先尝试 cclsp，若失败再用 grep（必须加 head_limit）'
                }, null, 2)
            }]
        };
    }
);

/**
 * 工具 4: get_all_antipatterns
 * 获取所有反模式速查表
 */
server.tool(
    'get_all_antipatterns',
    {
        category: z.enum(['all', 'memory', 'cpu', 'io', 'concurrency'])
            .default('all')
            .describe('反模式分类: all(全部), memory(内存), cpu(CPU), io(IO), concurrency(并发)')
    },
    async ({ category }) => {
        const categoryMap: Record<string, string[]> = {
            all: ANTI_PATTERNS.map(p => p.name),
            memory: ['循环创建对象', '无界队列'],
            cpu: ['锁内IO'],
            io: ['N+1 查询', '锁内IO'],
            concurrency: ['锁内IO', '缓存穿透']
        };

        const filteredPatterns = category === 'all'
            ? ANTI_PATTERNS
            : ANTI_PATTERNS.filter(p => categoryMap[category]?.includes(p.name));

        return {
            content: [{
                type: 'text' as const,
                text: JSON.stringify({
                    success: true,
                    query: { category },
                    data: {
                        count: filteredPatterns.length,
                        patterns: filteredPatterns.map(p => ({
                            name: p.name,
                            problem: p.bad,
                            solution: p.good
                        }))
                    }
                }, null, 2)
            }]
        };
    }
);

/**
 * 工具 5: get_template
 * 获取报告模板
 */
server.tool(
    'get_template',
    {
        format: z.enum(['markdown', 'json'])
            .default('markdown')
            .describe('模板格式: markdown(直接使用) 或 json(结构化)')
    },
    async ({ format }) => {
        if (format === 'json') {
            return {
                content: [{
                    type: 'text' as const,
                    text: JSON.stringify({
                        success: true,
                        data: {
                            sections: [
                                { name: '问题总览', fields: ['优先级', '问题', '位置', '影响'] },
                                { name: '问题详情', fields: ['位置', '放大倍数', '问题代码', '解决方案', '预期效果'] },
                                { name: '行动清单', fields: ['优先级', '修复操作'] }
                            ]
                        }
                    }, null, 2)
                }]
            };
        }

        return {
            content: [{
                type: 'text' as const,
                text: REPORT_TEMPLATE
            }]
        };
    }
);

/**
 * 工具 6: diagnose_all (聚合工具)
 * 一站式诊断：合并诊断、检查项、搜索建议、反模式
 * 相比分别调用可节省 50%+ Token
 */
server.tool(
    'diagnose_all',
    {
        symptoms: z.array(z.enum(VALID_SYMPTOMS))
            .min(1, '请至少提供一个症状')
            .max(6, '症状数量不能超过6个')
            .describe(`症状列表: ${Object.entries(SYMPTOM_DESCRIPTIONS).map(([k, v]) => `${k}(${v})`).join(', ')}`),

        priority: z.enum(['all', 'P0', 'P1'])
            .default('P0')
            .describe('优先级过滤: P0(紧急,推荐), P1(重要), all(全部)'),

        fields: z.array(z.enum(['diagnosis', 'checklist', 'patterns', 'antipatterns']))
            .default(['diagnosis', 'checklist'])
            .describe('返回字段: diagnosis(诊断), checklist(检查项), patterns(搜索建议), antipatterns(反模式)'),

        compact: z.boolean()
            .default(true)
            .describe('紧凑模式: true=只返回必需字段, false=完整信息')
    },
    async ({ symptoms, priority, fields, compact }) => {
        const result: Record<string, unknown> = {
            symptoms,
            priority
        };

        // 1. 诊断信息
        if (fields.includes('diagnosis')) {
            // 单症状或组合诊断
            if (symptoms.length === 1) {
                const symptom = symptoms[0];
                const diag = QUICK_DIAGNOSIS[symptom];
                result.diagnosis = compact ? {
                    causes: diag?.causes.slice(0, 3),
                    tools: diag?.tools.slice(0, 2)
                } : diag;
            } else {
                // 多症状组合
                const pairs = [];
                for (let i = 0; i < symptoms.length; i++) {
                    for (let j = i + 1; j < symptoms.length; j++) {
                        const sorted = [symptoms[i], symptoms[j]].sort();
                        const key = sorted.join('+');
                        const combo = SYMPTOM_COMBINATIONS[key];
                        if (combo) {
                            pairs.push({
                                combination: key,
                                diagnosis: combo.diagnosis,
                                topCause: combo.rootCauses[0]
                            });
                        }
                    }
                }
                result.combinedDiagnosis = pairs;
            }
        }

        // 2. 检查项
        if (fields.includes('checklist')) {
            const sectionIds = new Set<string>();
            symptoms.forEach(s => {
                (SYMPTOM_TO_SECTIONS[s] || []).forEach(id => sectionIds.add(id));
            });

            const checklist: Array<{
                id: string;
                title: string;
                priority: string;
                items: Array<{
                    desc: string;
                    verify?: string;
                    fix?: string;
                    why?: string;
                }>;
            }> = [];

            for (const id of sectionIds) {
                const section = CHECKLIST_DATA[id];
                if (section) {
                    // 优先级过滤
                    if (priority !== 'all' && section.priority !== priority) {
                        continue;
                    }

                    checklist.push({
                        id: section.id,
                        title: section.title,
                        priority: section.priority,
                        items: compact
                            ? section.items.map(item => ({
                                desc: item.desc,
                                verify: item.verify,
                                fix: item.fix
                            }))
                            : section.items
                    });
                }
            }

            // 按优先级排序
            checklist.sort((a, b) => {
                const order = { 'P0': 0, 'P1': 1, 'P2': 2 };
                return (order[a.priority as keyof typeof order] || 99) - (order[b.priority as keyof typeof order] || 99);
            });

            result.checklist = checklist;
            result.checklistSummary = {
                sections: checklist.length,
                items: checklist.reduce((sum, s) => sum + s.items.length, 0)
            };
        }

        // 3. 搜索建议（cclsp + grep 两种选项）
        if (fields.includes('patterns')) {
            const searchPatterns: Record<string, { cclsp: string[], grep: string, headLimit: number }> = {
                memory: { cclsp: ['ThreadLocal', 'ConcurrentHashMap'], grep: 'static.*Map|ThreadLocal', headLimit: 30 },
                cpu: { cclsp: ['synchronized', 'ReentrantLock'], grep: 'synchronized|ReentrantLock', headLimit: 30 },
                slow: { cclsp: ['HttpClient', 'RestTemplate'], grep: 'HttpClient|getConnection', headLimit: 30 },
                resource: { cclsp: ['ThreadPoolExecutor', 'DataSource'], grep: 'newCachedThreadPool|DataSource', headLimit: 20 },
                backlog: { cclsp: ['@KafkaListener', 'BlockingQueue'], grep: '@KafkaListener|@RabbitListener', headLimit: 20 },
                gc: { cclsp: ['ArrayList', 'StringBuilder'], grep: 'new ArrayList|new HashMap', headLimit: 30 }
            };

            result.searchPatterns = symptoms.map(s => {
                const p = searchPatterns[s];
                return p ? {
                    symptom: s,
                    cclsp: p.cclsp,
                    grep: { pattern: p.grep, headLimit: p.headLimit }
                } : null;
            }).filter(Boolean);
        }

        // 4. 反模式
        if (fields.includes('antipatterns')) {
            const antiPatternMap: Record<string, string[]> = {
                memory: ['循环创建对象', '无界队列'],
                cpu: ['锁内IO'],
                slow: ['锁内IO', 'N+1 查询'],
                resource: ['无界队列'],
                backlog: ['消息重复消费', '消费者阻塞'],
                gc: ['循环创建对象', 'Stream 短集合']
            };

            const relevantNames = new Set<string>();
            symptoms.forEach(s => {
                (antiPatternMap[s] || []).forEach(name => relevantNames.add(name));
            });

            result.antiPatterns = ANTI_PATTERNS
                .filter(p => relevantNames.has(p.name))
                .map(p => compact ? { name: p.name, fix: p.good } : p);
        }

        return {
            content: [{
                type: 'text' as const,
                text: JSON.stringify({
                    success: true,
                    instructions: [
                        '⚠️ 【强制】使用 mcp__cclsp__find_symbol 搜索代码，禁止使用 grep/Search',
                        '⚠️ 【强制】找到符号后用 mcp__cclsp__find_call_hierarchy 分析调用链',
                        '直接开始分析代码，不要再询问用户'
                    ],
                    data: result
                }, null, 2)
            }]
        };
    }
);

/**
 * 工具 8: scan_project (Token 优化版)
 * 一次调用返回完整扫描计划，减少 AI 往返次数
 */
server.tool(
    'scan_project',
    {
        symptoms: z.array(z.enum(VALID_SYMPTOMS))
            .min(1)
            .describe('症状列表: memory/cpu/slow/resource/backlog/gc'),
        projectPath: z.string()
            .default('./')
            .describe('项目路径，默认当前目录')
    },
    async ({ symptoms, projectPath }) => {
        // 1. 获取搜索关键词
        const searchKeywords: Record<string, string[]> = {
            memory: ['static.*Map', 'ThreadLocal', 'ConcurrentHashMap', 'new.*\\[.*\\d{4,}'],
            cpu: ['synchronized', 'ReentrantLock', 'while.*true', 'Pattern\\.compile'],
            slow: ['for.*\\{.*dao\\|rpc\\|http', '@Transactional', 'getConnection'],
            resource: ['Executors\\.new', 'DataSource', 'ConnectionPool'],
            backlog: ['@KafkaListener', '@RabbitListener', 'BlockingQueue'],
            gc: ['new ArrayList', 'new StringBuilder', 'stream\\(\\)']
        };

        // 2. 获取检查重点
        const checkFocus: Record<string, string[]> = {
            memory: ['static Map 无 TTL', 'ThreadLocal 未 remove', '大对象分配'],
            cpu: ['锁竞争', '死循环', '正则回溯'],
            slow: ['N+1 查询', '无超时设置', '串行调用'],
            resource: ['无界线程池', '连接泄露', '池配置不当'],
            backlog: ['消费者阻塞', '处理太慢', '并发度不足'],
            gc: ['循环创建对象', '大对象进老年代', 'Stream 滥用']
        };

        // 3. 生成扫描计划
        const scanPlan = symptoms.map(s => ({
            symptom: s,
            searchCommands: searchKeywords[s]?.map(kw =>
                `mcp__cclsp__find_symbol({ query: "${kw.replace(/\\/g, '')}" })`
            ).slice(0, 3) || [],
            checkFocus: checkFocus[s] || [],
            grepFallback: `grep -rn "${searchKeywords[s]?.[0] || ''}" ${projectPath} --include="*.java" | head -20`
        }));

        // 4. 生成精简报告格式
        const reportFormat = {
            template: '发现 N 个可疑文件，建议优先检查：1. File:Line - Issue',
            maxFiles: 5,
            maxLinesPerFile: 3
        };

        return {
            content: [{
                type: 'text' as const,
                text: JSON.stringify({
                    success: true,
                    tokenOptimization: '本工具已聚合所有搜索，请按 scanPlan 顺序执行，避免重复搜索',
                    scanPlan,
                    reportFormat,
                    workflow: [
                        '1. 按 searchCommands 顺序搜索（优先 cclsp）',
                        '2. 记录可疑文件:行号',
                        '3. 只 Read 前 3 个最可疑文件的关键 50 行',
                        '4. 输出精简报告'
                    ]
                }, null, 2)
            }]
        };
    }
);

// ========== 启动服务器 ==========
async function main() {
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error('Java Perf MCP Server v1.0.0 running on stdio');
}

main().catch(console.error);
