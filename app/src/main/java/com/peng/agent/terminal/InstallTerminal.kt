package com.peng.agent.terminal

import android.content.Context
import android.util.Log
import androidx.compose.runtime.*
import androidx.compose.ui.platform.LocalContext
import com.peng.agent.setup.UbuntuManager
import kotlinx.coroutines.delay

/**
 * 安装终端 - 显示 Ubuntu 安装过程
 */
@Composable
fun InstallTerminal(
    onComplete: () -> Unit,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier
) {
    val context = LocalContext.current
    val buffer = remember { TerminalBuffer() }
    var isComplete by remember { mutableStateOf(false) }
    
    // 检查是否已安装
    LaunchedEffect(Unit) {
        if (UbuntuManager.isReady()) {
            buffer.success("Ubuntu 环境已就绪")
            buffer.info("启动 peng-daemon...")
            isComplete = true
            delay(500)
            onComplete()
        } else {
            buffer.info("开始安装 Ubuntu 环境...")
            buffer.info("")
            
            // 在后台执行安装
            performInstall(context, buffer) { success ->
                if (success) {
                    buffer.success("")
                    buffer.success("安装完成!")
                }
                isComplete = true
            }
        }
    }
    
    // 当安装完成时，延迟调用 onComplete
    LaunchedEffect(isComplete) {
        if (isComplete) {
            delay(1500)
            onComplete()
        }
    }
    
    TerminalScreen(
        lines = buffer.lines,
        modifier = modifier
    )
}

/**
 * 执行安装流程
 */
private fun performInstall(
    context: Context,
    buffer: TerminalBuffer,
    onResult: (Boolean) -> Unit
) {
    Thread {
        try {
            // Step 1: 解压 Bootstrap
            buffer.command("正在解压 Termux Bootstrap...")
            val bootstrapResult = UbuntuManager.extractBootstrapWithOutput(context) { line ->
                buffer.stdout(line)
            }
            if (!bootstrapResult) {
                buffer.error("Bootstrap 解压失败")
                onResult(false)
            } else {
                buffer.success("Bootstrap 解压完成")
                buffer.info("")
                
                // Step 2: 安装 proot-distro
                buffer.command("pkg install proot-distro")
                val prootResult = UbuntuManager.installProotDistroWithOutput(context) { line ->
                    buffer.stdout(line)
                }
                if (!prootResult) {
                    buffer.error("proot-distro 安装失败")
                    onResult(false)
                } else {
                    buffer.success("proot-distro 安装完成")
                    buffer.info("")
                    
                    // Step 3: 安装 Ubuntu
                    buffer.command("proot-distro install ubuntu")
                    buffer.info("下载 Ubuntu 镜像 (约 200MB, 请稍候...)")
                    val ubuntuResult = UbuntuManager.installUbuntuWithOutput(context) { line ->
                        buffer.stdout(line)
                    }
                    if (!ubuntuResult) {
                        buffer.error("Ubuntu 安装失败")
                        onResult(false)
                    } else {
                        buffer.success("Ubuntu 安装完成")
                        buffer.info("")
                        
                        // Step 4: 安装软件包
                        buffer.command("apt install python3 nodejs ffmpeg...")
                        val packagesResult = UbuntuManager.installPackagesWithOutput(context) { line ->
                            buffer.stdout(line)
                        }
                        if (!packagesResult) {
                            buffer.warning("部分软件包安装失败，但可以继续")
                        } else {
                            buffer.success("软件包安装完成")
                        }
                        buffer.info("")
                        
                        // Step 5: 安装 daemon
                        buffer.command("安装 peng-daemon...")
                        val daemonResult = UbuntuManager.installDaemonWithOutput(context) { line ->
                            buffer.stdout(line)
                        }
                        if (!daemonResult) {
                            buffer.error("peng-daemon 安装失败")
                            onResult(false)
                        } else {
                            buffer.success("peng-daemon 安装完成")
                            onResult(true)
                        }
                    }
                }
            }
        } catch (e: Exception) {
            buffer.error("安装异常: ${e.message}")
            Log.e("InstallTerminal", "Install failed", e)
            onResult(false)
        }
    }.start()
}