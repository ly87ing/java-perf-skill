/**
 * Forensic 模块 - 日志时序分析 + 坐标提取
 * 
 * 核心能力：
 * 1. 时序折叠算法：将高频重复日志压缩为统计信息
 * 2. 坐标提取：从堆栈中提取 (File.java:123) 格式的代码位置
 * 3. 错误摘要：提取 Exception/ERROR 信息
 */

import * as fs from 'fs';
import * as path from 'path';
import { CrimeScene, LogAnomaly, LogAnalysisResult } from '../types.js';

// ========== 日志归一化 ==========

/**
 * 归一化日志行（去除时间戳、数字、UUID 等变量部分）
 * 目的：识别重复模式
 */
function normalizeLogLine(line: string): string {
    return line
        // 去除常见时间戳格式
        .replace(/\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}[.,]?\d*/g, '{TIME}')
        // 去除纯数字
        .replace(/\b\d+\b/g, '{N}')
        // 去除 UUID
        .replace(/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}/gi, '{UUID}')
        // 去除 IP 地址
        .replace(/\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/g, '{IP}')
        // 截断过长内容
        .trim()
        .substring(0, 150);
}

/**
 * 从日志行提取时间戳（毫秒）
 */
function extractTimestamp(line: string): number | null {
    // 匹配常见格式：2024-01-01 12:00:00 或 2024-01-01T12:00:00
    const patterns = [
        /(\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2})/,
        /(\d{2}:\d{2}:\d{2}[.,]\d{3})/  // HH:mm:ss.SSS
    ];

    for (const pattern of patterns) {
        const match = line.match(pattern);
        if (match) {
            const ts = Date.parse(match[1].replace(' ', 'T'));
            if (!isNaN(ts)) return ts;
        }
    }
    return null;
}

// ========== 坐标提取 ==========

/**
 * 从日志内容中提取代码坐标（堆栈信息）
 * 匹配格式：(OrderService.java:45) 或 at com.xxx.OrderService.method(OrderService.java:45)
 */
function extractCoordinates(content: string): CrimeScene[] {
    const scenes: CrimeScene[] = [];
    const seen = new Set<string>();

    // 匹配 Java 堆栈格式
    const regex = /\((\w+\.java):(\d+)\)/g;
    let match;

    while ((match = regex.exec(content)) !== null) {
        const key = `${match[1]}:${match[2]}`;
        if (!seen.has(key)) {
            seen.add(key);
            scenes.push({
                file: match[1],
                line: parseInt(match[2]),
                reason: 'Stack Trace'
            });
        }
    }

    // 按出现频率排序（频繁出现的可能是热点）
    return scenes.slice(0, 20);  // 最多返回 20 个坐标
}

// ========== 安全限制常量 ==========
const MAX_MEMORY_MB = 1024;       // 最大内存增量 1GB
const MIN_PROCESS_TIME_MS = 30000; // 最小处理时间 30 秒
const MS_PER_MB = 100;            // 每 MB 给 100ms 处理时间
const CHUNK_SIZE = 256 * 1024;    // 每次读取 256KB

/**
 * 检查内存使用，返回当前 MB
 */
function getMemoryUsageMB(): number {
    return process.memoryUsage().heapUsed / (1024 * 1024);
}

/**
 * 流式分析日志文件（安全版本）
 * 
 * 安全特性：
 * 1. 流式处理 - 不一次性加载全部内容
 * 2. 内存熔断 - 超过 100MB 自动停止
 * 3. 时间熔断 - 超过 10 秒自动停止
 * 
 * @param filePath 日志文件路径
 * @param maxLines 最大读取行数（防止内存溢出）
 */
