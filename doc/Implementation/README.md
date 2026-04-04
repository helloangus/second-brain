# Implementation Documentation

## 概述

本目录包含 Second Brain 系统的实现文档，描述各模块的功能、协作方式和关键设计决策。

## 文档结构

| 文件 | 内容 |
|------|------|
| [01-System-Overview](01-System-Overview.md) | 系统整体架构和设计理念 |
| [02-brain-core](02-brain-core.md) | 核心库详解：模型、数据库、Markdown、AI 适配器 |
| [03-brain-cli](03-brain-cli.md) | CLI 命令行工具详解 |
| [04-brain-indexerd](04-brain-indexerd.md) | 文件系统监控守护进程 |
| [05-brain-pipeline](05-brain-pipeline.md) | AI 处理流水线详解 |
| [06-Data-Flow](06-Data-Flow.md) | 模块协作与数据流详解 |
| [07-Key-Patterns](07-Key-Patterns.md) | 关键设计模式总结 |

## 快速参考

### 四大 Crate

```
brain-src/
├── brain-core/      # 核心库（所有功能的基础）
├── brain-cli/       # CLI 工具（用户入口）
├── brain-indexerd/  # 文件监控守护进程
└── brain-pipeline/  # AI 处理流水线
```

### 核心命令

```bash
# 查询
brain search "关键词"      # 全文搜索
brain timeline 2026-03    # 月度时间线
brain today               # 今日事件
brain entity list          # 列出实体
brain entity show <id>    # 查看实体
brain stats               # 统计信息

# 添加
brain add --type meeting --summary "描述"  # 手动添加事件
brain ingest --file <path> --type image   # 摄入文件

# 处理
brain process             # 运行 AI 流水线
brain logs                # 查看日志
```

### 数据流

```
用户操作 → brain-cli → Markdown + SQLite
                    ↘
              brain-indexerd → SQLite (实时同步)

原始数据 → brain ingest → pipeline/queue/pending/
                              ↓
                        brain-pipeline → AI 分析
                              ↓
                        Event → Markdown + SQLite
```

### 关键设计

| 模式 | 说明 |
|------|------|
| Markdown-First | Markdown 是真相，SQLite 是索引 |
| AI Adapter | AI 模型可替换，通过 trait 抽象 |
| Event Builder | AI 输出 JSON，Builder 控制文件生成 |
| Repository | 封装数据库访问，提供持久化接口 |
| 文件队列 | 用文件系统代替消息队列 |
| 字典系统 | 保持 AI 输出的术语一致性 |
