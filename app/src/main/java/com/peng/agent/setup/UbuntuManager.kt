package com.peng.agent.setup

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
import java.io.*
import java.util.zip.GZIPInputStream
import java.util.zip.ZipInputStream

/**
 * Ubuntu 环境管理器
 * 负责初始化和管理 Ubuntu proot 环境
 */
object UbuntuManager {
    
    private const val TAG = "UbuntuManager"
    
    // 路径常量
    const val APP_FILES = "/data/data/com.peng.agent/files"
    const val UBUNTU_ROOTFS = "$APP_FILES/ubuntu-rootfs"
    const val PROOT_PATH = "$APP_FILES/bin/proot"
    const val SOCKET_PATH = "$APP_FILES/peng.sock"
    
    // 状态
    sealed class SetupState {
        object NotStarted : SetupState()
        object ExtractingProot : SetupState()
        object ExtractingUbuntu : SetupState()
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
        val prootFile = File(PROOT_PATH)
        val ubuntuDir = File(UBUNTU_ROOTFS)
        return prootFile.exists() && ubuntuDir.exists() && ubuntuDir.listFiles()?.isNotEmpty() == true
    }
    
    /**
     * 执行首次安装流程
     */
    suspend fun performSetup(
        context: Context,
        onProgress: (SetupState) -> Unit
    ): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            // Step 1: 安装 proot
            currentState = SetupState.ExtractingProot
            onProgress(currentState)
            Log.i(TAG, "Step 1: Installing proot...")
            installProot(context)
            
            // Step 2: 解压 Ubuntu rootfs
            currentState = SetupState.ExtractingUbuntu
            onProgress(currentState)
            Log.i(TAG, "Step 2: Extracting Ubuntu rootfs...")
            extractUbuntuRootfs(context)
            
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
     * 安装 proot
     */
    private fun installProot(context: Context) {
        val prootFile = File(PROOT_PATH)
        
        // 如果已存在，跳过
        if (prootFile.exists()) {
            Log.i(TAG, "proot already exists, skipping")
            return
        }
        
        // 创建目标目录
        prootFile.parentFile?.mkdirs()
        
        try {
            // 从 assets 复制
            context.assets.open("proot").use { input ->
                prootFile.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
            prootFile.setExecutable(true)
            Log.i(TAG, "proot installed to ${prootFile.absolutePath}")
        } catch (e: FileNotFoundException) {
            Log.w(TAG, "proot not found in assets")
            throw RuntimeException("proot binary not found in assets")
        }
    }
    
    /**
     * 解压 Ubuntu rootfs
     */
    private fun extractUbuntuRootfs(context: Context) {
        val targetDir = File(UBUNTU_ROOTFS)
        
        // 如果已存在，跳过
        if (targetDir.exists() && targetDir.listFiles()?.isNotEmpty() == true) {
            Log.i(TAG, "Ubuntu rootfs already exists, skipping extraction")
            return
        }
        
        targetDir.mkdirs()
        
        try {
            val rootfs = context.assets.open("ubuntu-rootfs.tar.gz")
            extractTarGz(rootfs, targetDir)
            Log.i(TAG, "Ubuntu rootfs extracted to ${targetDir.absolutePath}")
        } catch (e: FileNotFoundException) {
            Log.w(TAG, "Ubuntu rootfs not found in assets")
            // 创建基本的目录结构
            createBasicUbuntuStructure(targetDir)
        }
    }
    
    /**
     * 创建基本的 Ubuntu 目录结构
     */
    private fun createBasicUbuntuStructure(targetDir: File) {
        val directories = listOf(
            "bin", "sbin", "usr/bin", "usr/lib", "usr/share",
            "lib", "lib64", "etc", "var", "tmp", "root", "home",
            "dev", "proc", "sys", "run"
        )
        
        directories.forEach { dir ->
            File(targetDir, dir).mkdirs()
        }
        
        // 创建基本的 /etc/os-release
        File(targetDir, "etc/os-release").writeText("""
            PRETTY_NAME="PengAgent Ubuntu 22.04"
            NAME="Ubuntu"
            VERSION_ID="22.04"
            VERSION="22.04 (Jammy Jellyfish)"
            ID=ubuntu
            ID_LIKE=debian
            HOME_URL="https://www.ubuntu.com/"
        """.trimIndent())
        
        Log.i(TAG, "Created basic Ubuntu structure")
    }
    
    /**
     * 解压 tar.gz 文件
     */
    private fun extractTarGz(inputStream: InputStream, targetDir: File) {
        // 使用 Apache Commons Compress
        val gzip = GZIPInputStream(inputStream)
        val tar = TarArchiveInputStream(gzip)
        
        var entry = tar.nextTarEntry
        while (entry != null) {
            val file = File(targetDir, entry.name)
            
            if (entry.isDirectory) {
                file.mkdirs()
            } else {
                file.parentFile?.mkdirs()
                file.outputStream().use { output ->
                    tar.copyTo(output)
                }
                // 保留权限
                val mode = entry.mode
                file.setExecutable(mode and 0b000000111 != 0)
                file.setReadable(mode and 0b000100000 != 0)
                file.setWritable(mode and 0b000010000 != 0)
            }
            entry = tar.nextTarEntry
        }
        
        gzip.close()
    }
}