export function analyzeLog(filePath: string, maxLines: number = 50000): LogAnalysisResult {
    const startTime = Date.now();
    const startMemory = getMemoryUsageMB();

    // 用于收集数据的 Map（限制大小）
    const patternMap = new Map<string, {
        count: number;
        firstTs: number | null;
        lastTs: number | null;
        example: string;
    }>();

    const exceptionMap = new Map<string, {
        type: string;
        location: string;
        count: number;
        example: string;
    }>();

    const coordinates: CrimeScene[] = [];
    const coordSeen = new Set<string>();

    let linesProcessed = 0;
    let truncated = false;
    let truncateReason = '';

    try {
        const stat = fs.statSync(filePath);
        const fileSize = stat.size;

        // ===== 流式读取 =====
        const fd = fs.openSync(filePath, 'r');
        const buffer = Buffer.alloc(CHUNK_SIZE);
        let position = 0;
        let leftover = '';

        // 动态超时：根据文件大小计算
        const fileSizeMB = fileSize / (1024 * 1024);
        const dynamicTimeout = Math.max(MIN_PROCESS_TIME_MS, fileSizeMB * MS_PER_MB);

        while (position < fileSize && linesProcessed < maxLines) {
            // 熔断检查：时间（动态超时）
            if (Date.now() - startTime > dynamicTimeout) {
                truncated = true;
                truncateReason = `⚠️ 分析超时 (>${Math.round(dynamicTimeout / 1000)}s for ${fileSizeMB.toFixed(0)}MB)，已自动终止`;
                break;
            }

            // 熔断检查：内存
            const currentMemory = getMemoryUsageMB();
            if (currentMemory - startMemory > MAX_MEMORY_MB) {
                truncated = true;
                truncateReason = `⚠️ 内存占用过高 (>${MAX_MEMORY_MB}MB)，已自动终止`;
                break;
            }

            // 读取一块数据
            const bytesRead = fs.readSync(fd, buffer, 0, CHUNK_SIZE, position);
            if (bytesRead === 0) break;

            position += bytesRead;
            const chunk = leftover + buffer.toString('utf-8', 0, bytesRead);
            const lines = chunk.split('\n');

            // 保留最后一行（可能不完整）
            leftover = lines.pop() || '';

            // 处理每一行
            for (const line of lines) {
                if (!line.trim()) continue;
                linesProcessed++;

                // 归一化模式统计
                const normalized = normalizeLogLine(line);
                const ts = extractTimestamp(line);

                if (!patternMap.has(normalized)) {
                    // 限制 Map 大小
                    if (patternMap.size < 1000) {
                        patternMap.set(normalized, {
                            count: 0,
                            firstTs: ts,
                            lastTs: ts,
                            example: line.substring(0, 200)
                        });
                    }
                }

                const entry = patternMap.get(normalized);
                if (entry) {
                    entry.count++;
                    if (ts) entry.lastTs = ts;
                }

                // 异常指纹提取
                const exMatch = line.match(/(\w+Exception|\w+Error)\s*(:|at\s+)?\s*([^\n]*)/i);
                if (exMatch) {
                    const exType = exMatch[1];
                    const context = exMatch[3] || '';
                    const locationMatch = context.match(/(\w+\.)+\w+/);
                    const location = locationMatch ? locationMatch[0].split('.').slice(-2).join('.') : 'Unknown';
                    const fingerprint = `${exType}@${location}`;

                    if (!exceptionMap.has(fingerprint)) {
                        if (exceptionMap.size < 100) {
                            exceptionMap.set(fingerprint, {
                                type: exType,
                                location,
                                count: 0,
                                example: exMatch[0].substring(0, 150)
                            });
                        }
                    }

                    const exEntry = exceptionMap.get(fingerprint);
                    if (exEntry) exEntry.count++;
                }

                // 坐标提取
                const coordMatch = line.match(/\((\w+\.java):(\d+)\)/);
                if (coordMatch && coordinates.length < 20) {
                    const key = `${coordMatch[1]}:${coordMatch[2]}`;
                    if (!coordSeen.has(key)) {
                        coordSeen.add(key);
                        coordinates.push({
                            file: coordMatch[1],
                            line: parseInt(coordMatch[2]),
                            reason: 'Stack Trace'
                        });
                    }
                }
            }
        }

        fs.closeSync(fd);

    } catch (err) {
        return {
            summary: `Error reading log file: ${err}`,
            anomalies: [],
            errors: [],
            coordinates: []
        };
    }

    const processTime = Date.now() - startTime;
    const memoryUsed = Math.round(getMemoryUsageMB() - startMemory);

    // ===== 计算高频异常 =====
    const anomalies: LogAnomaly[] = [];

    for (const [pattern, data] of patternMap) {
        const duration = (data.lastTs && data.firstTs)
            ? (data.lastTs - data.firstTs) / 1000
            : 0;
        const rate = duration > 0 ? data.count / duration : 0;

        if (data.count > 1000 || rate > 10) {
            anomalies.push({
                pattern,
                count: data.count,
                rate: Math.round(rate * 10) / 10,
                duration: Math.round(duration),
                example: data.example
            });
        }
    }
    anomalies.sort((a, b) => b.rate - a.rate);

    // ===== 转换异常指纹 =====
    const exceptionFingerprints = Array.from(exceptionMap.values())
        .sort((a, b) => b.count - a.count);

    // ===== 生成摘要 =====
    let summary = `### 日志分析: ${path.basename(filePath)}\n\n`;

    // 安全指标
    summary += `**性能**: ${linesProcessed.toLocaleString()} 行, ${processTime}ms, +${memoryUsed}MB\n`;
    if (truncated) {
        summary += `\n> [!CAUTION]\n> ${truncateReason}\n\n`;
    }
    summary += `\n`;

    // 异常指纹归类
    if (exceptionFingerprints.length > 0) {
        const totalExceptions = exceptionFingerprints.reduce((s, e) => s + e.count, 0);

        summary += `## 🔬 异常指纹归类 (${exceptionFingerprints.length} 类, 共 ${totalExceptions.toLocaleString()} 次)\n\n`;
        summary += `| # | 类型 | 位置 | 次数 | 标记 |\n`;
        summary += `|---|------|------|------|------|\n`;

        exceptionFingerprints.slice(0, 10).forEach((e, i) => {
            let tag = '';
            if (e.count > 1000) tag = '🔥 核心噪音';
            else if (e.count < 10) tag = '⚠️ 可能根因';
            else if (e.count < 100) tag = '🔍 需关注';

            summary += `| ${i + 1} | \`${e.type}\` | ${e.location} | ${e.count.toLocaleString()} | ${tag} |\n`;
        });
        summary += '\n';

        const keyErrors = exceptionFingerprints.filter(e => e.count < 10);
        if (keyErrors.length > 0) {
            summary += `> [!IMPORTANT]\n`;
            summary += `> 发现 ${keyErrors.length} 个低频异常，可能是根因！\n\n`;
        }
    }

    // 高频日志风暴
    if (anomalies.length > 0) {
        summary += `## 🚨 高频日志风暴\n\n`;
        anomalies.slice(0, 3).forEach((a, i) => {
            summary += `${i + 1}. **${a.rate}/s** (${a.count.toLocaleString()}次) ${a.example.substring(0, 60)}...\n`;
        });
        summary += '\n';
    }

    // 代码坐标
    if (coordinates.length > 0) {
        summary += `## 📍 代码坐标\n\n`;
        coordinates.slice(0, 5).forEach(c => {
            summary += `- \`${c.file}:${c.line}\`\n`;
        });
    }

    return {
        summary,
        anomalies: anomalies.slice(0, 10),
        errors: exceptionFingerprints.slice(0, 20).map(e => e.example),
        coordinates
    };
}

