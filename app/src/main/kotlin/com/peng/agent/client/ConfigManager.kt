package com.peng.agent.client

import android.content.Context
import android.util.Log
import com.google.gson.Gson
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Manages application configuration: persists via SharedPreferences,
 * syncs to the .env file consumed by the Rust backend, and exposes
 * the current config as a [StateFlow] for reactive UI.
 */
object ConfigManager {

    private const val TAG = "ConfigManager"
    private const val PREFS_NAME = "peng_config"
    private const val KEY_CONFIG = "app_config"
    private const val ENV_FILE_PATH = "/sdcard/peng-agent/.env"

    private val gson = Gson()

    private val _configFlow = MutableStateFlow(AppConfig())

    /** Reactive stream of the current [AppConfig]. Updated on every save. */
    val config: StateFlow<AppConfig> = _configFlow.asStateFlow()

    // ── Load ─────────────────────────────────────────────────────────────

    /**
     * Loads config from SharedPreferences. Falls back to defaults on any error.
     * Also reads the .env file for any values that may have been edited outside
     * the app (e.g. by the user or the Rust backend).
     */
    fun loadConfig(context: Context): AppConfig {
        try {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            val json = prefs.getString(KEY_CONFIG, null)
            if (json != null) {
                val fromPrefs = gson.fromJson(json, AppConfig::class.java)
                // Overlay with any .env values that exist (env takes precedence
                // so manual edits to .env are respected on reload)
                val merged = mergeWithEnvFile(fromPrefs)
                _configFlow.value = merged
                return merged
            }
        } catch (e: Exception) {
            Log.e(TAG, "加载配置失败", e)
        }

        // No saved prefs — try reading .env, otherwise return defaults
        val fromEnv = readEnvFile()
        if (fromEnv != null) {
            _configFlow.value = fromEnv
            return fromEnv
        }

        val defaults = AppConfig()
        _configFlow.value = defaults
        return defaults
    }

    /**
     * Suspend-friendly load that performs disk I/O off the main thread.
     */
    suspend fun loadConfigSuspend(context: Context): AppConfig =
        withContext(Dispatchers.IO) { loadConfig(context) }

    // ── Save ─────────────────────────────────────────────────────────────

    /**
     * Persists [config] to SharedPreferences and syncs the .env file
     * consumed by the Rust backend.
     */
    fun saveConfig(context: Context, config: AppConfig) {
        try {
            val json = gson.toJson(config)
            context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .edit()
                .putString(KEY_CONFIG, json)
                .apply()
            syncToEnvFile(config)
            _configFlow.value = config
            Log.i(TAG, "配置已保存: ${config.model}, apiKey长度=${config.apiKey.length}")
        } catch (e: Exception) {
            Log.e(TAG, "保存配置失败", e)
        }
    }

    /**
     * Suspend-friendly save that performs disk I/O off the main thread.
     */
    suspend fun saveConfigSuspend(context: Context, config: AppConfig) =
        withContext(Dispatchers.IO) { saveConfig(context, config) }

    // ── Update single key ────────────────────────────────────────────────

    /**
     * Updates a single config field identified by [key] with [value],
     * then persists the result. Returns the updated config.
     */
    fun updateConfig(context: Context, key: String, value: Any): AppConfig {
        val current = loadConfig(context)
        val updated = current.updateField(key, value)
        saveConfig(context, updated)
        return updated
    }

    /**
     * Suspend-friendly update for a single key.
     */
    suspend fun updateConfigSuspend(context: Context, key: String, value: Any): AppConfig =
        withContext(Dispatchers.IO) { updateConfig(context, key, value) }

    // ── .env sync ────────────────────────────────────────────────────────

