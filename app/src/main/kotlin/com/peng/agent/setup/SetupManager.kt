package com.peng.agent.setup

import android.content.Context
import android.os.Build
import android.os.Environment
import android.util.Log
import java.io.File

/**
 * 首次设置管理器 — 负责应用首次启动时的初始化流程
 *
 * 流程：
 * 1. 检查存储权限
 * 2. 初始化数据目录
 * 3. 初始化后端
 */
object SetupManager {

    private const val TAG = "SetupManager"
    private const val PREFS_NAME = "peng_setup"
    private const val KEY_SETUP_COMPLETE = "setup_complete"
    private const val KEY_SETUP_VERSION = "setup_version"

    private const val CURRENT_SETUP_VERSION = 1

    // ── Public API ─────────────────────────────────────────────────────────

    fun isSetupComplete(context: Context): Boolean {
        val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val complete = prefs.getBoolean(KEY_SETUP_COMPLETE, false)
        val version = prefs.getInt(KEY_SETUP_VERSION, 0)
        return complete && version >= CURRENT_SETUP_VERSION
    }

    fun hasStoragePermission(): Boolean {
        return if (Build.VERSION.SDK_INT >= 30) {
            Environment.isExternalStorageManager()
        } else {
            true // Below API 30, WRITE_EXTERNAL_STORAGE is granted at install time
        }
    }

    suspend fun performFirstTimeSetup(context: Context): Boolean {
        return try {
            Log.i(TAG, "🔧 开始首次设置...")

            // Step 1: Ensure data directories
            ensureDataDirectories()

            // Step 2: Ensure default config
            ensureDefaultConfig(context)

            // Step 3: Mark setup complete
            markSetupComplete(context)

            Log.i(TAG, "✅ 首次设置完成")
            true
        } catch (e: Exception) {
            Log.e(TAG, "❌ 首次设置失败", e)
            false
        }
    }

    // ── Internal ───────────────────────────────────────────────────────────

    private fun ensureDataDirectories() {
        val baseDir = "/sdcard/peng-agent"
        val subDirs = listOf(
            "sessions", "logs", "skills", "knowledge",
            "canvas", "scripts", "models", "tool-results"
        )

        File(baseDir).mkdirs()
        for (subDir in subDirs) {
            val dir = File(baseDir, subDir)
            if (!dir.exists()) {
                val created = dir.mkdirs()
                Log.i(TAG, "创建目录: ${dir.absolutePath}, 成功=$created")
            }
        }
    }

    private fun ensureDefaultConfig(context: Context) {
        val envFile = File("/sdcard/peng-agent/.env")
        if (!envFile.exists()) {
            envFile.writeText(
                """
                |# 鹏Agent 配置文件
                |# 首次安装自动生成，请在App设置页面修改
                |
                |# AI模型配置
                |PENG_AGENT_MODEL=deepseek-v4-flash
                |PENG_AGENT_API_BASE=https://api.deepseek.com/v1
                |PENG_AGENT_API_KEY=
                |PENG_AGENT_MAX_TOKENS=4096
                |PENG_AGENT_TEMPERATURE=0.7
                |PENG_AGENT_TOP_P=0.95
                |
                |# 子Agent配置
                |PENG_AGENT_WORKER_MODEL=
                |PENG_AGENT_WORKER_API_BASE=
                |PENG_AGENT_WORKER_API_KEY=
                """.trimMargin()
            )
            Log.i(TAG, "✅ 默认.env已创建")
        }
    }

    private fun markSetupComplete(context: Context) {
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_SETUP_COMPLETE, true)
            .putInt(KEY_SETUP_VERSION, CURRENT_SETUP_VERSION)
            .apply()
    }
}
