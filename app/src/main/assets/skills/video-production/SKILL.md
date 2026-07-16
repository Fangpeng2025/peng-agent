---
name: video-production
version: 6.0.0
description: 视频制作技能 — 基于HyperFrames引擎，HTML+GSAP动画渲染为MP4。分步Pipeline模式：构思→场景设计→HTML生成→渲染→展示。
enabled: true
source: user
tags: [视频, 渲染, video, 动画, hyperframes]
---

# 视频制作技能（HyperFrames Pipeline）

基于 HyperFrames 渲染引擎，将 HTML+GSAP 动画渲染为 MP4 视频。

## Pipeline: 构思 → 场景设计 → HTML生成 → 渲染 → 展示

```
用户需求 → LLM构思 → 场景设计 → HTML+GSAP代码 → video_render → 视频卡片展示
```

## 何时使用

用户要求制作/生成/渲染视频时加载此技能。

## 阶段1：构思与场景规划

**LLM负责**：
- 理解用户需求（主题、风格、时长）
- 设计2-5个场景，每个场景3-6秒
- 为每个场景设计文字内容、背景色、动画效果
- 确定视频尺寸（默认竖屏 1080x1920）

**先向用户简述构思**，不要直接跳到代码：
- 总共几个场景
- 每个场景展示什么
- 整体风格（配色、动画节奏）

## 阶段2：分步生成HTML

**⚠️ 核心原则：分步生成，避免超时**

**绝对不要一次性生成完整的 HTML+GSAP 脚本！** 一次性生成超过2000字符的代码会导致 SSE 流长时间无输出，触发超时中断。

**必须分步生成：**
1. 先用 `shell_exec` 生成 HTML 骨架（CSS+DOM元素，不含动画代码）
2. 逐场景追加 GSAP 动画代码（每次 `shell_exec` 追加一个场景）
3. 最后调用 `video_render` 渲染

### HTML骨架模板

```html
<!DOCTYPE html>
<html data-duration="18.0">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=1080">
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{width:1080px;height:1920px;overflow:hidden;background:#0a0a2e;font-family:sans-serif}
.scene{position:absolute;width:100%;height:100%;opacity:0}
</style>
</head>
<body>
<div id="scene1" class="scene" style="background:linear-gradient(135deg,#0c2d6b,#1a5276)">
  <!-- 场景1元素 -->
</div>
<div id="scene2" class="scene" style="background:linear-gradient(135deg,#1a1a2e,#2d2d44)">
  <!-- 场景2元素 -->
</div>

<script src="https://cdnjs.cloudflare.com/ajax/libs/gsap/3.12.5/gsap.min.js"></script>
<script>
// 主时间线
const tl = gsap.timeline({paused:true});

// === 动画代码将在此处逐场景追加 ===

// 注册到HyperFrames
window.__timelines = [{name:"main",timeline:tl}];
tl.play();
</script>
</body>
</html>
```

### 关键HTML规范

1. **`<html data-duration="18.0">`** — 必须设置，告诉渲染引擎视频时长
2. **`<meta name="viewport" content="width=1080">`** — 固定宽度
3. **每个场景用 `<div class="scene">`** — 初始 `opacity:0`，由GSAP控制显示
4. **GSAP时间线用 `gsap.timeline({paused:true})`** — 必须paused，渲染引擎控制播放
5. **`window.__timelines = [{name:"main",timeline:tl}]`** — 必须注册，渲染引擎通过此接口seek帧
6. **`tl.play()`** — 注册后播放

### 追加场景动画的shell_exec示例

```bash
# 追加场景1动画
sed -i '/=== 动画代码将在此处逐场景追加 ===/a\
// 场景1: 开场标题 (0-3.5s)\
tl.fromTo("#scene1",{opacity:0},{opacity:1,duration:0.3},0);\
tl.fromTo("#s1-title",{scale:0,opacity:0},{scale:1,opacity:1,duration:0.8,ease:"back.out(1.7)"},0.2);\
tl.to("#scene1",{opacity:0,duration:0.3},3.5);' /sdcard/peng-agent/videos/skeleton.html
```

