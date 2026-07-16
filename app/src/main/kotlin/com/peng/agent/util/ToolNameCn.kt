package com.peng.agent.util

/**
 * 工具名称中文映射 — 将英文工具名转换为中文显示名
 */
object ToolNameCn {

    private val nameMap = mapOf(
        // Shell & 执行
        "shell_exec" to "Shell执行",
        "execute_shell" to "Shell执行",
        "execute_python" to "Python执行",
        "python_exec" to "Python执行",
        "bash" to "Bash执行",

        // 文件操作
        "read_file" to "读取文件",
        "write_file" to "写入文件",
        "edit_file" to "编辑文件",
        "list_directory" to "列出目录",
        "create_directory" to "创建目录",
        "delete_file" to "删除文件",
        "move_file" to "移动文件",
        "copy_file" to "复制文件",
        "search_files" to "搜索文件",
        "file_search" to "文件搜索",
        "grep" to "文本搜索",
        "find" to "查找文件",

        // 屏幕操作
        "get_screen" to "获取屏幕",
        "click_element" to "点击元素",
        "long_click_element" to "长按元素",
        "double_click_element" to "双击元素",
        "type_text" to "输入文本",
        "scroll_screen" to "滚动屏幕",
        "swipe_screen" to "滑动屏幕",
        "press_key" to "按键操作",
        "screenshot" to "截图",
        "take_screenshot" to "截图",

        // 应用管理
        "launch_app" to "启动应用",
        "list_apps" to "应用列表",
        "install_app" to "安装应用",
        "uninstall_app" to "卸载应用",

        // OCR
        "ocr_region" to "文字识别",
        "ocr" to "文字识别",

        // 剪贴板
        "copy_text" to "复制文本",
        "paste_text" to "粘贴文本",
        "cut_text" to "剪切文本",
        "select_all_text" to "全选文本",
        "clear_text" to "清除文本",

        // Canvas
        "canvas_create" to "创建画布",
        "canvas_list" to "画布列表",
        "canvas_render" to "渲染画布",

        // Video
        "video_render" to "视频渲染",
        "video_understand" to "视频理解",

        // 网络
        "web_search" to "网络搜索",
        "web_fetch" to "网页抓取",
        "http_request" to "HTTP请求",
        "fetch_url" to "获取URL",

        // 知识库
        "knowledge_search" to "知识搜索",
        "knowledge_add" to "添加知识",
        "knowledge_delete" to "删除知识",
        "knowledge_list" to "知识列表",
        "diagnose_knowledge" to "诊断知识库",

        // 记忆
        "memory_search" to "记忆搜索",
        "memory_add" to "添加记忆",
        "memory_delete" to "删除记忆",
        "memory_list" to "记忆列表",

        // 任务管理
        "todo" to "待办事项",
        "task_create" to "创建任务",
        "task_update" to "更新任务",
        "task_list" to "任务列表",
        "task_delete" to "删除任务",

        // 其他
        "think" to "思考",
        "plan" to "规划",
        "delegate" to "委派",
        "subagent" to "子Agent",
        "summarize" to "总结",
        "translate" to "翻译",
        "code_review" to "代码审查",
        "debug" to "调试",
        "test" to "测试",
        "deploy" to "部署",
        "monitor" to "监控",
        "notify" to "通知",
        "schedule" to "定时任务",
        "cron" to "定时任务",
        "git" to "Git操作",
        "docker" to "Docker操作",
        "database" to "数据库操作",
        "api" to "API调用",
        "json_parse" to "JSON解析",
        "csv_parse" to "CSV解析",
        "xml_parse" to "XML解析",
        "markdown" to "Markdown处理",
        "diff" to "差异对比",
        "patch" to "补丁应用",
        "compress" to "压缩",
        "decompress" to "解压缩",
        "encrypt" to "加密",
        "decrypt" to "解密",
        "hash" to "哈希计算",
        "base64_encode" to "Base64编码",
        "base64_decode" to "Base64解码",
        "image_process" to "图片处理",
        "audio_process" to "音频处理",
        "pdf_process" to "PDF处理",
        "spreadsheet" to "电子表格",
        "chart" to "图表生成",
        "diagram" to "图表绘制",
        "uml" to "UML生成",
        "swagger" to "API文档",
        "openapi" to "OpenAPI",
        "terraform" to "基础设施",
        "ansible" to "配置管理",
        "kubernetes" to "K8s操作",
        "helm" to "Helm操作",
        "ssh" to "SSH连接",
        "scp" to "文件传输",
        "rsync" to "文件同步",
        "wget" to "下载文件",
        "curl" to "请求URL",
        "ping" to "网络检测",
        "nslookup" to "DNS查询",
        "traceroute" to "路由追踪"
    )

    /**
     * 获取工具的中文名称
     * 如果没有映射，返回原始英文名
     */
    fun getCnName(toolName: String): String {
        return nameMap[toolName] ?: toolName
    }

    /**
     * 获取工具的分类
     */
    fun getCategory(toolName: String): String {
        return when {
            toolName in setOf("shell_exec", "execute_shell", "execute_python", "python_exec", "bash") -> "执行"
            toolName.startsWith("read_") || toolName.startsWith("write_") || toolName.startsWith("edit_") ||
                toolName.startsWith("list_") || toolName.startsWith("create_") || toolName.startsWith("delete_") ||
                toolName in setOf("grep", "find", "search_files", "file_search", "move_file", "copy_file") -> "文件"
            toolName in setOf("get_screen", "click_element", "long_click_element", "double_click_element",
                "type_text", "scroll_screen", "swipe_screen", "press_key", "screenshot", "take_screenshot") -> "屏幕"
            toolName in setOf("launch_app", "list_apps", "install_app", "uninstall_app") -> "应用"
            toolName in setOf("ocr_region", "ocr") -> "识别"
            toolName in setOf("copy_text", "paste_text", "cut_text", "select_all_text", "clear_text") -> "剪贴板"
            toolName.startsWith("canvas_") -> "画布"
            toolName.startsWith("video_") -> "视频"
            toolName.startsWith("web_") || toolName.startsWith("http_") || toolName.startsWith("fetch_") -> "网络"
            toolName.startsWith("knowledge_") || toolName == "diagnose_knowledge" -> "知识"
            toolName.startsWith("memory_") -> "记忆"
            toolName.startsWith("todo") || toolName.startsWith("task_") -> "任务"
            else -> "其他"
        }
    }
}
