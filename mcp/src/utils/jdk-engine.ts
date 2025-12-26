/**
 * JDK 引擎 - 动态/字节码分析
 * 
 * 核心能力：
 * 1. JDK 环境检测（自动检测是否可用）
 * 2. jstack 线程分析（死锁检测、BLOCKED 线程）
 * 3. javap 字节码分析（锁范围、方法内联）
 * 4. jmap 堆分析（大对象、类实例统计）
 * 5. 优雅降级（无 JDK 时提供清晰提示）
 */

import { exec } from 'child_process';
import { promisify } from 'util';
import * as path from 'path';
import * as fs from 'fs';

const execAsync = promisify(exec);

export interface JdkStatus {
    available: boolean;
    version: string | null;
    javaHome: string | null;
}

export interface ThreadDumpAnalysis {
    totalThreads: number;
    blockedThreads: number;
    waitingThreads: number;
    deadlocks: string[];
    hotSpots: Array<{ thread: string; state: string; stack: string[] }>;
}

export class JdkEngine {
    private status: JdkStatus = {
        available: false,
        version: null,
        javaHome: null
    };

    constructor() {
        this.checkJdk();
    }

    /**
     * 检测 JDK 环境
     */
    private async checkJdk(): Promise<void> {
        try {
            // 检查 java 版本
            const { stdout: javaVersion } = await execAsync('java -version 2>&1');
            const versionMatch = javaVersion.match(/version "([^"]+)"/);

            this.status.version = versionMatch ? versionMatch[1] : 'unknown';
            this.status.javaHome = process.env.JAVA_HOME || null;
            this.status.available = true;
        } catch {
            this.status.available = false;
        }
    }

    /**
     * 获取 JDK 状态
     */
    public getStatus(): JdkStatus {
        return this.status;
    }

    /**
     * 获取线程 Dump 并分析
     */
    public async analyzeThreadDump(pid: string): Promise<ThreadDumpAnalysis | string> {
        if (!this.status.available) {
            return this.getUnavailableMessage('jstack');
        }

        try {
            const { stdout } = await execAsync(`jstack -l ${pid}`, { timeout: 30000 });
            return this.parseThreadDump(stdout);
        } catch (error: any) {
            if (error.message.includes('No such process')) {
                return `进程 ${pid} 不存在，请确认 PID 是否正确`;
            }
            return `jstack 执行失败: ${error.message}`;
        }
    }

    /**
     * 解析线程 Dump
     */
    private parseThreadDump(dump: string): ThreadDumpAnalysis {
        const lines = dump.split('\n');
        const result: ThreadDumpAnalysis = {
            totalThreads: 0,
            blockedThreads: 0,
            waitingThreads: 0,
            deadlocks: [],
            hotSpots: []
        };

        let currentThread = '';
        let currentState = '';
        let currentStack: string[] = [];

        for (const line of lines) {
            // 检测线程开始
            if (line.startsWith('"')) {
                // 保存上一个线程
                if (currentThread && (currentState === 'BLOCKED' || currentState === 'WAITING')) {
                    result.hotSpots.push({
                        thread: currentThread,
                        state: currentState,
                        stack: currentStack.slice(0, 5)  // 只保留前 5 行堆栈
                    });
                }

                currentThread = line.split('"')[1] || '';
                currentStack = [];
                result.totalThreads++;
            }

            // 检测线程状态
            if (line.includes('java.lang.Thread.State:')) {
                const stateMatch = line.match(/State:\s+(\w+)/);
                currentState = stateMatch ? stateMatch[1] : '';

                if (currentState === 'BLOCKED') result.blockedThreads++;
                if (currentState === 'WAITING' || currentState === 'TIMED_WAITING') result.waitingThreads++;
            }

            // 收集堆栈
            if (line.trim().startsWith('at ')) {
                currentStack.push(line.trim());
            }

            // 检测死锁
            if (line.includes('Found one Java-level deadlock') || line.includes('Found deadlock')) {
                result.deadlocks.push(line);
            }
        }

        // 只保留前 10 个热点线程
        result.hotSpots = result.hotSpots.slice(0, 10);

        return result;
    }

    /**
     * 分析字节码
     */
    public async analyzeBytecode(classFilePath: string): Promise<string> {
        if (!this.status.available) {
            return this.getUnavailableMessage('javap');
        }

        // 尝试找到 class 文件
        const possiblePaths = this.findClassFile(classFilePath);

        for (const classPath of possiblePaths) {
            if (fs.existsSync(classPath)) {
                try {
                    const { stdout } = await execAsync(`javap -c -v -p "${classPath}"`, { timeout: 30000 });
                    return this.summarizeBytecode(stdout);
                } catch (error: any) {
                    return `javap 执行失败: ${error.message}`;
                }
            }
        }

        return `未找到编译后的 class 文件。请先编译项目 (mvn compile 或 ./gradlew build)。
        
尝试查找的路径:
${possiblePaths.map(p => `  - ${p}`).join('\n')}`;
    }

    /**
     * 查找 class 文件
     */
    private findClassFile(sourceFilePath: string): string[] {
        const baseName = path.basename(sourceFilePath, '.java');
        const dirName = path.dirname(sourceFilePath);

        // 常见的编译输出路径
        const patterns = [
            // Maven
            sourceFilePath.replace('src/main/java', 'target/classes').replace('.java', '.class'),
            // Gradle
            sourceFilePath.replace('src/main/java', 'build/classes/java/main').replace('.java', '.class'),
            // 直接同目录
            path.join(dirName, baseName + '.class')
        ];

        return patterns;
    }

    /**
     * 精简字节码输出
     */
    private summarizeBytecode(bytecode: string): string {
        const lines = bytecode.split('\n');
        const summary: string[] = [];

        let inMethod = false;
        let methodLines: string[] = [];
        let syncCount = 0;

        for (const line of lines) {
            // 类信息
            if (line.includes('Compiled from') || line.includes('public class')) {
                summary.push(line);
            }

            // 方法签名
            if (line.match(/^\s*(public|private|protected).*\(/)) {
                if (methodLines.length > 0) {
                    summary.push(methodLines[0]);
                    if (syncCount > 0) summary.push(`    [SYNC] monitorenter/exit: ${syncCount}`);
                }
                methodLines = [line];
                syncCount = 0;
                inMethod = true;
            }

            // 同步指令
            if (line.includes('monitorenter') || line.includes('monitorexit')) {
                syncCount++;
            }
        }

        return summary.slice(0, 30).join('\n') + '\n\n[输出已截断，仅显示关键信息]';
    }

    /**
     * 获取堆统计
     */
    public async analyzeHeap(pid: string): Promise<string> {
        if (!this.status.available) {
            return this.getUnavailableMessage('jmap');
        }

        try {
            const { stdout } = await execAsync(`jmap -histo:live ${pid} | head -30`, { timeout: 60000 });
            return `### 堆内存 Top 30 对象\n\n\`\`\`\n${stdout}\n\`\`\``;
        } catch (error: any) {
            return `jmap 执行失败: ${error.message}`;
        }
    }

    /**
     * JDK 不可用时的提示信息
     */
    private getUnavailableMessage(tool: string): string {
        return `## ⚠️ JDK 工具不可用

${tool} 需要 JDK 环境，但未检测到。

**解决方案**:
1. 安装 JDK: \`brew install openjdk\` (Mac) 或 \`apt install openjdk-17-jdk\` (Linux)
2. 设置 JAVA_HOME 环境变量
3. 确保 ${tool} 在 PATH 中

**当前状态**: 将使用静态代码分析作为替代方案。`;
    }
}

// 导出单例
export const jdkEngine = new JdkEngine();
