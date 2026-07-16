package com.peng.agent.termux

import android.content.Context
import android.util.Log
import java.io.File

/**
 * Termux 环境管理器 — 初始化和管理 Termux 执行环境
 *
 * 提供 shell 命令执行能力，包括：
 * - Termux 环境初始化
 * - Shell 命令执行
 * - Python 执行环境
 */
object TermuxEnvironment {

    private const val TAG = "TermuxEnvironment"
    private const val TERMUX_DIR = "/data/data/com.termux"
    private const val PENG_TERMUX_DIR = "/sdcard/peng-agent/termux"

    private var isInitialized = false

    // ── Public API ─────────────────────────────────────────────────────────

    suspend fun init(context: Context): Boolean {
        if (isInitialized) return true

        return try {
            Log.i(TAG, "🔧 初始化Termux环境...")

            // Check if Termux is installed
            val termuxInstalled = isTermuxInstalled(context)

            // Ensure our termux directory exists
            val termuxDir = File(PENG_TERMUX_DIR)
            if (!termuxDir.exists()) termuxDir.mkdirs()

            // Initialize shell environment
            initShellEnvironment(context)

            isInitialized = true
            Log.i(TAG, "✅ Termux环境就绪 (Termux已安装: $termuxInstalled)")
            true
        } catch (e: Exception) {
            Log.e(TAG, "❌ Termux环境初始化失败", e)
            false
        }
    }

    fun isReady(): Boolean = isInitialized

    fun executeShell(command: String, timeout: Long = 30L): String {
        if (!isInitialized) return """{"success":false,"error":"Termux环境未初始化"}"""

        return try {
            val process = Runtime.getRuntime().exec(
                arrayOf("sh", "-c", command)
            )
            val output = process.inputStream.bufferedReader().readText()
            val error = process.errorStream.bufferedReader().readText()
            process.waitFor()

            if (process.exitValue() == 0) {
                output.ifEmpty { "执行成功（无输出）" }
            } else {
                "执行失败 (exit ${process.exitValue()}): $error"
            }
        } catch (e: Exception) {
            "执行异常: ${e.message}"
        }
    }

    // ── Internal ───────────────────────────────────────────────────────────

    private fun isTermuxInstalled(context: Context): Boolean {
        return try {
            context.packageManager.getPackageInfo("com.termux", 0)
            true
        } catch (_: Exception) {
            false
        }
    }

    private fun initShellEnvironment(context: Context) {
        // Create helper scripts
        val scriptsDir = File(PENG_TERMUX_DIR, "scripts")
        scriptsDir.mkdirs()

        // Create a basic shell profile
        val profile = File(scriptsDir, "peng_profile.sh")
        if (!profile.exists()) {
            profile.writeText(
                """
                |#!/bin/sh
                |# 鹏Agent Shell 环境
                |export HOME=$PENG_TERMUX_DIR
                |export PATH=/system/bin:/system/xbin:${'$'}PATH
                |export PENG_AGENT=1
                """.trimMargin()
            )
        }
    }
}
