package com.peng.agent.setup

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.*
import java.util.zip.ZipInputStream

/**
 * Ubuntu 环境管理器
 * 使用 Termux proot-distro 动态安装 Ubuntu
 */
object UbuntuManager {
    
    private const val TAG = "UbuntuManager"
    
    // 路径常量
    const val APP_FILES = "/data/data/com.peng.agent/files"
    const val TERMUX_PREFIX = "$APP_FILES/usr"
    const val PROOT_DISTRO_ROOT = "$TERMUX_PREFIX/var/proot-distro/installed-rootfs"
    const val UBUNTU_ROOTFS = "$PROOT_DISTRO_ROOT/ubuntu"
    const val SOCKET_PATH = "$APP_FILES/peng.sock"
    
    // 状态
    sealed class SetupState {
        object NotStarted : SetupState()
        object ExtractingBootstrap : SetupState()
        object InstallingProotDistro : SetupState()
        object DownloadingUbuntu : SetupState()
        object InstallingPackages : SetupState()
        object InstallingDaemon : SetupState()
        object Ready : SetupState()
        data class Error(val message: String) : SetupState()
        data class Progress(val percent: Int, val message: String) : SetupState()
    }
    
    private var currentState: SetupState = SetupState.NotStarted
    
    /**
     * 获取当前状态
     */
    fun getState(): SetupState = currentState
    
    /**
     * 检查环境是否已就绪
     */
    fun isReady(): Boolean {
        val ubuntuDir = File(UBUNTU_ROOTFS)
        val daemonFile = File("$UBUNTU_ROOTFS/root/peng-daemon")
        return ubuntuDir.exists() && daemonFile.exists()
    }
    
    // ========================================================================
    // 带实时输出的安装方法
    // ========================================================================
    
    /**
     * 解压 Bootstrap（带输出）
     */
    fun extractBootstrapWithOutput(context: Context, onOutput: (String) -> Unit): Boolean {
        val targetDir = File(TERMUX_PREFIX)
        
        if (targetDir.exists() && File("$TERMUX_PREFIX/bin/bash").exists()) {
            onOutput("Bootstrap 已存在，跳过解压")
            return true
        }
        
        targetDir.mkdirs()
        
        try {
            val bootstrap = context.assets.open("bootstrap-aarch64.zip")
            val fileCount = unzipWithOutput(bootstrap, targetDir, onOutput)
            onOutput("已解压 $fileCount 个文件")
            return true
        } catch (e: Exception) {
            onOutput("错误: ${e.message}")
            return false
        }
    }
    
    /**
     * 安装 proot-distro（带输出）
     */
    fun installProotDistroWithOutput(context: Context, onOutput: (String) -> Unit): Boolean {
        val prootDistro = File("$TERMUX_PREFIX/bin/proot-distro")
        
        if (prootDistro.exists()) {
            onOutput("proot-distro 已安装")
            return true
        }
        
        val result = executeInTermuxWithOutput("pkg install -y proot-distro", onOutput)
        return result.exitCode == 0
    }
    
    /**
     * 安装 Ubuntu（带输出）
     */
    fun installUbuntuWithOutput(context: Context, onOutput: (String) -> Unit): Boolean {
        val ubuntuDir = File(UBUNTU_ROOTFS)
        
        if (ubuntuDir.exists()) {
            onOutput("Ubuntu 已安装")
            return true
        }
        
        val result = executeInTermuxWithOutput("proot-distro install ubuntu", onOutput)
        return result.exitCode == 0
    }
    
    /**
     * 安装软件包（带输出）
     */
    fun installPackagesWithOutput(context: Context, onOutput: (String) -> Unit): Boolean {
        val packages = "python3 python3-pip nodejs npm ffmpeg git curl wget sqlite3 ca-certificates"
        
        // 先更新
        executeInUbuntuWithOutput("apt update", onOutput)
        
        // 安装包
        val result = executeInUbuntuWithOutput("apt install -y $packages", onOutput)
        
        // 安装 Python 包
        executeInUbuntuWithOutput("pip3 install numpy pandas requests Pillow || true", onOutput)
        
        return result.exitCode == 0
    }
    