    /**
     * Writes the config as `PENG_AGENT_*` environment variables into the
     * .env file, preserving any user-added custom lines.
     */
    private fun syncToEnvFile(config: AppConfig) {
        try {
            val envFile = File(ENV_FILE_PATH)
            Log.i(TAG, "syncToEnvFile: model=${config.model}, apiKey长度=${config.apiKey.length}")

            // Preserve custom (non-PENG_AGENT_, non-comment) lines
            val customLines = if (envFile.exists()) {
                envFile.readLines()
                    .map { it.trim() }
                    .filter { it.isNotEmpty() }
                    .filter { !it.startsWith("PENG_AGENT_") && !it.startsWith("#") }
            } else {
                emptyList()
            }

            val agentLines = buildList {
                if (config.apiKey.isNotEmpty()) {
                    add("PENG_AGENT_API_KEY=${config.apiKey}")
                }
                add("PENG_AGENT_MODEL=${config.model}")
                add("PENG_AGENT_API_BASE=${config.apiBase}")
                add("PENG_AGENT_MAX_TOKENS=${config.maxTokens}")
                add("PENG_AGENT_TEMPERATURE=${config.temperature}")
                add("PENG_AGENT_MAX_TURNS=${config.maxTurns}")
                add("PENG_AGENT_TOOL_TIMEOUT_SECS=${config.toolTimeoutSecs}")
                add("PENG_AGENT_STREAM_TIMEOUT_SECS=${config.streamTimeoutSecs}")
                add("PENG_AGENT_ABORT_ON_ERROR=${config.abortOnError}")
                if (config.workerModel.isNotEmpty()) {
                    add("PENG_AGENT_WORKER_MODEL=${config.workerModel}")
                }
                if (config.workerApiBase.isNotEmpty()) {
                    add("PENG_AGENT_WORKER_API_BASE=${config.workerApiBase}")
                }
                if (config.workerApiKey.isNotEmpty()) {
                    add("PENG_AGENT_WORKER_API_KEY=${config.workerApiKey}")
                }
                if (config.visionApiKey.isNotEmpty()) {
                    add("PENG_AGENT_VISION_API_KEY=${config.visionApiKey}")
                }
                add("PENG_AGENT_VISION_API_BASE=${config.visionApiBase}")
                add("PENG_AGENT_VISION_MODEL=${config.visionModel}")
            }

            val content = (agentLines + customLines).joinToString("\n") + "\n"
            envFile.writeText(content)
            Log.i(
                TAG,
                "已同步配置到 .env 文件: model=${config.model}, 保留${customLines.size}条自定义配置"
            )
        } catch (e: Exception) {
            Log.e(TAG, "同步 .env 文件失败", e)
        }
    }

    // ── .env read ────────────────────────────────────────────────────────

    /**
     * Reads the .env file and returns an [AppConfig] with any
     * `PENG_AGENT_*` variables applied on top of defaults.
     * Returns `null` if the file does not exist.
     */
    private fun readEnvFile(): AppConfig? {
        val envFile = File(ENV_FILE_PATH)
        if (!envFile.exists()) return null

        return try {
            val envMap = envFile.readLines()
                .map { it.trim() }
                .filter { it.isNotEmpty() && !it.startsWith("#") }
                .associate { line ->
                    val idx = line.indexOf('=')
                    if (idx > 0) line.substring(0, idx).trim() to line.substring(idx + 1).trim()
                    else "" to ""
                }
                .filterKeys { it.startsWith("PENG_AGENT_") }

            if (envMap.isEmpty()) return null

            AppConfig().applyEnvOverrides(envMap)
        } catch (e: Exception) {
            Log.e(TAG, "读取 .env 文件失败", e)
            null
        }
    }

    /**
     * Merges values from the .env file on top of an existing [AppConfig],
     * so that manual edits to .env are respected on reload.
     */
    private fun mergeWithEnvFile(config: AppConfig): AppConfig {
        val envFile = File(ENV_FILE_PATH)
        if (!envFile.exists()) return config

        return try {
            val envMap = envFile.readLines()
                .map { it.trim() }
                .filter { it.isNotEmpty() && !it.startsWith("#") }
                .associate { line ->
                    val idx = line.indexOf('=')
                    if (idx > 0) line.substring(0, idx).trim() to line.substring(idx + 1).trim()
                    else "" to ""
                }
                .filterKeys { it.startsWith("PENG_AGENT_") }

            if (envMap.isEmpty()) config else config.applyEnvOverrides(envMap)
        } catch (e: Exception) {
            Log.e(TAG, "合并 .env 配置失败", e)
            config
        }
    }

