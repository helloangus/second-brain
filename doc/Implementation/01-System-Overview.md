# System Overview

## 设计理念

**Second Brain** 是一个**事件驱动的个人认知系统**。所有生活数据通过 Event 流入系统，转化为可计算的记忆。系统遵循以下核心原则：

| 原则 | 含义 |
|------|------|
| Local-first | 数据本地存储，不依赖云服务 |
| Git-based | events/ 和 entities/ 纳入 Git 版本控制 |
| Markdown 真相 | YAML frontmatter 是数据的最终真相，SQLite 只是索引 |
| AI Adapter 抽象 | AI 模型可替换，不绑定特定供应商 |
| Event Builder 协议 | AI 输出 JSON，绝不直接写 markdown |

## 核心数据模型

系统围绕两个核心实体构建：

**Event（事件）** — 时间线上的一个节点
- 时间戳（开始/结束）
- 类型（meeting, photo, note, task, research 等）
- 关联的实体（人物、项目、概念、地点等）
- AI 生成摘要、标签、主题
- 原始数据引用

**Entity（实体）** — 持久化的对象
- 人物、组织、项目、概念、话题等 14 种类型
- 标签、别名、描述
- 与事件的关联度量和活跃度

## 系统架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                         brain-cli                                │
│   (用户交互入口: search, timeline, today, add, entity, stats)    │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              │ 读写事件/实体
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        brain-core                                │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐  ┌────────────────┐  │
│  │ Database │  │ Repositor │  │ Markdown │  │ AI Adapters    │  │
│  │ (SQLite) │  │ -ies     │  │ Parser/  │  │ (Ollama/OpenAI/│  │
│  │          │  │          │  │ Serializer│  │  MiniMax)      │  │
│  └──────────┘  └───────────┘  └──────────┘  └────────────────┘  │
└─────────────────────────────┬───────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐   ┌─────────────────┐   ┌─────────────────────┐
│ brain-indexerd│   │  brain-pipeline  │   │   Markdown Files    │
│ (Daemon)      │   │  (AI Processor)  │   │   (Source of Truth) │
│               │   │                 │   │                     │
│ 文件监控      │   │ pending/        │   │ events/{year}/{mon} │
│ → 更新索引    │   │ processing/     │   │ entities/           │
│               │   │ done/           │   │                     │
└───────────────┘   └─────────────────┘   └─────────────────────┘
```

## 四大 Crate 职责

| Crate | 职责 | 关键词 |
|-------|------|--------|
| **brain-core** | 核心库：数据模型、数据库、Markdown 解析、AI 适配器 | 模型、存储、解析 |
| **brain-cli** | CLI 工具：用户命令入口 | 交互、查询、添加 |
| **brain-indexerd** | 后台守护进程：监控文件系统变化，实时更新 SQLite 索引 | 监听、同步 |
| **brain-pipeline** | AI 处理流水线：消费原始数据，调用 AI 生成 Event | AI、分析、生成 |

## 数据流向

### 手动添加 Event
```
用户 (brain add)
    │
    ▼
Markdown 文件写入 events/{year}/{month}/{id}.md
    │
    ▼
brain-indexerd 检测到变化
    │
    ▼
SQLite events 表 + FTS 索引更新
```

### AI 自动处理
```
原始数据文件 (图片/音频/文本)
    │
    ▼
用户 (brain ingest --file xxx --process)
    │
    ▼
文件复制到 data/raw/{type}/{date}/
    │
    ▼
Task 加入 pipeline/queue/pending/
    │
    ▼
brain-pipeline process (或定时任务)
    │
    ├── AI 分析 (Ollama/OpenAI/MiniMax)
    ├── 生成 Event JSON
    ├── EventBuilder 创建 Markdown
    └── 写入 events/ + 索引 SQLite
```

### 查询流程
```
用户 (brain search "关键词")
    │
    ▼
SQLite FTS 全文搜索
    │
    ▼
返回匹配 Event ID
    │
    ▼
显示结果 (时间、类型、摘要、标签)
```
