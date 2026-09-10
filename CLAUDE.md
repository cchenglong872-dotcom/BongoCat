# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

BongoCat 是一个基于 Tauri v2 的跨平台桌面应用，在屏幕上显示一只根据键盘、鼠标和手柄操作做出实时反应的 Live2D 动画猫咪。支持 macOS、Windows 和 Linux(x11)。

## 常用命令

### 开发

```bash
# 安装依赖（仅允许 pnpm）
pnpm install

# 启动 Tauri 开发模式（同时启动 Vite 前端 + Rust 后端）
pnpm tauri dev

# 仅启动 Vite 前端（用于前端独立调试）
pnpm dev
```

### 构建

```bash
# 生产构建
pnpm tauri build

# 带调试符号的构建
pnpm tauri build --debug

# 仅构建前端（Vite）
pnpm build
```

### 代码检查与发布

```bash
# ESLint 检查并自动修复
pnpm lint

# 发布（触发 release-it 工作流）
pnpm release
```

### 其他

```bash
# 预览 Vite 构建结果
pnpm preview

# 生成 Tauri 应用图标
pnpm build:icon
```

## 技术栈架构

### 前端（`src/`）

| 层 | 技术 |
|---|---|
| 框架 | Vue 3 (Composition API) + TypeScript |
| 构建 | Vite 6 |
| 状态管理 | Pinia + `@tauri-store/pinia`（自动持久化到 Tauri Store） |
| 路由 | Vue Router (Hash 模式)，两个路由：`/` 和 `/preference` |
| UI | Ant Design Vue Next (`antdv-next`) + UnoCSS（Wind3 + Icons + Antd 预设） |
| 渲染 | PixiJS 8 + `easy-live2d` 渲染 Live2D 模型 |
| 国际化 | vue-i18n，支持 zh-CN、zh-TW、en-US、vi-VN、pt-BR |
| 工具库 | `@vueuse/core`、`es-toolkit`、`dayjs` |

### Rust 后端（`src-tauri/`）

| 层 | 技术 |
|---|---|
| 框架 | Tauri 2 |
| 输入捕获 | `rdev`（键盘/鼠标）、`gilrs`（手柄） |
| 自定义插件 | `tauri-plugin-custom-window`（窗口管理）、`tauri-plugin-admin-status`（管理员检测） |
| macOS 面板 | `tauri-nspanel`（将主窗口作为浮动面板嵌入桌面） |

### 双窗口架构

应用有**两个窗口**：

1. **`main`** — 猫咪覆盖层：透明、无边框、无任务栏图标、始终置顶。渲染 Live2D 模型并通过全局输入事件驱动猫咪动作。
2. **`preference`** — 设置面板：一个带侧边栏导航的常规窗口（猫咪、通用、模型、快捷键、关于）。默认隐藏，通过托盘菜单打开。

### Tauri 插件（自定义，位于 `src-tauri/src/plugins/`）

- **`custom-window`** — 跨平台窗口操作：`show_window`、`hide_window`、`set_always_on_top`、`set_taskbar_visibility`。在 macOS 上通过 `objc`/`ns_window` 调用，在 Windows 上通过 Win32 API 调用，在 Linux 上通过 X11 调用。
- **`admin-status`** — 检测应用是否以管理员/root 权限运行。

## 核心数据流

```
OS 输入事件 → Rust (rdev/gilrs) → Tauri 事件 → 前端 composables → Live2D 参数
```

1. **`src-tauri/src/core/device.rs`** — 使用 `rdev` 监听全局键盘/鼠标事件，以 `device-changed` 事件发送至前端。
2. **`src-tauri/src/core/gamepad.rs`** — 使用 `gilrs` 监听手柄事件（仅在模型模式为 `gamepad` 时启用），以 `gamepad-changed` 事件发送。
3. **`src/composables/useDevice.ts`** — 监听 `device-changed` 事件，将按键映射至 Live2D 参数（通过 `useModel`），处理鼠标移动插值、自动释放（Windows）、悬停隐藏等功能。
4. **`src/composables/useModel.ts`** — 核心协调器。加载 Live2D 模型，将按键名称映射至对应的键盘按键图片，控制手部按下/释放状态，并将鼠标位置转换为 Live2D 参数（`ParamMouseX`、`ParamAngleX` 等）。
5. **`src/utils/live2d.ts`** — `easy-live2d` + PixiJS 的 singleton 封装。管理模型生命周期（加载、缩放、销毁）、动作/表情触发以及参数设置。

### Rust 命令（前端通过 `invoke` 调用）

| 命令 | 用途 |
|---|---|
| `start_device_listening` | 开始全局键盘/鼠标监听 |
| `start_gamepad_listing` | 开始手柄输入轮询 |
| `stop_gamepad_listing` | 停止手柄输入轮询 |
| `copy_dir` | 递归复制目录（用于模型导入） |

## 目录结构要点

- **`src/pages/main/`** — 猫咪覆盖层页面。所有 cat store 属性的 `watch` 效应汇聚于此。
- **`src/pages/preference/`** — 设置窗口。通过 `menus` 计算属性实现基于侧边栏的标签页导航。
- **`src/stores/`** — Pinia stores：`app`、`cat`（模型行为与窗口状态）、`model`（当前模型数据与按键映射）、`general`（外观、语言、开机自启）、`shortcut`（全局快捷键绑定）。
- **`src/components/`** — 可复用组件：`pro-list`、`shortcut` 绑定 UI、`update-app` 通知。
- **`src/constants/index.ts`** — 事件键名、调用命令键名、语言选项、窗口标签的集中定义。
- **`src/locales/`** — 各语言 JSON 翻译文件。
- **`src-tauri/src/core/setup/`** — 平台特定设置。macOS 变体将主窗口配置为 `nspanel`，使用 `tauri-nspanel` 在 Dock 层级实现桌面级浮动效果；其他平台使用空的 `common` 实现。
- **`skills/`** — 自定义 Claude 技能（每个 `<名称>-skill/` 含 `SKILL.md` + Python 脚本）。已有：`12306-skill`（车票查询）、`weather-skill`（天气）、`Scheduled Shutdown-skill`（定时关机）、`weibo-hotsearch-skill`（微博热搜前 N 条，无需账号）。

## 配置规范

- **包管理**：仅允许 pnpm（通过 `preinstall` 脚本强制执行）。
- **Commit 规范**：Conventional Commits（`@commitlint/config-conventional`）。提交前通过 `simple-git-hooks` + `lint-staged` 运行 ESLint。
- **代码风格**：`@antfu/eslint-config`，启用格式化工具和 UnoCSS 规则。Vue 属性按字母顺序排序（`vue/attributes-order`），导入语句自然排序（`perfectionist/sort-imports`）。
- **路径别名**：`@/` 映射到 `src/`（Vite 和 TypeScript 均已配置）。
- **发布**：`release-it` 配合 `.release-it.ts` 配置文件，以及基于 GitHub Actions 的多平台构建矩阵（macOS aarch64/x86_64、Windows x86_64/i686/aarch64、Linux x86_64/aarch64）。

## macOS 注意事项

- 主窗口使用 `tauri-nspanel` 创建为非激活 `NSPanel`，置于 Dock 层级，跨桌面空间保持可见。
- 在非 macOS 平台上，`src-tauri/src/core/setup/common.rs` 中的平台设置函数为空操作 — 所有桌面集成均通过 macOS 私有 API 处理。
- 应用隐藏了 Dock 图标（`set_dock_visibility(false)`），通过托盘图标运行。
- 鼠标坐标需根据窗口缩放因子进行调整（`src/composables/useDevice.ts` 中的 `scaleFactor`）。