## 阶段3：调用video_render

HTML完成后，调用 `video_render` 渲染：

### 方式1：用html_path指定文件路径（推荐，避免大script参数）

```json
{
  "html_path": "/sdcard/peng-agent/videos/skeleton.html",
  "output_path": "/sdcard/peng-agent/videos/my_video.mp4",
  "width": 1080,
  "height": 1920,
  "fps": 30
}
```

### 方式2：直接传script（仅短HTML适用）

```json
{
  "script": "<HTML内容>",
  "output_path": "/sdcard/peng-agent/videos/my_video.mp4"
}
```

### 参数说明

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| script | string | 否 | HTML内容，与html_path二选一 |
| html_path | string | 否 | HTML文件路径，优先于script |
| output_path | string | 否 | 输出MP4路径，留空自动生成 |
| width | int | 否 | 视频宽度，默认1080 |
| height | int | 否 | 视频高度，默认1920 |
| fps | int | 否 | 帧率，默认30 |
| duration | float | 否 | 时长(秒)，默认从HTML data-duration读取 |

**注意**：如果script和html_path都为空，但output_path是目录，会自动查找目录中最新的.html文件。

## 阶段4：展示

`video_render` 成功后，UI 自动展示视频卡片。LLM只需简短回复，如"视频已生成，请点击播放"。

如果渲染失败（如Termux环境未初始化），LLM应提示用户去设置页面初始化Termux环境。

## GSAP动画速查

### 常用动画方法

| 方法 | 说明 | 示例 |
|------|------|------|
| `tl.fromTo(target, from, to, position)` | 从A到B | `tl.fromTo("#el",{opacity:0},{opacity:1,duration:0.5},0.2)` |
| `tl.to(target, vars, position)` | 从当前到目标 | `tl.to("#el",{opacity:0,duration:0.3},3.5)` |
| `tl.from(target, vars, position)` | 从指定值到当前 | `tl.from("#el",{y:100,duration:0.5},1.0)` |

### position参数

- 数字：时间线上的秒数位置（如 `0` = 开头，`3.5` = 3.5秒处）
- `"+=0.5"`：上一个动画结束后0.5秒
- `">"`：上一个动画结束后

### 常用easing

`power2.out` / `power3.out` / `back.out(1.7)` / `elastic.out(1,0.6)` / `none`

### 典型动画效果

```javascript
// 淡入
tl.fromTo("#el",{opacity:0},{opacity:1,duration:0.5},0);

// 缩放弹入
tl.fromTo("#el",{scale:0,opacity:0},{scale:1,opacity:1,duration:0.8,ease:"back.out(1.7)"},0.2);

// 从左侧滑入
tl.fromTo("#el",{x:-200,opacity:0},{x:0,opacity:1,duration:0.6,ease:"power3.out"},0.5);

// 从底部升起
tl.fromTo("#el",{y:100,opacity:0},{y:0,opacity:1,duration:0.5,ease:"power2.out"},1.0);

// 场景转场（淡出当前场景）
tl.to("#scene1",{opacity:0,duration:0.3},3.5);
```

## 视频尺寸参考

| 类型 | 宽x高 | 比例 |
|------|-------|------|
| 竖屏短视频 | 1080x1920 | 9:16 |
| 横屏视频 | 1280x720 | 16:9 |
| 方屏视频 | 1080x1080 | 1:1 |

## 注意事项

- 渲染引擎使用 HyperFrames（Chrome Headless截图 + FFmpeg合成），需要Termux环境
- 渲染时间约视频时长的1-3倍（取决于场景复杂度和fps）
- **不要**在HTML中使用 `<img>` 加载外部URL图片（Chrome Headless无网络）
- **不要**使用 `fetch`/`XMLHttpRequest` 等网络请求
- CSS动画可以用，但必须在GSAP时间线中也有对应的seekable控制
- `data-duration` 是必须的，否则渲染引擎不知道何时停止
