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

        includeItems: z.boolean()
            .default(true)
            .describe('是否返回详细检查项，默认 true')
    },
    async ({ symptoms, includeItems }) => {
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
            itemCount: number;
            items?: string[];
        }> = [];

        for (const id of sectionIds) {
            const section = CHECKLIST_DATA[id];
            if (section) {
                checklist.push({
                    id: section.id,
                    title: section.title,
                    itemCount: section.items.length,
                    ...(includeItems ? { items: section.items } : {})
                });
            }
        }

        return {
            content: [{
                type: 'text' as const,
                text: JSON.stringify({
                    success: true,
                    query: { symptoms, includeItems },
                    data: {
                        sectionCount: checklist.length,
                        totalItems: checklist.reduce((sum, s) => sum + s.itemCount, 0),
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
        const patterns: Record<string, { lsp: string[], grep: string[] }> = {
            memory: {
                lsp: ['ThreadLocal', 'ConcurrentHashMap', 'HashMap', 'ArrayList', 'LinkedList'],
                grep: ['static.*Map|static.*List|ThreadLocal|ConcurrentHashMap']
            },
            cpu: {
                lsp: ['synchronized', 'ReentrantLock', 'Pattern', 'Atomic', 'volatile'],
                grep: ['synchronized|ReentrantLock|Pattern\\.compile|AtomicInteger']
            },
            slow: {
                lsp: ['HttpClient', 'RestTemplate', 'WebClient', 'Connection', 'Statement'],
                grep: ['HttpClient|RestTemplate|@FeignClient|getConnection|createStatement']
            },
            resource: {
                lsp: ['ThreadPoolExecutor', 'ExecutorService', 'DataSource', 'PooledConnection'],
                grep: ['newCachedThreadPool|newFixedThreadPool|DataSource|ConnectionPool']
            },
            backlog: {
                lsp: ['BlockingQueue', 'LinkedBlockingQueue', 'Consumer', 'Subscriber'],
                grep: ['BlockingQueue|@RabbitListener|@KafkaListener|@StreamListener']
            },
            gc: {
                lsp: ['ArrayList', 'HashMap', 'StringBuilder', 'BigDecimal', 'Stream'],
                grep: ['new ArrayList|new HashMap|new StringBuilder|new BigDecimal|\.stream\\(\\)']
            }
        };

        const searchPatterns = patterns[symptom] || { lsp: [], grep: [] };
        const limitedLsp = searchPatterns.lsp.slice(0, maxPatterns);
        const limitedGrep = searchPatterns.grep.slice(0, Math.ceil(maxPatterns / 2));

        return {
            content: [{
                type: 'text' as const,
                text: JSON.stringify({
                    success: true,
                    query: { symptom, preferLsp, maxPatterns },
                    data: {
                        recommendation: preferLsp
                            ? 'LSP (cclsp) - 语义搜索，大幅节省 Token'
                            : 'Grep - 文本搜索，需限制输出',
                        lspSymbols: limitedLsp,
                        grepPatterns: limitedGrep,
                        instructions: preferLsp
                            ? [
                                '1. 先测试 LSP: mcp__cclsp__find_definition',
                                '2. 使用 mcp__cclsp__find_references 查找引用',
                                '3. 只返回位置，不占用大量 Context'
                            ]
                            : [
                                '1. 使用 grep_search 搜索模式',
                                '2. 必须添加 head_limit: 50',
                                '3. 找到文件后用 view_file 定点读取'
                            ]
                    }
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

// ========== 启动服务器 ==========
async function main() {
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error('Java Perf MCP Server v1.0.0 running on stdio');
}

main().catch(console.error);
