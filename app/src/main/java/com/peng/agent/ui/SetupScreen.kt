package com.peng.agent.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.peng.agent.setup.UbuntuManager
import kotlinx.coroutines.launch

/**
 * 初始化安装界面
 */
@Composable
fun SetupScreen(
    onSetupComplete: () -> Unit
) {
    var state by remember { mutableStateOf<UbuntuManager.SetupState>(UbuntuManager.SetupState.NotStarted) }
    var progress by remember { mutableStateOf(0f) }
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    
    // 启动安装流程
    LaunchedEffect(Unit) {
        if (UbuntuManager.isReady()) {
            state = UbuntuManager.SetupState.Ready
        } else {
            UbuntuManager.performSetup(context) { newState ->
                state = newState
                progress = when (newState) {
                    is UbuntuManager.SetupState.NotStarted -> 0f
                    is UbuntuManager.SetupState.ExtractingProot -> 0.2f
                    is UbuntuManager.SetupState.ExtractingUbuntu -> 0.6f
                    is UbuntuManager.SetupState.Ready -> 1f
                    is UbuntuManager.SetupState.Error -> progress
                    is UbuntuManager.SetupState.Progress -> (newState as UbuntuManager.SetupState.Progress).percent / 100f
                }
            }
        }
    }
    
    // 当准备好时，触发完成回调
    if (state is UbuntuManager.SetupState.Ready) {
        LaunchedEffect(Unit) {
            onSetupComplete()
        }
    }
    
    Surface(
        modifier = Modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.background
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            // Logo
            Text(
                text = "鹏",
                style = MaterialTheme.typography.displayLarge,
                color = MaterialTheme.colorScheme.primary
            )
            
            Spacer(modifier = Modifier.height(24.dp))
            
            // 状态文字
            val statusText = when (state) {
                is UbuntuManager.SetupState.NotStarted -> "准备中..."
                is UbuntuManager.SetupState.ExtractingProot -> "安装运行环境..."
                is UbuntuManager.SetupState.ExtractingUbuntu -> "安装 Ubuntu 系统..."
                is UbuntuManager.SetupState.Progress -> (state as UbuntuManager.SetupState.Progress).message
                is UbuntuManager.SetupState.Ready -> "就绪!"
                is UbuntuManager.SetupState.Error -> "错误: ${(state as UbuntuManager.SetupState.Error).message}"
            }
            
            Text(
                text = statusText,
                style = MaterialTheme.typography.titleMedium,
                color = if (state is UbuntuManager.SetupState.Error) 
                    MaterialTheme.colorScheme.error 
                else 
                    MaterialTheme.colorScheme.onBackground,
                textAlign = TextAlign.Center
            )
            
            Spacer(modifier = Modifier.height(32.dp))
            
            // 进度条
            if (state !is UbuntuManager.SetupState.Ready && state !is UbuntuManager.SetupState.Error) {
                LinearProgressIndicator(
                    progress = { progress },
                    modifier = Modifier
                        .fillMaxWidth(0.6f)
                        .height(8.dp),
                    color = MaterialTheme.colorScheme.primary,
                    trackColor = MaterialTheme.colorScheme.surfaceVariant
                )
                
                Spacer(modifier = Modifier.height(16.dp))
                
                Text(
                    text = "${(progress * 100).toInt()}%",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.7f)
                )
            }
            
            // 错误时显示重试按钮
            if (state is UbuntuManager.SetupState.Error) {
                Spacer(modifier = Modifier.height(24.dp))
                
                Button(
                    onClick = {
                        state = UbuntuManager.SetupState.NotStarted
                        progress = 0f
                        scope.launch {
                            UbuntuManager.performSetup(context) { newState ->
                                state = newState
                                progress = when (newState) {
                                    is UbuntuManager.SetupState.NotStarted -> 0f
                                    is UbuntuManager.SetupState.ExtractingProot -> 0.2f
                                    is UbuntuManager.SetupState.ExtractingUbuntu -> 0.6f
                                    is UbuntuManager.SetupState.Ready -> 1f
                                    is UbuntuManager.SetupState.Error -> progress
                                    is UbuntuManager.SetupState.Progress -> (newState as UbuntuManager.SetupState.Progress).percent / 100f
                                }
                            }
                        }
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.primary
                    )
                ) {
                    Text("重试")
                }
            }
        }
    }
}