    // ── Field update helpers ─────────────────────────────────────────────

    /**
     * Returns a copy of this config with the given [key] updated to [value].
     * Supports type coercion: strings, ints, floats, longs, booleans.
     */
    private fun AppConfig.updateField(key: String, value: Any): AppConfig = when (key) {
        // String fields
        "model" -> copy(model = value.asString())
        "api_base" -> copy(apiBase = value.asString())
        "api_key" -> copy(apiKey = value.asString())
        "worker_model" -> copy(workerModel = value.asString())
        "worker_api_base" -> copy(workerApiBase = value.asString())
        "worker_api_key" -> copy(workerApiKey = value.asString())
        "system_prompt" -> copy(systemPrompt = value.asString())
        "user_name" -> copy(userName = value.asString())
        "user_style" -> copy(userStyle = value.asString())
        "vision_api_key" -> copy(visionApiKey = value.asString())
        "vision_api_base" -> copy(visionApiBase = value.asString())
        "vision_model" -> copy(visionModel = value.asString())
        "bridge_url" -> copy(bridgeUrl = value.asString())
        "compression_strategy" -> copy(compressionStrategy = value.asString())

        // Int fields
        "ws_port" -> copy(wsPort = value.asInt(18080))
        "max_tokens" -> copy(maxTokens = value.asInt(4096))
        "context_window" -> copy(contextWindow = value.asInt(128000))
        "memory_search_limit" -> copy(memorySearchLimit = value.asInt(5))
        "knowledge_search_limit" -> copy(knowledgeSearchLimit = value.asInt(3))
        "knowledge_max_docs" -> copy(knowledgeMaxDocs = value.asInt(0))
        "max_turns" -> copy(maxTurns = value.asInt(50))

        // Float fields
        "temperature" -> copy(temperature = value.asFloat(0.7f))
        "top_p" -> copy(topP = value.asFloat(0.9f))
        "compression_threshold" -> copy(compressionThreshold = value.asFloat(0.7f))

        // Long fields
        "tool_timeout_secs" -> copy(toolTimeoutSecs = value.asLong(30L))
        "stream_timeout_secs" -> copy(streamTimeoutSecs = value.asLong(60L))
        "knowledge_max_age_days" -> copy(knowledgeMaxAgeDays = value.asLong(0L))

        // Boolean fields
        "tool_execution_parallel" -> copy(toolExecutionParallel = value.asBoolean(true))
        "abort_on_error" -> copy(abortOnError = value.asBoolean(false))
        "knowledge_auto_cleanup" -> copy(knowledgeAutoCleanup = value.asBoolean(false))

        else -> {
            Log.w(TAG, "不支持的配置项: $key")
            this
        }
    }

