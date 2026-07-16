package com.peng.agent

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.draw.clip
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.draw.clip
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.outlined.ChatBubbleOutline
import androidx.compose.material.icons.outlined.GridView
import androidx.compose.material.icons.outlined.Psychology
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.outlined.SmartToy
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.lifecycleScope
import com.peng.agent.client.BackendClient
import com.peng.agent.service.DynamicIslandService
import com.peng.agent.service.DynamicIslandState
import com.peng.agent.setup.SetupManager
import com.peng.agent.setup.SetupState
import com.peng.agent.setup.SetupStep
import com.peng.agent.ui.ChatScreen
import com.peng.agent.ui.KanbanScreen
import com.peng.agent.ui.MemoryPage
import com.peng.agent.ui.SettingsScreen
import com.peng.agent.ui.SkillsScreen
import com.peng.agent.ui.theme.BrandPrimary
import com.peng.agent.ui.theme.PengTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

// ═══════════════════════════════════════════════════════════════════════
//  Tab 定义
// ═══════════════════════════════════════════════════════════════════════

enum class AppTab(
    val label: String,
    val icon: ImageVector
) {
    CHAT("聊天", Icons.Outlined.ChatBubbleOutline),
    SKILLS("技能", Icons.Outlined.SmartToy),
    KANBAN("看板", Icons.Outlined.GridView),
    MEMORY("记忆", Icons.Outlined.Psychology),
    SETTINGS("设置", Icons.Outlined.Settings)
}

// ═══════════════════════════════════════════════════════════════════════
//  MainActivity
// ═══════════════════════════════════════════════════════════════════════

class MainActivity : ComponentActivity() {

    private val client: BackendClient = PengApplication.client

    private val scope = CoroutineScope(Dispatchers.Main)

    private val _setupState = MutableStateFlow(SetupState())
    val setupState: StateFlow<SetupState> = _setupState.asStateFlow()

    private val _showSetup = MutableStateFlow(false)
    val showSetup: StateFlow<Boolean> = _showSetup.asStateFlow()

    private val _returnToSessionId = MutableStateFlow("")
    val returnToSessionId: StateFlow<String> = _returnToSessionId.asStateFlow()