    /**
     * 安装 daemon（带输出）
     */
    fun installDaemonWithOutput(context: Context, onOutput: (String) -> Unit): Boolean {
        val daemonTarget = File("$UBUNTU_ROOTFS/root/peng-daemon")
        daemonTarget.parentFile?.mkdirs()
        
        try {
            context.assets.open("peng-daemon").use { input ->
                daemonTarget.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
            daemonTarget.setExecutable(true)
            onOutput("peng-daemon 已安装到 ${daemonTarget.absolutePath}")
            return true
        } catch (e: FileNotFoundException) {
            onOutput("警告: peng-daemon 未找到在 assets 中")
            // 尝试从 data 目录查找
            val localDaemon = File("$APP_FILES/peng-daemon")
            if (localDaemon.exists()) {
                localDaemon.copyTo(daemonTarget, overwrite = true)
                daemonTarget.setExecutable(true)
                onOutput("peng-daemon 已从本地复制")
                return true
            }
            return false
        } catch (e: Exception) {
            onOutput("错误: ${e.message}")
            return false
        }
    }
    
    // ========================================================================
    // 执行命令
    // ========================================================================
    
    /**
     * 在 Termux 环境中执行命令（带实时输出）
     */
    private fun executeInTermuxWithOutput(command: String, onOutput: (String) -> Unit): ExecutionResult {
        val bashPath = "$TERMUX_PREFIX/bin/bash"
        val env = arrayOf(
            "PATH=$TERMUX_PREFIX/bin:/system/bin",
            "HOME=$TERMUX_PREFIX/home",
            "TERM=xterm-256color",
            "LANG=C.UTF-8",
            "LD_LIBRARY_PATH=$TERMUX_PREFIX/lib",
            "TMPDIR=$TERMUX_PREFIX/tmp"
        )
        
        return try {
            val process = Runtime.getRuntime().exec(
                arrayOf(bashPath, "-c", command),
                env
            )
            
            // 读取标准输出
            Thread {
                process.inputStream.bufferedReader().use { reader ->
                    var line = reader.readLine()
                    while (line != null) {
                        onOutput(line)
                        line = reader.readLine()
                    }
                }
            }.start()
            
            // 读取错误输出
            Thread {
                process.errorStream.bufferedReader().use { reader ->
                    var line = reader.readLine()
                    while (line != null) {
                        onOutput("[ERR] $line")
                        line = reader.readLine()
                    }
                }
            }.start()
            
            val exitCode = process.waitFor()
            ExecutionResult(exitCode, "", "")
        } catch (e: Exception) {
            ExecutionResult(-1, "", e.message ?: "Unknown error")
        }
    }
    
    /**
     * 在 Ubuntu 环境中执行命令（通过 proot-distro，带实时输出）
     */
    private fun executeInUbuntuWithOutput(command: String, onOutput: (String) -> Unit): ExecutionResult {
        val prootCommand = "proot-distro run ubuntu -- $command"
        return executeInTermuxWithOutput(prootCommand, onOutput)
    }
    
    // ========================================================================
    // 原有的协程式安装流程（保留兼容）
    // ========================================================================
    
    /**
     * 执行首次安装流程
     */
    suspend fun performSetup(
        context: Context,
        onProgress: (SetupState) -> Unit
    ): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            // Step 1: 解压 Termux Bootstrap
            currentState = SetupState.ExtractingBootstrap
            onProgress(currentState)
            Log.i(TAG, "Step 1: Extracting Termux Bootstrap...")
            extractBootstrap(context)
            
            // Step 2: 安装 proot-distro（如果没有）
            currentState = SetupState.InstallingProotDistro
            onProgress(currentState)
            Log.i(TAG, "Step 2: Checking proot-distro...")
            ensureProotDistro()
            
            // Step 3: 使用 proot-distro 安装 Ubuntu
            currentState = SetupState.DownloadingUbuntu
            onProgress(currentState)
            Log.i(TAG, "Step 3: Installing Ubuntu via proot-distro...")
            installUbuntu()
            
            // Step 4: 安装必要的包
            currentState = SetupState.InstallingPackages
            onProgress(currentState)
            Log.i(TAG, "Step 4: Installing packages...")
            installPackages()
            
            // Step 5: 安装 peng-daemon
            currentState = SetupState.InstallingDaemon
            onProgress(currentState)
            Log.i(TAG, "Step 5: Installing peng-daemon...")
            installDaemon(context)
            
            // 完成
            currentState = SetupState.Ready
            onProgress(currentState)
            Log.i(TAG, "Setup complete!")
            
            Result.success(Unit)
        } catch (e: Exception) {
            currentState = SetupState.Error(e.message ?: "Unknown error")
            onProgress(currentState)
            Log.e(TAG, "Setup failed", e)
            Result.failure(e)
        }
    }
    
    /**
     * 解压 Termux Bootstrap
     */
    private fun extractBootstrap(context: Context) {
        val targetDir = File(TERMUX_PREFIX)
        
        if (targetDir.exists() && File("$TERMUX_PREFIX/bin/bash").exists()) {
            Log.i(TAG, "Bootstrap already extracted, skipping")
            return
        }
        
        targetDir.mkdirs()
        
        try {
            val bootstrap = context.assets.open("bootstrap-aarch64.zip")
            unzip(bootstrap, targetDir)
            Log.i(TAG, "Bootstrap extracted to ${targetDir.absolutePath}")
        } catch (e: FileNotFoundException) {
            Log.w(TAG, "Bootstrap not found in assets")
            throw RuntimeException("Bootstrap not found. Please include bootstrap-aarch64.zip in assets.")
        }
    }
    
    /**
     * 确保 proot-distro 已安装
     */
    private fun ensureProotDistro() {
        val prootDistro = File("$TERMUX_PREFIX/bin/proot-distro")
        
        if (prootDistro.exists()) {
            Log.i(TAG, "proot-distro already installed")
            return
        }
        
        val result = executeInTermux("pkg install -y proot-distro")
        if (result.exitCode != 0) {
            throw RuntimeException("Failed to install proot-distro: ${result.stderr}")
        }
        
        Log.i(TAG, "proot-distro installed")
    }
    
    /**
     * 使用 proot-distro 安装 Ubuntu
     */
    private fun installUbuntu() {
        val ubuntuDir = File(UBUNTU_ROOTFS)
        
        if (ubuntuDir.exists()) {
            Log.i(TAG, "Ubuntu already installed")
            return
        }
        
        val result = executeInTermux("proot-distro install ubuntu")
        if (result.exitCode != 0) {
            throw RuntimeException("Failed to install Ubuntu: ${result.stderr}")
        }
        
        Log.i(TAG, "Ubuntu installed via proot-distro")
    }
    
    /**
     * 安装必要的包
     */
    private fun installPackages() {
        val packages = "python3 python3-pip nodejs npm ffmpeg git curl wget sqlite3 ca-certificates"
        
        val result = executeInUbuntu("apt update && apt install -y $packages")
        if (result.exitCode != 0) {
            Log.w(TAG, "Some packages may have failed to install: ${result.stderr}")
        }
        
        executeInUbuntu("pip3 install numpy pandas requests Pillow || true")
        
        Log.i(TAG, "Packages installed")
    }
    
    /**
     * 安装 peng-daemon
     */
    private fun installDaemon(context: Context) {
        val daemonTarget = File("$UBUNTU_ROOTFS/root/peng-daemon")
        daemonTarget.parentFile?.mkdirs()
        
        try {
            context.assets.open("peng-daemon").use { input ->
                daemonTarget.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
            daemonTarget.setExecutable(true)
            Log.i(TAG, "peng-daemon installed to ${daemonTarget.absolutePath}")
        } catch (e: FileNotFoundException) {
            Log.w(TAG, "peng-daemon not found in assets")
        }
    }
    
    /**
     * 在 Termux 环境中执行命令
     */
    private fun executeInTermux(command: String): ExecutionResult {
        val bashPath = "$TERMUX_PREFIX/bin/bash"
        val env = arrayOf(
            "PATH=$TERMUX_PREFIX/bin:/system/bin",
            "HOME=$TERMUX_PREFIX/home",
            "TERM=xterm-256color",
            "LANG=C.UTF-8",
            "LD_LIBRARY_PATH=$TERMUX_PREFIX/lib"
        )
        
        return try {
            val process = Runtime.getRuntime().exec(
                arrayOf(bashPath, "-c", command),
                env
            )
            
            val stdout = process.inputStream.bufferedReader().readText()
            val stderr = process.errorStream.bufferedReader().readText()
            val exitCode = process.waitFor()
            
            ExecutionResult(exitCode, stdout, stderr)
        } catch (e: Exception) {
            ExecutionResult(-1, "", e.message ?: "Unknown error")
        }
    }
    
    /**
     * 在 Ubuntu 环境中执行命令（通过 proot-distro）
     */
    private fun executeInUbuntu(command: String): ExecutionResult {
        val prootCommand = "proot-distro run ubuntu -- $command"
        return executeInTermux(prootCommand)
    }
    
    // ========================================================================
    // 工具方法
    // ========================================================================
    
    /**
     * 解压 ZIP 文件
     */
    private fun unzip(inputStream: InputStream, targetDir: File) {
        ZipInputStream(inputStream).use { zip ->
            var entry = zip.nextEntry
            while (entry != null) {
                val file = File(targetDir, entry.name)
                
                if (entry.isDirectory) {
                    file.mkdirs()
                } else {
                    file.parentFile?.mkdirs()
                    file.outputStream().use { output ->
                        zip.copyTo(output)
                    }
                    if (entry.name.startsWith("bin/") || 
                        entry.name.startsWith("libexec/") ||
                        entry.name.endsWith(".sh")) {
                        file.setExecutable(true)
                    }
                }
                entry = zip.nextEntry
            }
        }
    }
    
    /**
     * 解压 ZIP 文件（带输出）
     */
    private fun unzipWithOutput(inputStream: InputStream, targetDir: File, onOutput: (String) -> Unit): Int {
        var count = 0
        ZipInputStream(inputStream).use { zip ->
            var entry = zip.nextEntry
            while (entry != null) {
                val file = File(targetDir, entry.name)
                
                if (entry.isDirectory) {
                    file.mkdirs()
                } else {
                    file.parentFile?.mkdirs()
                    file.outputStream().use { output ->
                        zip.copyTo(output)
                    }
                    if (entry.name.startsWith("bin/") || 
                        entry.name.startsWith("libexec/") ||
                        entry.name.endsWith(".sh")) {
                        file.setExecutable(true)
                    }
                    count++
                    if (count % 100 == 0) {
                        onOutput("已解压 $count 个文件...")
                    }
                }
                entry = zip.nextEntry
            }
        }
        return count
    }
    
    /**
     * 执行结果
     */
    data class ExecutionResult(
        val exitCode: Int,
        val stdout: String,
        val stderr: String
    )
}