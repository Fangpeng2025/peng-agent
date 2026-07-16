package com.peng.agent.daemon

import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.util.Log
import com.peng.agent.ubuntu.UbuntuRuntime
import java.io.File

/**
 * Daemon 服务
 * 保持 peng-daemon 进程运行
 */
class DaemonService : Service() {
    
    companion object {
        private const val TAG = "DaemonService"
        const val SOCKET_PATH = "/data/data/com.peng.agent/files/peng.sock"
    }
    
    private var daemonProcess: Process? = null
    private var ubuntuRuntime: UbuntuRuntime? = null
    private var isRunning = false
    
    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "DaemonService created")
        ubuntuRuntime = UbuntuRuntime(this)
    }
    
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.i(TAG, "DaemonService started")
        
        if (!isRunning) {
            startDaemon()
        }
        
        // 确保服务不被杀死
        return START_STICKY
    }
    
    override fun onBind(intent: Intent?): IBinder? = null
    
    override fun onDestroy() {
        Log.i(TAG, "DaemonService destroyed")
        stopDaemon()
        super.onDestroy()
    }
    
    /**
     * 启动 daemon 进程
     */
    private fun startDaemon() {
        if (daemonProcess != null && daemonProcess!!.isAlive) {
            Log.i(TAG, "Daemon already running")
            return
        }
        
        val runtime = ubuntuRuntime ?: return
        
        // 检查 Ubuntu 环境是否就绪
        if (!runtime.isAvailable()) {
            Log.e(TAG, "Ubuntu environment not ready")
            return
        }
        
        // 清理旧的 socket
        val socketFile = File(SOCKET_PATH)
        if (socketFile.exists()) {
            socketFile.delete()
        }
        
        // 启动 daemon
        daemonProcess = runtime.startDaemon()
        
        if (daemonProcess != null) {
            isRunning = true
            Log.i(TAG, "Daemon started successfully")
            
            // 监控进程状态
            Thread {
                try {
                    val exitCode = daemonProcess?.waitFor() ?: -1
                    Log.w(TAG, "Daemon exited with code $exitCode")
                    isRunning = false
                    
                    // 如果异常退出，尝试重启
                    if (exitCode != 0) {
                        Thread.sleep(3000)
                        if (isRunning) {
                            Log.i(TAG, "Attempting to restart daemon...")
                            startDaemon()
                        }
                    }
                } catch (e: InterruptedException) {
                    Log.i(TAG, "Daemon monitor interrupted")
                }
            }.start()
        } else {
            Log.e(TAG, "Failed to start daemon")
        }
    }
    
    /**
     * 停止 daemon 进程
     */
    private fun stopDaemon() {
        daemonProcess?.let { process ->
            if (process.isAlive) {
                Log.i(TAG, "Stopping daemon...")
                process.destroy()
                
                try {
                    process.waitFor(5, java.util.concurrent.TimeUnit.SECONDS)
                } catch (e: Exception) {
                    process.destroyForcibly()
                }
            }
        }
        
        daemonProcess = null
        isRunning = false
        Log.i(TAG, "Daemon stopped")
    }
    
    /**
     * 检查 daemon 是否运行中
     */
    fun isDaemonRunning(): Boolean {
        return daemonProcess?.isAlive == true && File(SOCKET_PATH).exists()
    }
}