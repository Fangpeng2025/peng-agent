package com.peng.agent.setup

/**
 * 设置步骤枚举
 */
enum class SetupStep(val title: String, val description: String) {
    CHECKING("系统检查", "正在检查系统环境..."),
    REQUESTING_PERMISSION("权限请求", "正在请求必要权限..."),
    INITIALIZING_BACKEND("初始化后端", "正在初始化嵌入式后端..."),
    COMPLETE("设置完成", "所有初始化步骤已完成"),
    ERROR("设置失败", "初始化过程中出现错误")
}

/**
 * 设置状态数据类
 */
data class SetupState(
    val step: SetupStep = SetupStep.CHECKING,
    val progress: Int = 0,
    val needsUserAction: Boolean = false,
    val userActionMessage: String? = null,
    val errorMessage: String? = null,
    val bytesDownloaded: Long = 0L,
    val totalBytes: Long = 0L
) {
    val isError: Boolean get() = step == SetupStep.ERROR
    val isComplete: Boolean get() = step == SetupStep.COMPLETE
}