/**
 * 读取图片为 Base64
 */
export function readImageAsBase64(filePath: string): string | null {
    try {
        const buffer = fs.readFileSync(filePath);
        return buffer.toString('base64');
    } catch {
        return null;
    }
}

/**
 * 扫描目录中的日志和图片
 */
export function scanEvidenceDir(dirPath: string): {
    logs: LogAnalysisResult[];
    images: Array<{ path: string; base64: string; mimeType: string }>;
} {
    const result = {
        logs: [] as LogAnalysisResult[],
        images: [] as Array<{ path: string; base64: string; mimeType: string }>
    };

    if (!fs.existsSync(dirPath)) {
        return result;
    }

    const files = fs.readdirSync(dirPath);

    for (const file of files) {
        const fullPath = path.join(dirPath, file);
        const stat = fs.statSync(fullPath);

        if (!stat.isFile()) continue;

        // 日志文件
        if (/\.(log|txt|out)$/i.test(file)) {
            result.logs.push(analyzeLog(fullPath));
        }
        // 图片文件
        else if (/\.(png|jpg|jpeg|gif)$/i.test(file)) {
            const base64 = readImageAsBase64(fullPath);
            if (base64) {
                const mimeType = file.endsWith('.png') ? 'image/png' : 'image/jpeg';
                result.images.push({ path: fullPath, base64, mimeType });
            }
        }
    }

    return result;
}
