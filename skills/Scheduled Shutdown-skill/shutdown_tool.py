#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""定时关机工具

CLI 用法：
  python shutdown_tool.py <秒数>   # 设置 N 秒后关机
  python shutdown_tool.py cancel    # 取消已设置的定时关机
"""

import subprocess
import sys


def format_duration(seconds: int) -> str:
    if seconds >= 60:
        minutes, secs = divmod(seconds, 60)

        return f"{minutes} 分钟" if secs == 0 else f"{minutes} 分 {secs} 秒"

    return f"{seconds} 秒"


def schedule_shutdown(seconds: int) -> str:
    subprocess.run(["shutdown", "/s", "/t", str(seconds)], check=True, capture_output=True)

    return f"已设置 {format_duration(seconds)}后关机"


def cancel_shutdown() -> str:
    result = subprocess.run(["shutdown", "/a"], capture_output=True)

    # 1116 = "没有关机过程"，视为当前没有待取消的关机
    if result.returncode not in (0, 1116):
        result.check_returncode()

    return "已取消定时关机"


def main() -> int:
    if len(sys.argv) < 2:
        print("用法: python shutdown_tool.py <秒数|cancel>", file=sys.stderr)
        return 2

    arg = sys.argv[1]

    try:
        if arg == "cancel":
            print(cancel_shutdown())
        else:
            seconds = int(arg)

            if seconds <= 0:
                print("秒数必须为正整数", file=sys.stderr)
                return 1

            print(schedule_shutdown(seconds))

        return 0
    except (subprocess.CalledProcessError, ValueError) as err:
        print(f"执行关机命令失败: {err}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