    private val overlayPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (Settings.canDrawOverlays(this)) {
            Log.i(TAG, "✅ 悬浮窗权限已授予")
        } else {
            Log.w(TAG, "⚠️ 悬浮窗权限被拒绝")
        }
    }

    private fun isSettingUp(): Boolean =
        getSharedPreferences("peng_setup", Context.MODE_PRIVATE)
            .getBoolean("is_setting_up", false)

    private fun setSettingUp(value: Boolean) =
        getSharedPreferences("peng_setup", Context.MODE_PRIVATE)
            .edit().putBoolean("is_setting_up", value).apply()

    fun dismissSetup() { _showSetup.value = false }
    fun clearReturnToSessionId() { _returnToSessionId.value = "" }

    fun retrySetup() {
        scope.launch {
            _setupState.value = SetupState(step = SetupStep.CHECKING)
            runFirstTimeSetup(this@MainActivity)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        DynamicIslandState.init(this)
        checkOverlayPermission()
        androidx.core.view.WindowCompat.setDecorFitsSystemWindows(window, false)

        setContent {
            PengTheme {
                PengAgentApp(client = client, activity = this@MainActivity)
            }
        }

        intent?.getStringExtra("test_island")?.let { testIsland ->
            if (testIsland == "true") testDynamicIsland()
        }
        handleReturnFromTask(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleReturnFromTask(intent)
    }

    override fun onResume() {
        super.onResume()
        if (!isSettingUp()) {
            scope.launch { performStartupCheck(this@MainActivity) }
        }
    }

    private fun handleReturnFromTask(intent: Intent) {
        val returnSessionId = intent.getStringExtra("return_session_id")
        if (!returnSessionId.isNullOrEmpty()) {
            _returnToSessionId.value = returnSessionId
        }
    }

    private fun checkOverlayPermission() {
        if (!Settings.canDrawOverlays(this)) {
            val intent = Intent(
                Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                Uri.parse("package:$packageName")
            )
            overlayPermissionLauncher.launch(intent)
        }
    }

    private fun testDynamicIsland() {
        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission("android.permission.POST_NOTIFICATIONS") != 0
        ) {
            requestPermissions(arrayOf("android.permission.POST_NOTIFICATIONS"), 1001)
        }
        val intent = Intent(this, DynamicIslandService::class.java).apply {
            action = DynamicIslandService.ACTION_START
            putExtra(DynamicIslandService.EXTRA_TASK_ID, "test_island_001")
            putExtra(DynamicIslandService.EXTRA_TASK_NAME, "灵动岛测试任务")
        }
        startService(intent)
        lifecycleScope.launch {
            try {
                DynamicIslandState.startTask("test_island_001", "灵动岛测试任务")
                kotlinx.coroutines.delay(2000)
                DynamicIslandState.updatePhase("thinking")
                kotlinx.coroutines.delay(2000)
                DynamicIslandState.updateToolProgress("shell_exec", "running")
                kotlinx.coroutines.delay(2000)
                DynamicIslandState.updateToolProgress("shell_exec", "completed")
                kotlinx.coroutines.delay(2000)
                DynamicIslandState.updatePhase("streaming")
                kotlinx.coroutines.delay(3000)
                DynamicIslandState.stopTask()
            } catch (_: Exception) {}
        }
    }

    private suspend fun performStartupCheck(context: Context) {
        Log.i(TAG, "🔍 启动自检...")
        if (!SetupManager.isSetupComplete(context)) {
            _showSetup.value = true
            runFirstTimeSetup(context)
            return
        }
        if (!SetupManager.hasStoragePermission()) {
            _showSetup.value = true
            runFirstTimeSetup(context)
            return
        }
        _showSetup.value = false
        startBackendAndConnect(context)
    }

    private suspend fun runFirstTimeSetup(context: Context) {
        try {
            _setupState.value = SetupState(step = SetupStep.CHECKING)
            kotlinx.coroutines.delay(500)

            _setupState.value = SetupState(step = SetupStep.REQUESTING_PERMISSION, progress = 25)
            if (!SetupManager.hasStoragePermission()) {
                _setupState.value = SetupState(
                    step = SetupStep.REQUESTING_PERMISSION,
                    progress = 30,
                    needsUserAction = true,
                    userActionMessage = "请授予\"所有文件访问\"权限以继续设置"
                )
                setSettingUp(true)
                scope.launch(Dispatchers.Main) { requestStoragePermission() }
                var waited = 0
                while (!SetupManager.hasStoragePermission() && waited < 120) {
                    kotlinx.coroutines.delay(1000)
                    waited++
                    _setupState.value = _setupState.value.copy(progress = 30 + waited * 40 / 120)
                }
                setSettingUp(false)
                if (!SetupManager.hasStoragePermission()) {
                    _setupState.value = SetupState(
                        step = SetupStep.ERROR,
                        errorMessage = "存储权限未授予，无法继续设置"
                    )
                    return
                }
            }

            _setupState.value = SetupState(step = SetupStep.INITIALIZING_BACKEND, progress = 75)
            if (!SetupManager.performFirstTimeSetup(context)) {
                _setupState.value = SetupState(
                    step = SetupStep.ERROR,
                    errorMessage = "后端初始化失败，请重试"
                )
                return
            }

            _setupState.value = SetupState(step = SetupStep.COMPLETE, progress = 100)
            kotlinx.coroutines.delay(800)
            _showSetup.value = false
            startBackendAndConnect(context)
        } catch (e: Exception) {
            _setupState.value = SetupState(
                step = SetupStep.ERROR,
                errorMessage = e.message ?: "设置过程中出现未知错误"
            )
        }
    }

    private fun requestStoragePermission() {
        if (Build.VERSION.SDK_INT >= 30) {
            try {
                Intent(
                    Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                    Uri.parse("package:$packageName")
                ).also { startActivity(it) }
            } catch (_: Exception) {
                Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION).also { startActivity(it) }
            }
        }
    }

    private fun startBackendAndConnect(context: Context) {
        Log.i(TAG, "✅ 嵌入式后端已初始化（直连模式）")
    }

    companion object {
        private const val TAG = "MainActivity"
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Top-level Composable
// ═══════════════════════════════════════════════════════════════════════

@Composable
fun PengAgentApp(
    client: BackendClient,
    activity: MainActivity
) {
    val showSetup by activity.showSetup.collectAsState()
    val setupState by activity.setupState.collectAsState()

    if (showSetup || setupState.isError) {
        SetupProgressScreen(
            state = setupState,
            onRetry = { activity.retrySetup() },
            onSkip = { activity.dismissSetup() }
        )
    } else {
        MainAppContent(client = client, activity = activity)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  MainAppContent — 底部Tab导航
// ═══════════════════════════════════════════════════════════════════════

@Composable
fun MainAppContent(
    client: BackendClient,
    activity: MainActivity
) {
    val returnToSessionId by activity.returnToSessionId.collectAsState()
    var selectedTab by remember { mutableStateOf(AppTab.CHAT) }

    // Handle return from task → switch to chat tab
    LaunchedEffect(returnToSessionId) {
        if (returnToSessionId.isNotEmpty()) {
            selectedTab = AppTab.CHAT
            activity.clearReturnToSessionId()
        }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        bottomBar = {
            NavigationBar(
                containerColor = MaterialTheme.colorScheme.surface,
                tonalElevation = 2.dp,
                contentColor = MaterialTheme.colorScheme.onSurface
            ) {
                AppTab.entries.forEach { tab ->
                    NavigationBarItem(
                        icon = {
                            Icon(
                                imageVector = tab.icon,
                                contentDescription = tab.label
                            )
                        },
                        label = {
                            Text(
                                text = tab.label,
                                style = MaterialTheme.typography.labelSmall
                            )
                        },
                        selected = selectedTab == tab,
                        onClick = { selectedTab = tab },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = BrandPrimary,
                            selectedTextColor = BrandPrimary,
                            indicatorColor = BrandPrimary.copy(alpha = 0.12f),
                            unselectedIconColor = MaterialTheme.colorScheme.onSurfaceVariant,
                            unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    )
                }
            }
        }
    ) { innerPadding ->
        AnimatedContent(
            targetState = selectedTab,
            modifier = Modifier.padding(innerPadding),
            transitionSpec = {
                slideInVertically(initialOffsetY = { it / 4 }) togetherWith
                    slideOutVertically(targetOffsetY = { -it / 4 })
            },
            label = "tab_transition"
        ) { tab ->
            when (tab) {
                AppTab.CHAT -> ChatScreen(client = client)
                AppTab.SKILLS -> SkillsScreen(client = client, onSkillClick = { /* TODO */ })
                AppTab.KANBAN -> KanbanScreen(client = client)
                AppTab.MEMORY -> MemoryPage(client = client)
                AppTab.SETTINGS -> SettingsScreen(client = client)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  SetupProgressScreen
// ═══════════════════════════════════════════════════════════════════════

@Composable
fun SetupProgressScreen(
    state: SetupState,
    onRetry: () -> Unit,
    onSkip: () -> Unit
) {
    Surface(
        modifier = Modifier,
        color = MaterialTheme.colorScheme.background
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            StepIndicator(currentStep = state.step)

            Spacer(
                modifier = Modifier.height(32.dp)
            )

            Text(
                text = state.step.title,
                style = MaterialTheme.typography.headlineLarge,
                color = MaterialTheme.colorScheme.onBackground,
                fontWeight = FontWeight.Bold
            )

            Spacer(
                modifier = Modifier.height(8.dp)
            )

            Text(
                text = state.step.description,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center
            )

            Spacer(
                modifier = Modifier.height(24.dp)
            )

            if (state.step != SetupStep.ERROR) {
                LinearProgressIndicator(
                    progress = { state.progress / 100f },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(6.dp)
                        .clip(RoundedCornerShape(3.dp)),
                    color = BrandPrimary,
                    trackColor = MaterialTheme.colorScheme.surfaceVariant
                )
                Spacer(
                    modifier = Modifier.height(8.dp)
                )
                Text(
                    text = if (state.totalBytes > 0)
                        "${formatFileSize(state.bytesDownloaded)} / ${formatFileSize(state.totalBytes)}"
                    else "${state.progress}%",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            if (state.needsUserAction && state.userActionMessage != null) {
                Spacer(
                    modifier = Modifier.height(16.dp)
                )
                Surface(
                    shape = RoundedCornerShape(12.dp),
                    color = BrandPrimary.copy(alpha = 0.08f),
                    border = BorderStroke(1.dp, BrandPrimary.copy(alpha = 0.2f))
                ) {
                    Text(
                        text = state.userActionMessage,
                        modifier = Modifier.padding(16.dp),
                        style = MaterialTheme.typography.bodyMedium,
                        color = BrandPrimary
                    )
                }
            }

            if (state.isError && state.errorMessage != null) {
                Spacer(
                    modifier = Modifier.height(16.dp)
                )
                Surface(
                    shape = RoundedCornerShape(12.dp),
                    color = MaterialTheme.colorScheme.error.copy(alpha = 0.08f),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.error.copy(alpha = 0.2f))
                ) {
                    Row(
                        modifier = Modifier.padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(
                            imageVector = Icons.Default.Error,
                            contentDescription = null,
                            modifier = Modifier.size(20.dp),
                            tint = MaterialTheme.colorScheme.error
                        )
                        Spacer(
                            modifier = Modifier.width(8.dp)
                        )
                        Text(
                            text = state.errorMessage,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.error
                        )
                    }
                }
            }

            if (state.isComplete) {
                Spacer(
                    modifier = Modifier.height(16.dp)
                )
                Row(
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        imageVector = Icons.Default.CheckCircle,
                        contentDescription = null,
                        modifier = Modifier.size(24.dp),
                        tint = BrandPrimary
                    )
                    Spacer(
                        modifier = Modifier.width(8.dp)
                    )
                    Text(
                        text = "设置完成！",
                        style = MaterialTheme.typography.titleMedium,
                        color = BrandPrimary,
                        fontWeight = FontWeight.SemiBold
                    )
                }
            }

            if (state.isError) {
                Spacer(
                    modifier = Modifier.height(24.dp)
                )
                Row(
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    OutlinedButton(
                        onClick = onSkip,
                        shape = RoundedCornerShape(8.dp)
                    ) { Text("跳过") }
                    Button(
                        onClick = onRetry,
                        shape = RoundedCornerShape(8.dp),
                        colors = ButtonDefaults.buttonColors(containerColor = BrandPrimary)
                    ) {
                        Icon(
                            imageVector = Icons.Default.Refresh,
                            contentDescription = null,
                            modifier = Modifier.size(18.dp)
                        )
                        Spacer(
                            modifier = Modifier.width(4.dp)
                        )
                        Text("重试")
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  StepIndicator
// ═══════════════════════════════════════════════════════════════════════

@Composable
fun StepIndicator(currentStep: SetupStep) {
    val steps = listOf(
        SetupStep.CHECKING,
        SetupStep.REQUESTING_PERMISSION,
        SetupStep.INITIALIZING_BACKEND,
        SetupStep.COMPLETE
    )
    val currentIndex = when (currentStep) {
        SetupStep.CHECKING -> 0
        SetupStep.REQUESTING_PERMISSION -> 1
        SetupStep.INITIALIZING_BACKEND -> 2
        SetupStep.COMPLETE -> 3
        SetupStep.ERROR -> -1
    }
    Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        for (index in steps.indices) {
            val isActive = index == currentIndex
            val isCompleted = index < currentIndex || currentStep == SetupStep.COMPLETE
            val isError = currentStep == SetupStep.ERROR && index == currentIndex.coerceIn(0, steps.lastIndex)

            Box(
                modifier = Modifier
                    .size(36.dp)
                    .clip(CircleShape)
                    .background(
                        when {
                            isError -> MaterialTheme.colorScheme.error.copy(alpha = 0.1f)
                            isCompleted -> BrandPrimary.copy(alpha = 0.15f)
                            isActive -> BrandPrimary.copy(alpha = 0.1f)
                            else -> MaterialTheme.colorScheme.surfaceVariant
                        }
                    ),
                contentAlignment = Alignment.Center
            ) {
                if (isCompleted && !isError) {
                    Icon(
                        imageVector = Icons.Default.CheckCircle,
                        contentDescription = null,
                        modifier = Modifier.size(20.dp),
                        tint = BrandPrimary
                    )
                } else if (isError) {
                    Icon(
                        imageVector = Icons.Default.Error,
                        contentDescription = null,
                        modifier = Modifier.size(20.dp),
                        tint = MaterialTheme.colorScheme.error
                    )
                } else {
                    Text(
                        text = "${index + 1}",
                        style = MaterialTheme.typography.labelMedium,
                        color = if (isActive) BrandPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
                        fontWeight = if (isActive) FontWeight.Bold else FontWeight.Normal
                    )
                }
            }
            if (index < steps.lastIndex) {
                Box(
                    modifier = Modifier
                        .width(24.dp)
                        .height(2.dp)
                        .clip(RoundedCornerShape(1.dp))
                        .background(
                            if (isCompleted) BrandPrimary.copy(alpha = 0.4f)
                            else MaterialTheme.colorScheme.outlineVariant
                        )
                )
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Utility
// ═══════════════════════════════════════════════════════════════════════

private fun formatFileSize(bytes: Long): String = when {
    bytes < 1024 -> "$bytes B"
    bytes < 1048576 -> "%.1f KB".format(bytes / 1024.0)
    bytes < 1073741824 -> "%.1f MB".format(bytes / 1048576.0)
    else -> "%.2f GB".format(bytes / 1073741824.0)
}
