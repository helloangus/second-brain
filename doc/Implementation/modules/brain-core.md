# brain-core 核心库

位置：`crates/brain-core/src/`

## 模块结构

```
brain-core/src/
├── lib.rs           # 模块导出
├── error.rs         # 错误类型 (thiserror)
├── config.rs        # 配置管理 (BrainConfig)
├── dicts.rs         # 词典管理
├── adapters/        # AI 模型适配器
│   ├── mod.rs
│   ├── model_adapter.rs   # ModelAdapter trait 定义
│   ├── ollama_adapter.rs   # Ollama 适配器
│   └── openai_adapter.rs   # OpenAI 适配器
├── db/              # 数据库层
│   ├── mod.rs
│   ├── connection.rs      # Database 连接管理
│   ├── migrations.rs      # Schema 迁移
│   ├── event_repo.rs      # EventRepository
│   ├── entity_repo.rs      # EntityRepository
│   └── tag_repo.rs         # TagRepository
├── logging/         # 日志模块
│   ├── mod.rs
│   ├── log.rs             # Log 结构
│   ├── logger.rs          # Logger 实现
│   └── repo.rs            # 日志 Repository
├── markdown/        # Markdown 处理
│   ├── mod.rs
│   ├── parser.rs          # EventParser, EntityParser
│   └── serializer.rs      # EventSerializer, EntitySerializer
└── models/          # 数据模型
    ├── mod.rs
    ├── event.rs           # Event 类型
    ├── entity.rs          # Entity 类型
    ├── task.rs            # Task & PipelineTask
    ├── raw_data.rs        # RawData 类型
    └── tag.rs             # Tag 类型
```

---

## 配置管理

### BrainConfig
`config.rs`

程序启动时从配置文件加载配置：

```rust
pub struct BrainConfig {
    pub db_path: PathBuf,              // SQLite 数据库路径
    pub events_path: PathBuf,          // 事件文件目录
    pub entities_path: PathBuf,        // 实体文件目录
    pub raw_data_path: PathBuf,        // 原始数据目录
    pub pipeline_queue_path: PathBuf,  // AI 任务队列目录
    pub adapters: Vec<AdapterConfig>,  // AI 适配器配置
    pub dicts_path: PathBuf,           // 词典目录
    pub logs_path: PathBuf,            // 日志目录
}
```

**配置路径：** `config/brain.yaml` (相对路径)

---

## 数据模型

### Event
`models/event.rs`

核心实体，代表一个时间点发生的事件。

```rust
pub struct Event {
    pub id: String,                    // 格式: evt-YYYYMMDD-HHMMSS-xxx
    pub schema_version: i32,
    pub type_: EventType,
    pub time: EventTime,
    pub source: EventSource,
    pub status: String,
    pub confidence: f64,
    pub entities: EventEntities,
    pub tags: Vec<String>,
    pub raw_refs: Vec<String>,
    pub ai: EventAi,
    // ... 更多字段
}
```

**EventType 枚举：**
```rust
Meeting, Photo, Note, Activity, Research,
Reading, Exercise, Meal, Work, Other
```

### Entity
`models/entity.rs`

长期存在的对象。

```rust
pub struct Entity {
    pub id: String,                    // 格式: {type}-{slug}
    pub type_: EntityType,
    pub label: String,
    pub aliases: Vec<String>,
    pub status: EntityStatus,
    pub confidence: f64,
    pub classification: EntityClassification,
    pub identity: EntityIdentity,
    pub multimedia: EntityMultimedia,
    pub links: EntityLinks,
    pub metrics: EntityMetrics,
}
```

**实体类型 (按子目录存储)：**
```
activities, artifacts, concepts, devices, goals,
memory_clusters, organizations, people, places,
projects, resources, skills, states, topics
```

### Task & PipelineTask
`models/task.rs`

AI 处理任务定义。

```rust
pub enum TaskType {
    ImageCaption, FaceDetection, Ocr,
    Asr, SpeakerDiarization,
    Embedding, Reasoning, Summarize, Tagging,
}
```

### Tag
`models/tag.rs`

事件标签。

### RawData
`models/raw_data.rs`

原始数据类型定义。

---

## 数据库层

### Schema
`db/migrations.rs`

