/**
 * AST 引擎 - 基于 Tree-sitter 的静态代码分析
 * 
 * 核心能力：
 * 1. 精准 AST 解析（比正则更准确）
 * 2. 符号索引（方法 → 类映射）
 * 3. N+1 检测（循环内 DAO 调用）
 * 4. ThreadLocal 泄露检测
 * 5. 锁范围分析
 */

import Parser from 'tree-sitter';
import Java from 'tree-sitter-java';
import * as fs from 'fs';
import * as path from 'path';

// 初始化解析器
const parser = new Parser();
parser.setLanguage(Java);

// ========== 类型定义 ==========

export interface MethodSignature {
    className: string;
    methodName: string;
    isDao: boolean;       // 是否是 DAO/Mapper
    isSync: boolean;      // 是否有 synchronized
    filePath: string;
    line: number;
}

export interface AstIssue {
    type: 'N_PLUS_ONE' | 'THREADLOCAL_LEAK' | 'LOCK_CONTENTION' | 'NESTED_LOOP' | 'LARGE_SYNC_BLOCK';
    severity: 'P0' | 'P1';
    file: string;
    line: number;
    message: string;
    evidence: string;
    suggestion?: string;
}

// 符号索引
const symbolIndex = new Map<string, MethodSignature[]>();

// ========== 索引构建 ==========

/**
 * 构建项目符号索引
 */
export async function buildProjectIndex(rootPath: string): Promise<number> {
    symbolIndex.clear();
    const files = getAllJavaFiles(rootPath);

    for (const file of files) {
        try {
            const content = fs.readFileSync(file, 'utf-8');
            indexFile(file, content);
        } catch {
            // 忽略读取错误
        }
    }

    console.error(`[AST Engine] Indexed ${symbolIndex.size} methods from ${files.length} files`);
    return symbolIndex.size;
}

/**
 * 索引单个文件
 */
function indexFile(filePath: string, content: string): void {
    const tree = parser.parse(content);
    const rootNode = tree.rootNode;

    // 提取类名
    let className = 'Unknown';
    for (const child of rootNode.children) {
        if (child.type === 'class_declaration') {
            const nameNode = child.childForFieldName('name');
            if (nameNode) {
                className = nameNode.text;
                break;
            }
        }
    }

    // 判断是否是 DAO/Mapper
    const isDao = /Mapper|Dao|Repository/i.test(className) ||
        content.includes('@Mapper') ||
        content.includes('@Repository');

    // 提取方法
    extractMethods(rootNode, className, isDao, filePath, content);
}

/**
 * 递归提取方法
 */
function extractMethods(node: Parser.SyntaxNode, className: string, isDao: boolean, filePath: string, content: string): void {
    if (node.type === 'method_declaration') {
        const nameNode = node.childForFieldName('name');
        if (nameNode) {
            const methodName = nameNode.text;
            const isSync = content.substring(node.startIndex, node.startIndex + 100).includes('synchronized');
            const line = node.startPosition.row + 1;

            if (!symbolIndex.has(methodName)) {
                symbolIndex.set(methodName, []);
            }
            symbolIndex.get(methodName)!.push({
                className,
                methodName,
                isDao,
                isSync,
                filePath,
                line
            });
        }
    }

    for (const child of node.children) {
        extractMethods(child, className, isDao, filePath, content);
    }
}

// ========== 代码分析 ==========

/**
 * 分析源代码
 */
export function analyzeSourceCode(code: string, filePath: string = 'unknown.java'): AstIssue[] {
    const issues: AstIssue[] = [];
    const tree = parser.parse(code);
    const rootNode = tree.rootNode;

    // 1. 检测 N+1 问题
    detectNPlusOne(rootNode, filePath, code, issues);

    // 2. 检测 ThreadLocal 泄露
    detectThreadLocalLeak(rootNode, filePath, code, issues);

    // 3. 检测大锁块
    detectLargeSyncBlock(rootNode, filePath, code, issues);

    // 4. 检测嵌套循环
    detectNestedLoop(rootNode, filePath, code, issues);

    return issues;
}

/**
 * 检测 N+1 问题
 */
function detectNPlusOne(node: Parser.SyntaxNode, filePath: string, code: string, issues: AstIssue[]): void {
    const loopTypes = ['for_statement', 'enhanced_for_statement', 'while_statement'];

    if (loopTypes.includes(node.type)) {
        // 在循环内查找方法调用
        const calls = findMethodCalls(node);

        for (const call of calls) {
            const methodName = call.text;
            const candidates = symbolIndex.get(methodName) || [];
            const daoCall = candidates.find(c => c.isDao);

            if (daoCall) {
                issues.push({
                    type: 'N_PLUS_ONE',
                    severity: 'P0',
                    file: filePath,
                    line: call.startPosition.row + 1,
                    message: `循环内调用 DAO 方法 '${methodName}'（来自 ${daoCall.className}）`,
                    evidence: getLineContent(code, call.startPosition.row),
                    suggestion: '使用 IN 批量查询替代循环单条查询'
                });
            }

            // 启发式检测：方法名包含常见 DAO 模式
            if (!daoCall && /^(find|get|select|query|load|fetch)/.test(methodName)) {
                issues.push({
                    type: 'N_PLUS_ONE',
                    severity: 'P1',
                    file: filePath,
                    line: call.startPosition.row + 1,
                    message: `循环内调用疑似查询方法 '${methodName}'`,
                    evidence: getLineContent(code, call.startPosition.row),
                    suggestion: '请确认此方法是否涉及 IO，考虑批量化'
                });
            }
        }
    }

    for (const child of node.children) {
        detectNPlusOne(child, filePath, code, issues);
    }
}

