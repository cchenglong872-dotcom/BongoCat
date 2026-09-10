---
name: Scheduled Shutdown-skill
description: 使用 Python 脚本调用 Windows shutdown 命令，实现定时关机与取消关机。
---

# 定时关机 Skill

## Quick Start

- 设置定时关机：`python shutdown_tool.py <秒数>`，例如 `python shutdown_tool.py 600` 表示 10 分钟后关机。
- 取消定时关机：`python shutdown_tool.py cancel`。

## CLI 用法

| 命令 | 说明 |
|---|---|
| `python shutdown_tool.py <秒数>` | 设置 N 秒后关机 |
| `python shutdown_tool.py cancel` | 取消已设置的定时关机 |

## 说明

- 仅支持 Windows（依赖系统 `shutdown` 命令）。
- 秒数必须为正整数。
