#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""用 PyInstaller 把 4 个技能脚本打包为独立 exe，输出到 src-tauri/tools/。

打包出的 exe 自带 Python 运行时，目标机无需安装 Python。

用法（在项目根目录执行）：
    python scripts/build_tools.py

产物：
    src-tauri/tools/weather_tool.exe     —— 天气查询
    src-tauri/tools/shutdown_tool.exe    —— 定时关机
    src-tauri/tools/ticket_tool.exe      —— 12306 车票查询
    src-tauri/tools/weibo_hotsearch.exe  —— 微博热搜
    src-tauri/tools/stations.json        —— 车票站名表（离线缓存）

注意：若本机 Python 过新导致 PyInstaller 打包失败，建议改用 Python 3.11/3.12 虚拟环境执行。
"""

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOOLS_DIR = ROOT / "src-tauri" / "tools"

# (脚本路径, 输出 exe 名)
TOOLS = [
    (ROOT / "skills" / "weather-skill" / "weather_tool.py", "weather_tool"),
    (ROOT / "skills" / "Scheduled Shutdown-skill" / "shutdown_tool.py", "shutdown_tool"),
    (ROOT / "skills" / "12306-skill" / "scripts" / "12306_apis.py", "ticket_tool"),
    (ROOT / "skills" / "weibo-hotsearch-skill" / "weibo_hotsearch.py", "weibo_hotsearch"),
]

STATIONS = ROOT / "skills" / "12306-skill" / "scripts" / "stations.json"


def run(cmd: list[str]) -> None:
    print(f"+ {' '.join(cmd)}")
    subprocess.run(cmd, check=True, cwd=ROOT)


def main() -> int:
    # 清理旧产物，保证幂等
    shutil.rmtree(ROOT / "build", ignore_errors=True)
    shutil.rmtree(ROOT / "dist", ignore_errors=True)

    # 1. 安装打包依赖（PyInstaller + 脚本需要的第三方库）
    run([
        sys.executable, "-m", "pip", "install", "--upgrade",
        "pyinstaller", "requests", "python-dotenv",
    ])

    # 2. 逐个打包为 onefile exe
    for script, name in TOOLS:
        run([
            sys.executable, "-m", "PyInstaller",
            "--onefile", "--noconfirm", "--clean",
            "--name", name,
            str(script),
        ])

    # 3. 收集产物到 src-tauri/tools/
    TOOLS_DIR.mkdir(parents=True, exist_ok=True)
    for _, name in TOOLS:
        shutil.copy2(ROOT / "dist" / f"{name}.exe", TOOLS_DIR / f"{name}.exe")

    shutil.copy2(STATIONS, TOOLS_DIR / "stations.json")

    # 4. 清理 PyInstaller 中间产物
    shutil.rmtree(ROOT / "build", ignore_errors=True)
    shutil.rmtree(ROOT / "dist", ignore_errors=True)
    for _, name in TOOLS:
        (ROOT / f"{name}.spec").unlink(missing_ok=True)

    print(f"\n完成：4 个工具已输出到 {TOOLS_DIR}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
