# Second Brain 实现文档

本文档说明 `brain-src` 中已实现的功能模块，与设计文档（V1-V7）分开存放。

## 模块文档

### [brain-core 核心库](modules/brain-core.md)
- 数据模型（Event、Entity、Task、Tag、RawData）
- 数据库层与迁移 (SQLite)
- Markdown 解析与序列化 (YAML frontmatter)
- AI 适配器接口 (ModelAdapter trait)

### [brain-cli 命令行工具](modules/brain-cli.md)
- search - FTS5 全文搜索
- timeline - 月度时间线视图
- today - 今日事件
- add - 添加新事件
- entity - 实体管理 (list/show)
- stats - 统计信息

### [brain-indexerd 索引守护进程](modules/brain-indexerd.md)
- 文件系统监听 (notify crate)
- 自动索引更新
- 启动时全量索引

### [brain-pipeline AI处理流水线](modules/brain-pipeline.md)
- 队列管理 (pending/processing/done)
- AI 模型调用
- Event 自动生成 (EventBuilder)

## 核心设计

### 架构原则

| 原则 | 说明 |
|------|------|
| Local-first | 数据本地存储，不依赖云服务 |
| Markdown 真相 | 所有数据以 .md + YAML frontmatter 存储 |
| Git 版本控制 | events/ 和 entities/ 纳入 Git 管理 |
| AI 适配器抽象 | `ModelAdapter` trait 支持灵活切换 AI 模型 |
| Event Builder 协议 | AI 只输出 JSON，由 Builder 控制文件生成 |

### 数据流

```
┌─────────────────────────────────────────────────────────────┐
│                        数据入口                               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  CLI add ──→ markdown文件 ──→ SQLite索引                     │
│                                     ↑                        │
│  Pipeline ──→ AI分析 ──→ Event ──→ markdown ──→ SQLite      │
│                                     ↑                        │
│  indexerd ──→ 监听变更 ──→ 更新索引                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 数据库表

| 表名 | 说明 |
|------|------|
| `events` | 事件主表 |
| `entities` | 实体主表 |
| `event_entities` | 事件-实体关联 |
| `tags` | 标签 |
| `events_fts` | FTS5 全文搜索 |

## 工作空间结构

```
brain-src (workspace)
├── Cargo.toml              # Workspace 根配置
└── crates/
    ├── brain-core          # 核心库 (所有其他 crate 依赖它)
    ├── brain-cli           # CLI 工具 (依赖 brain-core)
    ├── brain-indexerd      # 索引守护进程 (依赖 brain-core)
    └── brain-pipeline      # AI 流水线 (依赖 brain-core)
```

## 相关文档

- [V1 Architecture 实现方案](../SolutionDesign/V1_IMPLEMENTATION.md) - 完整的实现规划
- [SolutionDesign/](../SolutionDesign/) - V1-V7 设计文档
