---
name: phone-control
description: 手机自动化技能 - 控制手机执行点击、输入、滑动、按键等操作
trigger:
  - contains: 点击
  - contains: 打开
  - contains: 输入
  - contains: 滑动
  - contains: 截屏
  - contains: 查看屏幕
  - contains: 复制
  - contains: 粘贴
  - contains: 截图
tools:
  - name: get_screen
    description: 获取当前屏幕信息（UI树 + 截图 + OCR文字识别）
    parameters:
      type: object
      properties:
        mode:
          type: string
          description: 提取模式：som(默认)/vision/ax
          default: som
      required: []
    enabled: true
  
  - name: click_element
    description: 点击屏幕指定坐标
    parameters:
      type: object
      properties:
        x:
          type: integer
          description: X坐标
        y:
          type: integer
          description: Y坐标
      required:
        - x
        - y
    enabled: true
  
  - name: long_click_element
    description: 长按屏幕指定坐标
    parameters:
      type: object
      properties:
        x:
          type: integer
          description: X坐标
        y:
          type: integer
          description: Y坐标
      required:
        - x
        - y
    enabled: true
  
  - name: double_click_element
    description: 双击屏幕指定坐标
    parameters:
      type: object
      properties:
        x:
          type: integer
          description: X坐标
        y:
          type: integer
          description: Y坐标
      required:
        - x
        - y
    enabled: true
  
  - name: type_text
    description: 在当前输入框输入文字（自动fallback：ACTION_SET_TEXT→剪贴板粘贴→shell命令）
    parameters:
      type: object
      properties:
        text:
          type: string
          description: 要输入的文字
      required:
        - text
    enabled: true
  
  - name: scroll_screen
    description: 滑动屏幕
    parameters:
      type: object
      properties:
        direction:
          type: string
          description: 滑动方向：up/down/left/right
          enum: [up, down, left, right]
      required:
        - direction
    enabled: true
  
  - name: swipe_screen
    description: 从一个坐标滑动到另一个坐标
    parameters:
      type: object
      properties:
        start_x:
          type: integer
          description: 起点X坐标
        start_y:
          type: integer
          description: 起点Y坐标
        end_x:
          type: integer
          description: 终点X坐标
        end_y:
          type: integer
          description: 终点Y坐标
      required:
        - start_x
        - start_y
        - end_x
        - end_y
    enabled: true
  
  - name: press_key
    description: 按系统按键（支持全局操作和模拟按键）
    parameters:
      type: object
      properties:
        key:
          type: string
          description: >-
            按键名称：
            全局操作: home/back/recents/power/notifications/quick_settings/lock
            模拟按键: volume_up/volume_down/enter/delete/tab/escape/space
          enum:
            - home
            - back
            - recents
            - power
            - notifications
            - quick_settings
            - lock
            - volume_up
            - volume_down
            - enter
            - delete
            - tab
            - escape
            - space
      required:
        - key
    enabled: true
  
  - name: launch_app
    description: 启动指定应用
    parameters:
      type: object
      properties:
        package:
          type: string
          description: 应用包名
      required:
        - package
    enabled: true
  
  - name: list_apps
    description: 列出已安装的第三方应用
    parameters:
      type: object
      properties: {}
      required: []
    enabled: true
  
  - name: ocr_region
    description: 对屏幕指定区域进行OCR文字识别
    parameters:
      type: object
      properties:
        x:
          type: integer
          description: 区域左上角X坐标
        y:
          type: integer
          description: 区域左上角Y坐标
        width:
          type: integer
          description: 区域宽度
        height:
          type: integer
          description: 区域高度
      required:
        - x
        - y
        - width
        - height
    enabled: true
  
  - name: copy_text
    description: 复制当前选中的文字
    parameters:
      type: object
      properties: {}
      required: []
    enabled: true
  
  - name: paste_text
    description: 粘贴剪贴板内容到当前输入框
    parameters:
      type: object
      properties: {}
      required: []
    enabled: true
  
  - name: cut_text
    description: 剪切当前选中的文字
    parameters:
      type: object
      properties: {}
      required: []
    enabled: true
  
  - name: select_all_text
    description: 全选当前输入框中的文字
    parameters:
      type: object
      properties: {}
      required: []
    enabled: true
  
  - name: clear_text
    description: 清除当前输入框中的文字（全选+删除）
    parameters:
      type: object
      properties: {}
      required: []
    enabled: true
---

# Phone Control Skill

## 功能说明

控制手机执行各种自动化操作，包括点击、输入、滑动、按键、启动APP、OCR识别等。

## 工具清单（16个）

| 类别 | 工具 | 说明 |
|------|------|------|
| 屏幕 | get_screen | 获取UI树+截图+OCR |
| 屏幕 | ocr_region | 区域OCR文字识别 |
| 点击 | click_element | 点击坐标 |
| 点击 | long_click_element | 长按坐标 |
| 点击 | double_click_element | 双击坐标 |
| 输入 | type_text | 输入文字（自动fallback） |
| 滑动 | scroll_screen | 方向滚动 |
| 滑动 | swipe_screen | 坐标间滑动 |
| 按键 | press_key | 系统按键+模拟按键 |
| 应用 | launch_app | 启动应用 |
| 应用 | list_apps | 列出应用 |
| 文本 | copy_text | 复制 |
| 文本 | paste_text | 粘贴 |
| 文本 | cut_text | 剪切 |
| 文本 | select_all_text | 全选 |
| 文本 | clear_text | 清除 |

## 使用场景

- 用户说"点击屏幕某个位置"
- 用户说"打开某个APP"
- 用户说"输入文字"
- 用户说"查看当前屏幕"
- 用户说"返回上一页"
- 用户说"打开通知"
- 用户说"锁屏"
- 用户说"复制/粘贴文字"

## 工作流程

1. 用户提出操作需求
2. Agent解析意图，确定工具和参数
3. 调用对应工具
4. AccessibilityService执行操作
5. 返回执行结果

## 权限要求

**必须开启无障碍服务**：
- 设置 → 无障碍 → 鹏Agent → 开启

## 示例

### 示例1：点击操作
用户: "点击屏幕中间"
Agent:
- 获取屏幕尺寸
- 计算中心坐标 (width/2, height/2)
- 调用 click_element(x=540, y=960)
- 返回: 已点击屏幕中心

### 示例2：打开APP
用户: "打开微信"
Agent:
- 调用 list_apps() 查找微信包名
- 找到: com.tencent.mm
- 调用 launch_app(package="com.tencent.mm")
- 返回: 已启动微信

### 示例3：输入文字
用户: "输入'你好'"
Agent:
- 确认当前有输入框焦点
- 调用 type_text(text="你好")
- 返回: 已输入文字（自动选择最佳输入方式）

### 示例4：查看屏幕
用户: "查看当前屏幕"
Agent:
- 调用 get_screen(mode="som")
- 返回: UI树结构 + OCR文字 + 截图

### 示例5：返回
用户: "返回上一页"
Agent:
- 调用 press_key(key="back")
- 返回: 已按下返回键

### 示例6：打开通知
用户: "打开通知栏"
Agent:
- 调用 press_key(key="notifications")
- 返回: 已打开通知栏
