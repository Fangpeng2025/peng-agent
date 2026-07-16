package com.peng.agent.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.peng.agent.terminal.OutputType
import com.peng.agent.terminal.TerminalBuffer
import com.peng.agent.terminal.TerminalOutput
import com.peng.agent.ubuntu.UbuntuRuntime
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Linux 终端页面
 * 提供与 Ubuntu proot 环境交互的终端界面
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LinuxTerminalScreen(
    ubuntuRuntime: UbuntuRuntime?,
    modifier: Modifier = Modifier
) {
    val scope = rememberCoroutineScope()
    val buffer = remember { TerminalBuffer() }
    var inputText by remember { mutableStateOf("") }
    var isExecuting by remember { mutableStateOf(false) }
    var currentPath by remember { mutableStateOf("/root") }
    var commandHistory by remember { mutableStateOf(listOf<String>()) }
    var historyIndex by remember { mutableStateOf(-1) }
    
    // 环境状态
    var isUbuntuAvailable by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf("检查环境...") }
    
    // 检查 Ubuntu 环境
    LaunchedEffect(ubuntuRuntime) {
        if (ubuntuRuntime != null) {
            withContext(Dispatchers.IO) {
                isUbuntuAvailable = ubuntuRuntime.isAvailable()
                statusMessage = if (isUbuntuAvailable) "Ubuntu 环境就绪" else "Ubuntu 环境未就绪"
            }
            if (isUbuntuAvailable) {
                buffer.success("Ubuntu proot 环境已就绪")
                buffer.info("输入 'help' 查看可用命令")
                // 获取当前路径
                val result = ubuntuRuntime.execute("pwd", 5)
                if (result.exitCode == 0) {
                    currentPath = result.stdout.trim()
                }
            } else {
                buffer.error("Ubuntu 环境未初始化")
                buffer.info("请先完成初始化设置")
            }
        } else {
            statusMessage = "Ubuntu 运行时不可用"
            buffer.error("Ubuntu 运行时不可用")
        }
    }
    
    Column(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        // 顶部状态栏
        TerminalStatusBar(
            isAvailable = isUbuntuAvailable,
            statusMessage = statusMessage,
            currentPath = currentPath
        )
        
        // 终端输出区域
        TerminalOutputArea(
            buffer = buffer,
            modifier = Modifier.weight(1f)
        )
        
        // 输入区域
        TerminalInputBar(
            text = inputText,
            onTextChange = { inputText = it },
            isExecuting = isExecuting,
            currentPath = currentPath,
            onExecute = {
                if (inputText.isNotBlank() && !isExecuting && ubuntuRuntime != null && isUbuntuAvailable) {
                    val command = inputText.trim()
                    buffer.command(command)
                    
                    // 添加到历史
                    commandHistory = commandHistory + command
                    historyIndex = -1
                    
                    inputText = ""
                    isExecuting = true
                    
                    scope.launch {
                        withContext(Dispatchers.IO) {
                            val result = ubuntuRuntime.execute(command, 120)
                            
                            withContext(Dispatchers.Main) {
                                if (result.stdout.isNotBlank()) {
                                    result.stdout.lines().forEach { line ->
                                        if (line.isNotBlank()) buffer.stdout(line)
                                    }
                                }
                                if (result.stderr.isNotBlank()) {
                                    result.stderr.lines().forEach { line ->
                                        if (line.isNotBlank()) buffer.stderr(line)
                                    }
                                }
                                if (result.exitCode != 0 && result.stderr.isBlank()) {
                                    buffer.error("退出码: ${result.exitCode}")
                                }
                                
                                // 更新当前路径
                                if (command.startsWith("cd ")) {
                                    val pathResult = ubuntuRuntime.execute("pwd", 5)
                                    if (pathResult.exitCode == 0) {
                                        currentPath = pathResult.stdout.trim()
                                    }
                                }
                                
                                isExecuting = false
                            }
                        }
                    }
                }
            },
            onClear = { buffer.clear() },
            onHistoryUp = {
                if (commandHistory.isNotEmpty()) {
                    historyIndex = (historyIndex + 1).coerceIn(0, commandHistory.lastIndex)
                    inputText = commandHistory[commandHistory.lastIndex - historyIndex]
                }
            },
            onHistoryDown = {
                if (historyIndex > 0) {
                    historyIndex--
                    inputText = commandHistory[commandHistory.lastIndex - historyIndex]
                } else if (historyIndex == 0) {
                    historyIndex = -1
                    inputText = ""
                }
            }
        )
    }
}

