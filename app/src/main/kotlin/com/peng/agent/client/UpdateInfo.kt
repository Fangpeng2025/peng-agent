package com.peng.agent.client

data class UpdateInfo(
    val hasUpdate: Boolean,
    val currentVersion: String,
    val latestVersion: String,
    val latestVersionCode: Int,
    val apkUrl: String,
    val serverUrl: String,
    val changelog: String,
    val apkSize: Long,
    val serverSize: Long
) {
    fun formatSize(bytes: Long): String = when {
        bytes <= 0 -> ""
        bytes < 1024 -> "${bytes}B"
        bytes < 1048576 -> "%.1fKB".format(bytes / 1024.0)
        else -> "%.1fMB".format(bytes / 1048576.0)
    }
}
