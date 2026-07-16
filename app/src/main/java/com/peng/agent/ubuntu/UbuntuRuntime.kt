package com.peng.agent.ubuntu

import android.content.Context
import android.util.Log
import java.io.*
import java.util.concurrent.TimeUnit

/**
 * Ubuntu Runtime 环境
 * 使用 proot 执行 Ubuntu rootfs 中的命令
 * 
 * 架构说明：
 * - proot: 从 assets 复制到 /data/data/com.peng.agent/files/bin/proot
 * - ubuntu-rootfs: 从 assets 解压到 /data/data/com.peng.agent/files/ubuntu-rootfs/
 * - peng-daemon: 位于 ubuntu-rootfs/root/peng-daemon，由 proot 启动
 */
class UbuntuRuntime(private val context: Context) {
    
    companion object {
        private const val TAG = "UbuntuRuntime"
        
        // 路径常量
        const val APP_FILES = "/data/data/com.peng.agent/files"
        const val PROOT_PATH = "$APP_FILES/bin/proot"
        const val UBUNTU_ROOTFS = "$APP_FILES/ubuntu-rootfs"
        const val SOCKET_PATH = "$APP_FILES/peng.sock"
    }
    
    /**
     * 执行命令的结果
     */
    data class ExecutionResult(
        val exitCode: Int,
        val stdout: String,
        val stderr: String
    )
    
    /**
     * 在 Ubuntu 环境中执行命令
     */
    fun execute(command: String, timeoutSeconds: Long = 60): ExecutionResult {
        return execute(arrayOf("/bin/bash", "-c", command), timeoutSeconds)
    }
    
    /**
     * 在 Ubuntu 环境中执行命令（带参数数组）
     */
    fun execute(args: Array<String>, timeoutSeconds: Long = 60): ExecutionResult {
        val prootArgs = buildProotArgs(args)
        
        return try {
            val process = Runtime.getRuntime().exec(prootArgs, buildEnv())
            
            val stdout = StringBuilder()
            val stderr = StringBuilder()
            
            // 读取输出
            Thread {
                process.inputStream.bufferedReader().use { reader ->
                    var line = reader.readLine()
                    while (line != null) {
                        stdout.append(line).append("\n")
                        line = reader.readLine()
                    }
                }
            }.start()
            
            Thread {
                process.errorStream.bufferedReader().use { reader ->
                    var line = reader.readLine()
                    while (line != null) {
                        stderr.append(line).append("\n")
                        line = reader.readLine()
                    }
                }
            }.start()
            
            // 等待完成
            val finished = process.waitFor(timeoutSeconds, TimeUnit.SECONDS)
            
            if (!finished) {
                process.destroyForcibly()
                return ExecutionResult(-1, "", "Timeout after ${timeoutSeconds}s")
            }
            
            ExecutionResult(process.exitValue(), stdout.toString(), stderr.toString())
        } catch (e: Exception) {
            Log.e(TAG, "Execution failed", e)
            ExecutionResult(-1, "", e.message ?: "Unknown error")
        }
    }
    
    /**
     * 构建 proot 命令参数
     */
    private fun buildProotArgs(args: Array<String>): Array<String> {
        val prootArgs = mutableListOf(
            PROOT_PATH,
            "--rootfs=$UBUNTU_ROOTFS",
            "-r", UBUNTU_ROOTFS,
            "-b", "/dev",
            "-b", "/proc",
            "-b", "/sys",
            "-b", "$APP_FILES:$APP_FILES",
            "--kill-on-exit"
        )
        
        // 添加用户传递的参数
        prootArgs.addAll(args)
        
        return prootArgs.toTypedArray()
    }
    
    /**
     * 构建环境变量
     */
    private fun buildEnv(): Array<String> {
        return arrayOf(
            "PATH=/usr/bin:/bin",
            "HOME=/root",
            "TERM=xterm-256color",
            "LANG=C.UTF-8",
            "LC_ALL=C.UTF-8",
            "USER=root",
            "SHELL=/bin/bash",
            "TMPDIR=/tmp",
            "PENG_SOCKET=$SOCKET_PATH",
            "PENG_DATA=$APP_FILES/data"
        )
    }
    
    /**
     * 检查 Ubuntu 环境是否可用
     */
    fun isAvailable(): Boolean {
        val proot = File(PROOT_PATH)
        val ubuntuRoot = File(UBUNTU_ROOTFS)
        
        if (!proot.exists() || !ubuntuRoot.exists()) {
            Log.w(TAG, "Ubuntu environment not ready: proot=${proot.exists()}, rootfs=${ubuntuRoot.exists()}")
            return false
        }
        
        // 测试执行一个简单命令
        val result = execute("echo test", 5)
        val available = result.exitCode == 0 && result.stdout.contains("test")
        
        Log.i(TAG, "Ubuntu available: $available")
        return available
    }
    
    /**
     * 启动 peng-daemon
     */
    fun startDaemon(): Process? {
        val daemonPath = "$UBUNTU_ROOTFS/root/peng-daemon"
        val daemonFile = File(daemonPath)
        
        if (!daemonFile.exists()) {
            Log.e(TAG, "peng-daemon not found at $daemonPath")
            return null
        }
        
        val prootArgs = buildProotArgs(arrayOf(
            "/root/peng-daemon"
        ))
        
        return try {
            Log.i(TAG, "Starting peng-daemon...")
            Runtime.getRuntime().exec(prootArgs, buildEnv())
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start daemon", e)
            null
        }
    }
    
    /**
     * 安装 Python 包
     */
    fun installPythonPackage(packageName: String): ExecutionResult {
        return execute("pip3 install $packageName", 300)
    }
    
    /**
     * 运行 Python 脚本
     */
    fun runPython(script: String, timeoutSeconds: Long = 60): ExecutionResult {
        val tempScript = File(context.cacheDir, "temp_script_${System.currentTimeMillis()}.py")
        tempScript.writeText(script)
        
        val result = execute("python3 ${tempScript.absolutePath}", timeoutSeconds)
        
        tempScript.delete()
        return result
    }
    
    /**
     * 运行 Node.js 脚本
     */
    fun runNodeJs(script: String, timeoutSeconds: Long = 60): ExecutionResult {
        val tempScript = File(context.cacheDir, "temp_script_${System.currentTimeMillis()}.js")
        tempScript.writeText(script)
        
        val result = execute("node ${tempScript.absolutePath}", timeoutSeconds)
        
        tempScript.delete()
        return result
    }
    
    /**
     * 使用 ffmpeg 处理视频
     */
    fun ffmpeg(args: List<String>, timeoutSeconds: Long = 300): ExecutionResult {
        return execute("ffmpeg ${args.joinToString(" ")}", timeoutSeconds)
    }
}