/**
 * 终端状态栏
 */
@Composable
fun TerminalStatusBar(
    isAvailable: Boolean,
    statusMessage: String,
    currentPath: String
) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surfaceVariant,
        tonalElevation = 2.dp
    ) {
        Row(
            modifier = Modifier
                .padding(horizontal = 12.dp, vertical = 8.dp)
                .fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // 状态指示器
            Box(
                modifier = Modifier
                    .size(10.dp)
                    .background(
                        if (isAvailable) Color(0xFF4CAF50) else Color(0xFFF44336),
                        RoundedCornerShape(5.dp)
                    )
            )
            
            Spacer(modifier = Modifier.width(8.dp))
            
            Text(
                text = statusMessage,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            Spacer(modifier = Modifier.weight(1f))
            
            // 当前路径
            Surface(
                shape = RoundedCornerShape(4.dp),
                color = MaterialTheme.colorScheme.primary.copy(alpha = 0.1f)
            ) {
                Text(
                    text = currentPath,
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                    style = MaterialTheme.typography.labelSmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.primary
                )
            }
        }
    }
}

/**
 * 终端输出区域
 */
@Composable
fun TerminalOutputArea(
    buffer: TerminalBuffer,
    modifier: Modifier = Modifier
) {
    val scrollState = rememberScrollState()
    val lines = buffer.lines
    
    // 自动滚动到底部
    LaunchedEffect(lines.size) {
        scrollState.animateScrollTo(scrollState.maxValue)
    }
    
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color(0xFF1E1E1E)) // VS Code 深色背景
    ) {
        if (lines.isEmpty()) {
            // 空状态提示
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(16.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center
            ) {
                Icon(
                    imageVector = Icons.Default.Terminal,
                    contentDescription = null,
                    modifier = Modifier.size(48.dp),
                    tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f)
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Ubuntu 终端",
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f)
                )
                Text(
                    text = "输入命令开始交互",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f)
                )
            }
        } else {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(scrollState)
                    .padding(8.dp)
            ) {
                lines.forEach { line ->
                    TerminalLine(line)
                }
                
                // 执行中指示器
                if (true) { // 可以添加 isExecuting 参数
                    Text(
                        text = "▌",
                        fontFamily = FontFamily.Monospace,
                        fontSize = 12.sp,
                        color = Color(0xFF4EC9B0)
                    )
                }
            }
        }
    }
}

/**
 * 终端单行
 */
