package com.peng.agent.terminal

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * 终端屏幕 - 显示文本行
 */
@Composable
fun TerminalScreen(
    lines: List<TerminalOutput>,
    modifier: Modifier = Modifier
) {
    val scrollState = rememberScrollState()
    
    // 自动滚动到底部
    LaunchedEffect(lines.size) {
        scrollState.animateScrollTo(scrollState.maxValue)
    }
    
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color(0xFF1E1E1E)) // VS Code 深色背景
    ) {
        SelectionContainer {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(scrollState)
                    .padding(8.dp)
            ) {
                lines.forEach { line ->
                    TerminalLine(line)
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
                // 命令行 - 绿色
                withStyle(SpanStyle(color = Color(0xFF4EC9B0))) {
                    append("$ ")
                }
                withStyle(SpanStyle(color = Color(0xFFDCDCAA))) {
                    append(line.text)
                }
            }
            OutputType.STDOUT -> {
                // 标准输出 - 白色
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
                // 成功 - 绿色
                withStyle(SpanStyle(color = Color(0xFF6A9955))) {
                    append("✓ ")
                    append(line.text)
                }
            }
            OutputType.ERROR -> {
                // 错误 - 红色加粗
                withStyle(SpanStyle(color = Color(0xFFF14C4C))) {
                    append("✗ ")
                    append(line.text)
                }
            }
            OutputType.INFO -> {
                // 信息 - 蓝色
                withStyle(SpanStyle(color = Color(0xFF569CD6))) {
                    append("ℹ ")
                    append(line.text)
                }
            }
            OutputType.WARNING -> {
                // 警告 - 黄色
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
    
    androidx.compose.material3.Text(
        text = text,
        fontFamily = FontFamily.Monospace,
        fontSize = 12.sp,
        modifier = Modifier.fillMaxWidth()
    )
}

/**
 * 终端输出类型
 */
enum class OutputType {
    COMMAND,    // 命令
    STDOUT,     // 标准输出
    STDERR,     // 错误输出
    SUCCESS,    // 成功
    ERROR,      // 错误
    INFO,       // 信息
    WARNING,    // 警告
    PROGRESS    // 进度
}

/**
 * 终端输出行
 */
data class TerminalOutput(
    val text: String,
    val type: OutputType = OutputType.STDOUT
)

/**
 * 终端输出缓冲区
 */
class TerminalBuffer {
    private val _lines = mutableStateListOf<TerminalOutput>()
    val lines: List<TerminalOutput> get() = _lines.toList()
    
    fun command(text: String) {
        _lines.add(TerminalOutput(text, OutputType.COMMAND))
    }
    
    fun stdout(text: String) {
        _lines.add(TerminalOutput(text, OutputType.STDOUT))
    }
    
    fun stderr(text: String) {
        _lines.add(TerminalOutput(text, OutputType.STDERR))
    }
    
    fun success(text: String) {
        _lines.add(TerminalOutput(text, OutputType.SUCCESS))
    }
    
    fun error(text: String) {
        _lines.add(TerminalOutput(text, OutputType.ERROR))
    }
    
    fun info(text: String) {
        _lines.add(TerminalOutput(text, OutputType.INFO))
    }
    
    fun warning(text: String) {
        _lines.add(TerminalOutput(text, OutputType.WARNING))
    }
    
    fun progress(text: String) {
        _lines.add(TerminalOutput(text, OutputType.PROGRESS))
    }
    
    fun clear() {
        _lines.clear()
    }
}