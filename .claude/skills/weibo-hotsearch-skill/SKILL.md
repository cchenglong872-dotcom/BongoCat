---
name: weibo-hotsearch-skill
description: 当用户想获取每日微博热搜榜前 N 条（如「获取热搜前 10 条 / 前 5 条」）并逐条总结每条热点内容时使用。无需微博账号。Use when the user wants to fetch Weibo hot search top N and summarize each entry.
---

# 微博热搜 Skill

## 何时使用

当用户想查询或获取每日微博热搜榜时（如「获取热搜前 10 条」「热搜前 5 条」「今日热搜」），使用本技能。

## 使用步骤

1. 运行本技能目录下的脚本拉取热搜，用 `--limit` 指定条数（支持任意 N 条）：

   ```bash
   python weibo_hotsearch.py --limit 10
   ```

   （脚本与本 SKILL.md 同目录，即 `<技能目录>/weibo_hotsearch.py`。）

2. 脚本返回 JSON 数组，每个元素含 `rank`（排名）、`title`（标题）、`hot_value`（热度值）、`label`（标签）。

3. 对每一条热搜**逐条总结热点内容**：
   - 标题明确且你已知晓的，直接给出 1-2 句准确总结；
   - 突发新闻或陌生话题，用 WebSearch 搜索标题抓取最新详情后再总结。

4. 以 Markdown 列表输出：排名 + 标题 + 热度 + 总结（可附来源链接）。

## 参数

- `--limit`：返回条数，默认 10。
- `--format`：`json`（默认）或 `text`。

## 说明

- 无需微博账号，使用微博 PC 端公开接口。
- 传输层用 curl（微博 WAF 会拦截 Python 的 TLS 握手），已过滤广告位与政务置顶条目。