```sql
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    schema_version INTEGER DEFAULT 1,
    time_start INTEGER NOT NULL,
    time_end INTEGER,
    timezone TEXT DEFAULT 'UTC',
    type TEXT NOT NULL,
    subtype TEXT,
    source_device TEXT,
    source_channel TEXT,
    source_capture_agent TEXT,
    status TEXT DEFAULT 'auto',
    confidence REAL DEFAULT 0.5,
    ai_summary TEXT,
    ai_topics TEXT,
    ai_sentiment TEXT,
    extraction_version INTEGER,
    importance REAL,
    recurrence INTEGER DEFAULT 0,
    created_at INTEGER,
    ingested_at INTEGER,
    updated_at INTEGER
);

CREATE TABLE entities (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    label TEXT NOT NULL,
    aliases TEXT,
    status TEXT DEFAULT 'active',
    confidence REAL DEFAULT 0.5,
    classification_domain TEXT,
    classification_parent TEXT,
    identity_description TEXT,
    summary TEXT,
    images TEXT,
    voices TEXT,
    embeddings_text TEXT,
    links_wikipedia TEXT,
    links_papers TEXT,
    merged_from TEXT,
    split_to TEXT,
    event_count INTEGER DEFAULT 0,
    last_seen INTEGER,
    activity_score REAL,
    created_at INTEGER,
    updated_at INTEGER
);

CREATE TABLE event_entities (
    event_id TEXT,
    entity_id TEXT,
    entity_type TEXT,
    relation TEXT,
    PRIMARY KEY (event_id, entity_id, relation)
);

CREATE TABLE tags (
    event_id TEXT,
    tag TEXT,
    confidence REAL DEFAULT 1.0,
    PRIMARY KEY (event_id, tag)
);

CREATE VIRTUAL TABLE events_fts USING fts5(id, ai_summary, content);

CREATE INDEX idx_events_time_start ON events(time_start);
CREATE INDEX idx_events_type ON events(type);
CREATE INDEX idx_entities_type ON entities(type);
CREATE INDEX idx_entities_label ON entities(label);
```

### Repositories

**EventRepository** (`db/event_repo.rs`):

| 方法 | 说明 |
|------|------|
| `upsert(event)` | 插入或更新事件，同时更新 FTS 和关联 |
| `delete(id)` | 删除事件及所有关联 |
| `find_by_id(id)` | 按 ID 查找 |
| `search(keyword)` | FTS5 全文搜索 |
| `find_by_time_range(start, end)` | 时间范围查询 |
| `find_by_date(date)` | 按日期查询 |
| `all()` | 获取所有事件 |

**EntityRepository** (`db/entity_repo.rs`):

| 方法 | 说明 |
|------|------|
| `upsert(entity)` | 插入或更新实体 |
| `delete(id)` | 删除实体及关联 |
| `find_by_id(id)` | 按 ID 查找 |
| `find_by_type(type)` | 按类型查找 |
| `search(keyword)` | 按标签搜索 |
| `all()` | 获取所有实体 |

**TagRepository** (`db/tag_repo.rs`):

| 方法 | 说明 |
|------|------|
| `get_all()` | 获取所有标签 |
| `get_top_tags(limit)` | 获取热门标签 |

**Database** (`db/connection.rs`):
- `open(path)` - 打开或创建数据库
- `connection()` - 获取连接（线程安全）
- 自动运行迁移

---

## Markdown 处理

### 格式示例

```markdown
---
schema: event/v1
id: evt-20260401-143022-a3f
type: meeting
time:
  start: 2026-04-01T10:00:00+09:00
  end: 2026-04-01T11:00:00+09:00
  timezone: Asia/Tokyo
status: manual
confidence: 0.9
entities:
  people:
    - person-zhangsan
  topics:
    - project-x
tags:
  - work
  - meeting
ai:
  summary: 讨论项目进度
---
```

### Parser
`markdown/parser.rs`

```rust
pub struct EventParser;
pub struct EntityParser;

let event = EventParser::parse(content)?;
let entity = EntityParser::parse(content)?;
```

### Serializer
`markdown/serializer.rs`

```rust
pub struct EventSerializer;
pub struct EntitySerializer;

let yaml = EventSerializer.serialize(&event)?;
```

---

## AI 适配器

### ModelAdapter Trait
`adapters/model_adapter.rs`

统一接口：

```rust
pub trait ModelAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn supported_data_types(&self) -> Vec<RawDataType>;
    fn supports(&self, data_type: &RawDataType) -> bool;
    fn analyze(&self, input: &RawDataInput) -> Result<AnalysisOutput>;
    fn summarize(&self, text: &str) -> Result<String>;
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn health_check(&self) -> Result<bool>;
}
```

### OllamaAdapter
`adapters/ollama_adapter.rs`

本地 Ollama 服务适配器。

**配置：**
```rust
AdapterConfig::ollama("http://localhost:11434", "llama3")
```

**API 端点：**
- `POST /api/generate` - 文本生成/分析
- `POST /api/embeddings` - 向量嵌入
- `GET /api/tags` - 健康检查

### OpenAIAdapter
`adapters/openai_adapter.rs`

**状态：** 结构已定义，实现待完成

---

## 日志模块

`logging/`

| 文件 | 说明 |
|------|------|
| `log.rs` | Log 结构定义 |
| `logger.rs` | Logger 实现 |
| `repo.rs` | 日志 Repository |