/**
 * 检测 ThreadLocal 泄露
 */
function detectThreadLocalLeak(node: Parser.SyntaxNode, filePath: string, code: string, issues: AstIssue[]): void {
    if (node.type === 'field_declaration') {
        const text = code.substring(node.startIndex, node.endIndex);
        if (text.includes('ThreadLocal')) {
            // 检查是否有 remove 调用
            const hasRemove = code.includes('.remove()') && code.includes('finally');

            if (!hasRemove) {
                issues.push({
                    type: 'THREADLOCAL_LEAK',
                    severity: 'P0',
                    file: filePath,
                    line: node.startPosition.row + 1,
                    message: 'ThreadLocal 变量可能未正确清理',
                    evidence: text.substring(0, 100),
                    suggestion: '在 finally 块中调用 threadLocal.remove()'
                });
            }
        }
    }

    for (const child of node.children) {
        detectThreadLocalLeak(child, filePath, code, issues);
    }
}

/**
 * 检测大锁块
 */
function detectLargeSyncBlock(node: Parser.SyntaxNode, filePath: string, code: string, issues: AstIssue[]): void {
    if (node.type === 'synchronized_statement') {
        const blockSize = node.endPosition.row - node.startPosition.row;

        if (blockSize > 20) {
            issues.push({
                type: 'LARGE_SYNC_BLOCK',
                severity: 'P1',
                file: filePath,
                line: node.startPosition.row + 1,
                message: `synchronized 块过大 (${blockSize} 行)`,
                evidence: getLineContent(code, node.startPosition.row),
                suggestion: '减小锁粒度，只锁关键代码'
            });
        }

        // 检测锁内是否有 IO
        const text = code.substring(node.startIndex, node.endIndex);
        if (/\.(http|rpc|dao|mapper|execute|connect)/i.test(text)) {
            issues.push({
                type: 'LOCK_CONTENTION',
                severity: 'P0',
                file: filePath,
                line: node.startPosition.row + 1,
                message: 'synchronized 块内包含 IO 操作',
                evidence: getLineContent(code, node.startPosition.row),
                suggestion: '将 IO 操作移到锁外'
            });
        }
    }

    for (const child of node.children) {
        detectLargeSyncBlock(child, filePath, code, issues);
    }
}

/**
 * 检测嵌套循环
 */
function detectNestedLoop(node: Parser.SyntaxNode, filePath: string, code: string, issues: AstIssue[], depth: number = 0): void {
    const loopTypes = ['for_statement', 'enhanced_for_statement', 'while_statement'];

    if (loopTypes.includes(node.type)) {
        if (depth >= 1) {
            issues.push({
                type: 'NESTED_LOOP',
                severity: 'P1',
                file: filePath,
                line: node.startPosition.row + 1,
                message: `检测到嵌套循环 (深度 ${depth + 1})`,
                evidence: getLineContent(code, node.startPosition.row),
                suggestion: '考虑使用 Map 等数据结构优化为 O(N)'
            });
        }
        depth++;
    }

    for (const child of node.children) {
        detectNestedLoop(child, filePath, code, issues, depth);
    }
}

// ========== 辅助函数 ==========

function findMethodCalls(node: Parser.SyntaxNode): Parser.SyntaxNode[] {
    const calls: Parser.SyntaxNode[] = [];

    if (node.type === 'method_invocation') {
        const nameNode = node.childForFieldName('name');
        if (nameNode) {
            calls.push(nameNode);
        }
    }

    for (const child of node.children) {
        calls.push(...findMethodCalls(child));
    }

    return calls;
}

function getLineContent(code: string, lineIndex: number): string {
    const lines = code.split('\n');
    return lines[lineIndex]?.trim().substring(0, 100) || '';
}

function getAllJavaFiles(dir: string, depth: number = 0): string[] {
    if (depth > 10) return [];

    const files: string[] = [];

    try {
        const entries = fs.readdirSync(dir);
        for (const entry of entries) {
            const fullPath = path.join(dir, entry);
            const stat = fs.statSync(fullPath);

            if (stat.isDirectory()) {
                if (!['node_modules', 'target', 'build', '.git', '.idea'].includes(entry)) {
                    files.push(...getAllJavaFiles(fullPath, depth + 1));
                }
            } else if (entry.endsWith('.java')) {
                files.push(fullPath);
            }
        }
    } catch {
        // 忽略权限错误
    }

    return files;
}

/**
 * 获取符号索引统计
 */
export function getIndexStats(): { methods: number; daoMethods: number } {
    let daoMethods = 0;
    for (const signatures of symbolIndex.values()) {
        if (signatures.some(s => s.isDao)) {
            daoMethods++;
        }
    }
    return { methods: symbolIndex.size, daoMethods };
}