@Composable
fun TerminalLine(line: TerminalOutput) {
    val text = buildAnnotatedString {
        when (line.type) {
            OutputType.COMMAND -> {
                // 命令行 - 绿色 $ + 黄色命令
                withStyle(SpanStyle(color = Color(0xFF4EC9B0))) {
                    append("$ ")
                }
                withStyle(SpanStyle(color = Color(0xFFDCDCAA))) {
                    append(line.text)
                }
            }
            OutputType.STDOUT -> {
                // 标准输出 - 白色/浅灰
                withStyle(SpanStyle(color = Color(0xFFD4D4D4))) {
                    append(line.text)
                }
            }
            OutputType.STDERR -> {
                // 错误输出 - 红色
                withStyle(SpanStyle(color = Color(0xFFF14C4C))) {
                    append(line.text)
                }
            }
            OutputType.SUCCESS -> {
                // 成功 - 绿色 ✓
                withStyle(SpanStyle(color = Color(0xFF6A9955))) {
                    append("✓ ")
                    append(line.text)
                }
            }
            OutputType.ERROR -> {
                // 错误 - 红色 ✗
                withStyle(SpanStyle(color = Color(0xFFF14C4C))) {
                    append("✗ ")
                    append(line.text)
                }
            }
            OutputType.INFO -> {
                // 信息 - 蓝色 ℹ
                withStyle(SpanStyle(color = Color(0xFF569CD6))) {
                    append("ℹ ")
                    append(line.text)
                }
            }
            OutputType.WARNING -> {
                // 警告 - 黄色 ⚠
                withStyle(SpanStyle(color = Color(0xFFCCA700))) {
                    append("⚠ ")
                    append(line.text)
                }
            }
            OutputType.PROGRESS -> {
                // 进度 - 青色
                withStyle(SpanStyle(color = Color(0xFF4EC9B0))) {
                    append(line.text)
                }
            }
        }
    }
    
    Text(
        text = text,
        fontFamily = FontFamily.Monospace,
        fontSize = 12.sp,
        modifier = Modifier.fillMaxWidth()
    )
}

/**
 * 终端输入栏
 */
@Composable
fun TerminalInputBar(
    text: String,
    onTextChange: (String) -> Unit,
    isExecuting: Boolean,
    currentPath: String,
    onExecute: () -> Unit,
    onClear: () -> Unit,
    onHistoryUp: () -> Unit,
    onHistoryDown: () -> Unit
) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 4.dp
    ) {
        Row(
            modifier = Modifier
                .padding(8.dp)
                .fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // 路径提示符
            Text(
                text = "$ ",
                fontFamily = FontFamily.Monospace,
                fontSize = 14.sp,
                color = Color(0xFF4EC9B0)
            )
            
            // 输入框
            OutlinedTextField(
                value = text,
                onValueChange = onTextChange,
                modifier = Modifier.weight(1f),
                placeholder = {
                    Text(
                        "输入命令...",
                        fontFamily = FontFamily.Monospace,
                        fontSize = 14.sp
                    )
                },
                textStyle = androidx.compose.ui.text.TextStyle(
                    fontFamily = FontFamily.Monospace,
                    fontSize = 14.sp,
                    color = Color(0xFFDCDCAA)
                ),
                singleLine = true,
                enabled = !isExecuting,
                keyboardOptions = KeyboardOptions(
                    imeAction = ImeAction.Send
                ),
                keyboardActions = KeyboardActions(
                    onSend = { onExecute() }
                ),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = Color(0xFF4EC9B0),
                    unfocusedBorderColor = Color(0xFF3C3C3C),
                    cursorColor = Color(0xFF4EC9B0),
                    disabledBorderColor = Color(0xFF2D2D2D)
                ),
                shape = RoundedCornerShape(8.dp)
            )
            
            Spacer(modifier = Modifier.width(8.dp))
            
            // 清除按钮
            IconButton(
                onClick = onClear,
                enabled = !isExecuting
            ) {
                Icon(
                    imageVector = Icons.Default.DeleteSweep,
                    contentDescription = "清除",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            // 执行按钮
            FilledIconButton(
                onClick = onExecute,
                enabled = text.isNotBlank() && !isExecuting,
                colors = IconButtonDefaults.filledIconButtonColors(
                    containerColor = Color(0xFF4EC9B0),
                    disabledContainerColor = Color(0xFF3C3C3C)
                )
            ) {
                if (isExecuting) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(18.dp),
                        strokeWidth = 2.dp,
                        color = Color.White
                    )
                } else {
                    Icon(
                        imageVector = Icons.Default.PlayArrow,
                        contentDescription = "执行",
                        tint = if (text.isNotBlank()) Color.White else Color(0xFF666666)
                    )
                }
            }
        }
    }
}
