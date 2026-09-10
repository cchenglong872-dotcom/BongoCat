# 微博热搜 Skill

获取每日微博热搜榜前 N 条（无需账号），并逐条总结每条热搜的热点内容。

## 快速使用

在 Claude Code 中对模型说：

- 「获取热搜前 10 条」→ 拉取并总结前 10 条
- 「获取热搜前 5 条」→ 拉取并总结前 5 条

## 目录结构

```
weibo-hotsearch-skill/
  ├── SKILL.md             # 技能说明（本目录的入口）
  └── weibo_hotsearch.py   # 拉取脚本（支持 --limit N）
```

## 脚本用法

```bash
python weibo_hotsearch.py --limit 10          # 前 10 条，JSON
python weibo_hotsearch.py --limit 5 --format text
```

## 说明

- 无需微博账号，走微博 PC 端公开接口。
- 传输层用 curl（微博 WAF 会拦截 Python 的 TLS 握手），已过滤广告位与政务置顶条目。
