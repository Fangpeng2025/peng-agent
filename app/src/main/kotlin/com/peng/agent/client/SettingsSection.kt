package com.peng.agent.client

enum class SettingsSection(val title: String, val icon: String) {
    BACKEND_STATUS("后端环境", "🖥️"),
    MODEL_CONNECTION("模型设置", "🔌"),
    GENERATION_PARAMS("生成参数", "🎲"),
    CONTEXT_MANAGEMENT("上下文管理", "📦"),
    AGENT_BEHAVIOR("Agent 行为", "🤖"),
    PERSONAL_INFO("个人信息", "👤")
}
