# brain-core：核心库

## 概述

`brain-core` 是整个系统的基础库，提供所有核心功能。其他三个 crate 都依赖它。

## 模块结构

```
brain-core/
├── config.rs          # BrainConfig：配置管理
├── error.rs           # Error 类型定义
├── models/            # 数据模型
│   ├── event.rs       # Event
│   ├── entity.rs      # Entity
│   ├── tag.rs         # Tag
│   ├── task.rs        # PipelineTask
│   └── raw_data.rs    # RawDataRef
├── db/                # 数据库层
│   ├── connection.rs  # Database (SQLite 连接管理)
│   ├── migrations.rs  # 数据库迁移/Schema
│   ├── event_repo.rs # EventRepository
│   ├── entity_repo.rs # EntityRepository
│   └── tag_repo.rs    # TagRepository
├── markdown/          # Markdown 处理
│   ├── parser.rs      # EventParser / EntityParser
│   └── serializer.rs  # EventSerializer / EntitySerializer
├── adapters/         # AI 模型适配器
│   ├── model_adapter.rs  # ModelAdapter trait
│   ├── ollama_adapter.rs # Ollama 实现
│   ├── openai_adapter.rs # OpenAI 实现
│   └── minimax_adapter.rs # MiniMax 实现
├── dicts/             # 字典系统
│   └── dicts.rs       # DictSet / Dict / DictEntry
└── logging/           # 日志系统
    ├── logger.rs      # Logger
    ├── log.rs         # LogEntry
    └── repo.rs        # LogRepository
```

## 核心组件

### 1. BrainConfig（配置）

配置文件路径：`~/.config/secondbrain/brain/brain.yaml` 或 `BRAIN_CONFIG_PATH` 环境变量

**关键路径配置：**
- `db_path` — SQLite 数据库位置
- `events_path` — Event Markdown 文件目录
- `entities_path` — Entity Markdown 文件目录
- `raw_data_path` — 原始数据存储目录
- `pipeline_queue_path` — AI 任务队列目录
- `dicts_path` — 字典文件目录
- `log_db_path` — 日志数据库目录

**AI 适配器配置：**
```yaml
adapters:
  - type: ollama
    endpoint: http://localhost:11434
    model: qwen3.5:9b-q4_K_M
```

### 2. 数据模型

**Event** — 事件
- `id`：格式 `evt-YYYYMMDD-HHMMSS-xxx`
- `type`：事件类型（meeting, photo, note, task, research 等）
- `time`：开始时间、结束时间、时区
- `source`：来源设备、渠道、采集代理
- `entities`：关联实体（人物、项目、概念等 14 类）
- `tags`：标签数组
- `ai`：AI 生成的摘要、主题、情感
- `raw_refs`：原始数据文件引用
- `derived_refs`：衍生数据（转录文本、嵌入向量）

**Entity** — 实体
- `id`：格式 `person-john`、`org-acme`、`concept-xxx`
- `type`：实体类型（Person, Organization, Project, Concept 等 14 种）
- `label`：显示名称
- `aliases`：别名数组
- `status`：Active / Archived / Merged
- `classification`：领域、父子分类
- `identity`：描述、摘要
- `metrics`：事件计数、最近活动时间、活跃度分数

### 3. Database 与 Repository

**Database** 封装 SQLite 连接，提供线程安全的连接访问。

**Repository** 模式提供数据访问接口：

| Repository | 操作 |
|------------|------|
| EventRepository | upsert, delete, find_by_id, search (FTS), find_by_time_range, all |
| EntityRepository | upsert, delete, find_by_id, find_by_type, search, all |
| TagRepository | get_for_event, all |

**upsert 流程（以 Event 为例）：**
1. INSERT OR REPLACE into `events` 表
2. 更新 `events_fts` FTS5 虚拟表（全文搜索）
3. 更新 `event_entities` 表（实体关联）
4. 更新 `tags` 表（标签）

### 4. Markdown Parser / Serializer

**Parser** 负责从 Markdown 提取数据：
- 解析 YAML frontmatter（`---` 分隔）
- 转换为 Rust 结构体

**Serializer** 负责将结构体写入 Markdown：
- 生成 YAML frontmatter
- 保留 markdown 内容区域

### 5. AI Adapter 模式

```
┌─────────────────────┐
│   ModelAdapter      │  ← Trait，定义 AI 接口
│   (trait)           │
└──────────┬──────────┘
           │
     ┌─────┴─────┬─────────────┐
     ▼           ▼             ▼
┌─────────┐ ┌─────────┐ ┌──────────┐
│ Ollama  │ │ OpenAI  │ │ MiniMax  │
│Adapter  │ │Adapter  │ │ Adapter  │
└─────────┘ └─────────┘ └──────────┘
```

**ModelAdapter Trait 定义：**
- `analyze(input)` — 分析原始数据，返回摘要、类型、标签、实体等
- `summarize(text)` — 文本摘要
- `embed(text)` — 生成文本嵌入向量

**双阶段分析流程：**
1. **Stage 1**：自由分析，AI 自由发挥提取信息
2. **Stage 2**：字典对齐，将结果与已有字典匹配，保证术语一致性

### 6. 字典系统 (DictSet)

用于保持术语一致性，包含：
- `device` — PC, iPhone, Android, Server
- `channel` — CLI, API, Web
- `capture_agent` — manual_entry, pipeline, sync_service
- `event_type` — note, task, research, photo
- `event_subtype` — summarize, reasoning, image_caption
- `tags` — AI, Rust, project
- `topics` — machine_learning, programming, life

AI 分析后可能发现新术语，会追加到字典文件中。

### 7. 日志系统

**Logger** 提供结构化日志记录：
- `log_event_crud` — Event 的增删改操作
- `log_entity_crud` — Entity 的增删改操作
- `log_ai_processing` — AI 处理耗时和结果
- `log_pipeline_task` — 任务状态流转
- `log_ingest_file` — 文件摄入记录

日志支持按月或按周轮转，存储在独立的 SQLite 数据库中。