    /**
     * Applies `PENG_AGENT_*` overrides from an env-map onto this config.
     */
    private fun AppConfig.applyEnvOverrides(env: Map<String, String>): AppConfig {
        var c = this
        env["PENG_AGENT_MODEL"]?.let { c = c.copy(model = it) }
        env["PENG_AGENT_API_BASE"]?.let { c = c.copy(apiBase = it) }
        env["PENG_AGENT_API_KEY"]?.let { c = c.copy(apiKey = it) }
        env["PENG_AGENT_WS_PORT"]?.toIntOrNull()?.let { c = c.copy(wsPort = it) }
        env["PENG_AGENT_WORKER_MODEL"]?.let { c = c.copy(workerModel = it) }
        env["PENG_AGENT_WORKER_API_BASE"]?.let { c = c.copy(workerApiBase = it) }
        env["PENG_AGENT_WORKER_API_KEY"]?.let { c = c.copy(workerApiKey = it) }
        env["PENG_AGENT_MAX_TOKENS"]?.toIntOrNull()?.let { c = c.copy(maxTokens = it) }
        env["PENG_AGENT_TEMPERATURE"]?.toFloatOrNull()?.let { c = c.copy(temperature = it) }
        env["PENG_AGENT_TOP_P"]?.toFloatOrNull()?.let { c = c.copy(topP = it) }
        env["PENG_AGENT_CONTEXT_WINDOW"]?.toIntOrNull()?.let { c = c.copy(contextWindow = it) }
        env["PENG_AGENT_COMPRESSION_THRESHOLD"]?.toFloatOrNull()
            ?.let { c = c.copy(compressionThreshold = it) }
        env["PENG_AGENT_COMPRESSION_STRATEGY"]?.let { c = c.copy(compressionStrategy = it) }
        env["PENG_AGENT_MEMORY_SEARCH_LIMIT"]?.toIntOrNull()
            ?.let { c = c.copy(memorySearchLimit = it) }
        env["PENG_AGENT_KNOWLEDGE_SEARCH_LIMIT"]?.toIntOrNull()
            ?.let { c = c.copy(knowledgeSearchLimit = it) }
        env["PENG_AGENT_KNOWLEDGE_MAX_DOCS"]?.toIntOrNull()
            ?.let { c = c.copy(knowledgeMaxDocs = it) }
        env["PENG_AGENT_KNOWLEDGE_MAX_AGE_DAYS"]?.toLongOrNull()
            ?.let { c = c.copy(knowledgeMaxAgeDays = it) }
        env["PENG_AGENT_KNOWLEDGE_AUTO_CLEANUP"]?.toBooleanStrictOrNull()
            ?.let { c = c.copy(knowledgeAutoCleanup = it) }
        env["PENG_AGENT_MAX_TURNS"]?.toIntOrNull()?.let { c = c.copy(maxTurns = it) }
        env["PENG_AGENT_TOOL_EXECUTION_PARALLEL"]?.toBooleanStrictOrNull()
            ?.let { c = c.copy(toolExecutionParallel = it) }
        env["PENG_AGENT_TOOL_TIMEOUT_SECS"]?.toLongOrNull()
            ?.let { c = c.copy(toolTimeoutSecs = it) }
        env["PENG_AGENT_STREAM_TIMEOUT_SECS"]?.toLongOrNull()
            ?.let { c = c.copy(streamTimeoutSecs = it) }
        env["PENG_AGENT_ABORT_ON_ERROR"]?.toBooleanStrictOrNull()
            ?.let { c = c.copy(abortOnError = it) }
        env["PENG_AGENT_SYSTEM_PROMPT"]?.let { c = c.copy(systemPrompt = it) }
        env["PENG_AGENT_USER_NAME"]?.let { c = c.copy(userName = it) }
        env["PENG_AGENT_USER_STYLE"]?.let { c = c.copy(userStyle = it) }
        env["PENG_AGENT_VISION_API_KEY"]?.let { c = c.copy(visionApiKey = it) }
        env["PENG_AGENT_VISION_API_BASE"]?.let { c = c.copy(visionApiBase = it) }
        env["PENG_AGENT_VISION_MODEL"]?.let { c = c.copy(visionModel = it) }
        env["PENG_AGENT_BRIDGE_URL"]?.let { c = c.copy(bridgeUrl = it) }
        return c
    }

    // ── Type coercion helpers ────────────────────────────────────────────

    private fun Any.asString(): String = (this as? String) ?: toString()

    private fun Any.asInt(defaultValue: Int = 0): Int = when (this) {
        is Number -> toInt()
        is String -> toIntOrNull() ?: defaultValue
        else -> defaultValue
    }

    private fun Any.asFloat(defaultValue: Float = 0f): Float = when (this) {
        is Number -> toFloat()
        is String -> toFloatOrNull() ?: defaultValue
        else -> defaultValue
    }

    private fun Any.asLong(defaultValue: Long = 0L): Long = when (this) {
        is Number -> toLong()
        is String -> toLongOrNull() ?: defaultValue
        else -> defaultValue
    }

    private fun Any.asBoolean(defaultValue: Boolean = false): Boolean = when (this) {
        is Boolean -> this
        is String -> toBooleanStrictOrNull() ?: defaultValue
        else -> defaultValue
    }
}
