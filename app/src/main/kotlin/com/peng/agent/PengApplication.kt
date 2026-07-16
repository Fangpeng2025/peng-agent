package com.peng.agent

import android.app.Application
import android.content.Intent
import android.util.Log
import com.peng.agent.client.IpcBackendClient
import com.peng.agent.daemon.DaemonService
import com.peng.agent.setup.UbuntuManager

/**
 * 鹏 Agent 应用
 * 
 * 架构：Ubuntu 环境作为核心运行环境
 * - 所有 AI Agent 逻辑运行在 Ubuntu (proot) 中
 * - Android 端作为纯 UI 客户端
 * - 通过 Unix Socket IPC 通信
 */
class PengApplication : Application() {

    companion object {
        private const val TAG = "PengApplication"
        
        var appContext: Application? = null
            private set
        
        private var _client: IpcBackendClient? = null
        val client: IpcBackendClient
            get() = _client ?: error("BackendClient not initialized")
    }

    override fun onCreate() {
        super.onCreate()
        appContext = this
        
        Log.i(TAG, "🚀 PengAgent 启动...")
        
        // 初始化 IPC 客户端
        initializeIpcClient()
        
        // 检查并启动 Ubuntu 环境
        checkAndStartUbuntu()
    }
    
    /**
     * 初始化 IPC 客户端
     */
    private fun initializeIpcClient() {
        Log.i(TAG, "初始化 IPC 客户端...")
        _client = IpcBackendClient(this)
    }
    
    /**
     * 检查并启动 Ubuntu 环境
     */
    private fun checkAndStartUbuntu() {
        if (UbuntuManager.isReady()) {
            Log.i(TAG, "✅ Ubuntu 环境已就绪，启动 daemon...")
            startDaemonService()
            
            // 连接到 daemon
            client.connect()
        } else {
            Log.i(TAG, "⏳ Ubuntu 环境未就绪，等待首次安装...")
        }
    }
    
    /**
     * 启动 Daemon 服务
     */
    private fun startDaemonService() {
        try {
            val intent = Intent(this, DaemonService::class.java)
            startForegroundService(intent)
            Log.i(TAG, "✅ Daemon 服务已启动")
        } catch (e: Exception) {
            Log.e(TAG, "❌ 启动 Daemon 服务失败", e)
        }
    }

    override fun onTerminate() {
        super.onTerminate()
        try {
            client.shutdown()
        } catch (_: Exception) {}
        Log.i(TAG, "Application terminated")
    }
}