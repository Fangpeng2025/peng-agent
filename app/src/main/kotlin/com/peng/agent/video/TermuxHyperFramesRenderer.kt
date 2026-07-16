package com.peng.agent.video

import android.util.Log
import org.json.JSONObject
import java.io.File

/**
 * Termux HyperFrames 视频渲染器
 *
 * 使用 HyperFrames 模式渲染视频：
 * - 将帧序列合成为视频
 * - 支持 Termux ffmpeg 后端
 */
object TermuxHyperFramesRenderer {

    private const val TAG = "HyperFrames"
    private const val FRAMES_DIR = "/sdcard/peng-agent/video/frames"
    private const val OUTPUT_DIR = "/sdcard/peng-agent/video"

    /**
     * 渲染视频
     *
     * @param args JSON参数，包含:
     *   - frames_dir: 帧目录路径
     *   - output: 输出路径
     *   - fps: 帧率 (默认 30)
     *   - codec: 编码器 (默认 libx264)
     *   - quality: 质量 (默认 medium)
     */
    fun render(args: JSONObject): String {
        return try {
            val framesDir = args.optString("frames_dir", FRAMES_DIR)
            val outputPath = args.optString("output", "$OUTPUT_DIR/output_${System.currentTimeMillis()}.mp4")
            val fps = args.optInt("fps", 30)
            val codec = args.optString("codec", "libx264")
            val quality = args.optString("quality", "medium")

            Log.i(TAG, "🎬 开始渲染视频: $framesDir → $outputPath")

            // Ensure output directory exists
            File(OUTPUT_DIR).mkdirs()

            // Check if frames exist
            val frames = File(framesDir)
            if (!frames.exists() || !frames.isDirectory) {
                return """{"success":false,"error":"帧目录不存在: $framesDir"}"""
            }

            val frameFiles = frames.listFiles()?.filter {
                it.name.endsWith(".png") || it.name.endsWith(".jpg")
            }?.sortedBy { it.name } ?: emptyList()

            if (frameFiles.isEmpty()) {
                return """{"success":false,"error":"帧目录为空: $framesDir"}"""
            }

            Log.i(TAG, "🎬 找到 ${frameFiles.size} 帧")

            // Build ffmpeg command
            val crf = when (quality) {
                "high" -> "18"
                "medium" -> "23"
                "low" -> "28"
                else -> "23"
            }

            val ffmpegCmd = buildString {
                append("ffmpeg -y ")
                append("-framerate $fps ")
                append("-i ${framesDir}/frame_%04d.png ")
                append("-c:v $codec ")
                append("-crf $crf ")
                append("-preset medium ")
                append("-pix_fmt yuv420p ")
                append("\"$outputPath\"")
            }

            // Execute ffmpeg
            val process = Runtime.getRuntime().exec(
                arrayOf("sh", "-c", ffmpegCmd)
            )

            val output = process.inputStream.bufferedReader().readText()
            val error = process.errorStream.bufferedReader().readText()
            val exitCode = process.waitFor()

            if (exitCode == 0) {
                val outputFile = File(outputPath)
                val sizeMb = if (outputFile.exists()) outputFile.length() / (1024.0 * 1024.0) else 0.0
                Log.i(TAG, "✅ 视频渲染完成: $outputPath (${String.format("%.1f", sizeMb)}MB)")

                """{"success":true,"output":"$outputPath","size_mb":${String.format("%.1f", sizeMb)},"frames":${frameFiles.size},"fps":$fps}"""
            } else {
                Log.e(TAG, "❌ ffmpeg执行失败: $error")
                """{"success":false,"error":"ffmpeg执行失败 (exit $exitCode): ${error.take(500)}"}"""
            }
        } catch (e: Exception) {
            Log.e(TAG, "❌ 视频渲染异常", e)
            """{"success":false,"error":"${e.message}"}"""
        }
    }
}
