package com.peng.agent.service

import android.content.Context
import android.graphics.Bitmap
import android.util.Log
import org.json.JSONObject
import java.io.File

/**
 * OCR 管理器 — 提供文字识别能力
 *
 * 使用 ONNX Runtime 进行本地 OCR 识别。
 * 降级方案：返回提示用户安装 OCR 模型。
 */
object OCRManager {

    private const val TAG = "OCRManager"
    private const val MODEL_DIR = "/sdcard/peng-agent/models"

    private var isInitialized = false

    // ── Public API ─────────────────────────────────────────────────────────

    fun initialize(context: Context): Boolean {
        if (isInitialized) return true
        return try {
            val modelDir = File(MODEL_DIR)
            if (!modelDir.exists()) modelDir.mkdirs()
            isInitialized = true
            Log.i(TAG, "✅ OCR管理器已初始化")
            true
        } catch (e: Exception) {
            Log.e(TAG, "❌ OCR管理器初始化失败", e)
            false
        }
    }

    fun recognize(region: String): String {
        if (!isInitialized) {
            return """{"success":false,"error":"OCR未初始化"}"""
        }

        return try {
            // Parse region coordinates
            val args = if (region.isNotBlank()) JSONObject(region) else JSONObject()
            val x = args.optInt("x", 0)
            val y = args.optInt("y", 0)
            val width = args.optInt("width", 0)
            val height = args.optInt("height", 0)

            // TODO: Implement actual OCR using ONNX Runtime
            // For now, return a placeholder indicating OCR is available but model not loaded
            """{"success":false,"error":"OCR模型未加载，请先下载OCR模型","region":{"x":$x,"y":$y,"width":$width,"height":$height}}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    fun recognizeBitmap(bitmap: Bitmap): String {
        return try {
            // TODO: Implement bitmap OCR
            """{"success":false,"error":"OCR模型未加载"}"""
        } catch (e: Exception) {
            """{"success":false,"error":"${e.message}"}"""
        }
    }

    fun isModelAvailable(): Boolean {
        val modelFile = File(MODEL_DIR, "ocr_model.onnx")
        return modelFile.exists()
    }
}
