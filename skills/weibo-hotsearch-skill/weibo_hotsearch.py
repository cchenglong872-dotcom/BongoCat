"""
微博热搜获取脚本（无需账号）

使用微博 PC 端公开接口获取实时热搜榜，支持返回任意条数。
由于微博 WAF 会拦截 Python(OpenSSL) 的 TLS 握手，故传输层改用 curl
（Windows 自带 curl.exe，走 Schannel，可正常通过），Python 只负责解析。

用法：
  python weibo_hotsearch.py                # 默认前 10 条，输出 JSON
  python weibo_hotsearch.py --limit 5      # 前 5 条
  python weibo_hotsearch.py --limit 10 --format text
"""

import argparse
import json
import subprocess
import sys
import time

# 微博 PC 端热搜接口（无需 Cookie/登录，需带 UA + Referer 头）
HOT_SEARCH_URL = "https://weibo.com/ajax/side/hotSearch"

USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
)


def _fetch_raw() -> bytes:
    """用 curl 抓取热搜接口的原始字节流。失败抛出异常。"""
    cmd = [
        "curl", "-sS", "-m", "15",
        "-A", USER_AGENT,
        "-H", "Referer: https://weibo.com/hot/search",
        HOT_SEARCH_URL,
    ]
    proc = subprocess.run(cmd, capture_output=True)
    if proc.returncode != 0:
        err = proc.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"curl 失败(exit {proc.returncode}): {err}")
    return proc.stdout


def _to_int(value):
    """将热度值统一转为 int，失败则原样返回。"""
    try:
        return int(value)
    except (TypeError, ValueError):
        return value


def fetch_hot_search(limit: int = 10) -> list[dict]:
    """
    拉取微博实时热搜榜前 limit 条。

    返回结构化列表，每个元素包含：
      - rank:      排名（realpos，从 1 开始）
      - title:     标题（note，已去掉话题 # 符号）
      - hot_value: 热度值（num）
      - label:     标签（如 热 / 新 / 爆，可能为空）
    失败时返回 [{"error": "..."}]。
    """
    last_error = "未知错误"
    for attempt in range(3):  # 最多重试 3 次
        try:
            data = json.loads(_fetch_raw().decode("utf-8"))
            realtime = data.get("data", {}).get("realtime", [])
            if not realtime:
                raise RuntimeError("响应中无实时热搜数据")

            items = []
            for item in realtime:
                # 过滤广告位：含 monitors / adid / is_ad 的条目跳过
                if item.get("monitors") or item.get("adid") or item.get("is_ad"):
                    continue
                # 无 realpos 的条目（通常也是广告）跳过
                realpos = item.get("realpos")
                if realpos is None:
                    continue
                items.append({
                    "rank": realpos,
                    "title": item.get("note") or item.get("word") or item.get("word_scheme"),
                    "hot_value": _to_int(item.get("num")),
                    "label": item.get("label_name") or "",
                })

            # 按排名升序排序，取前 limit 条
            items.sort(key=lambda x: x["rank"])
            return items[:limit]
        except Exception as e:  # noqa: BLE001 - 兜底所有异常，统一重试
            last_error = str(e)
            if attempt == 2:  # 最后一次仍失败
                return [{"error": last_error}]
            time.sleep(2)  # 等待 2 秒后重试

    return [{"error": last_error}]


def main() -> None:
    # Windows 下确保中文以 UTF-8 输出，避免 GBK 编码报错
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")

    parser = argparse.ArgumentParser(description="获取微博热搜榜前 N 条（无需账号）")
    parser.add_argument("--limit", type=int, default=10, help="返回条数，默认 10")
    parser.add_argument("--format", choices=["json", "text"], default="json", help="输出格式，默认 json")
    args = parser.parse_args()

    if args.limit <= 0:
        print("错误：--limit 必须为正整数", file=sys.stderr)
        sys.exit(1)

    items = fetch_hot_search(args.limit)

    if args.format == "json":
        print(json.dumps(items, ensure_ascii=False, indent=2))
        return

    # text 格式：人类可读
    for it in items:
        if "error" in it:
            print(f"获取失败: {it['error']}")
            continue
        hot = it.get("hot_value")
        hot_str = f"{hot:,}" if isinstance(hot, int) else str(hot)
        label = f" [{it['label']}]" if it.get("label") else ""
        print(f"{it['rank']}. {it['title']}{label} 热度 {hot_str}")


if __name__ == "__main__":
    main()
