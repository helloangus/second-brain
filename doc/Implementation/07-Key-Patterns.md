# 关键设计模式

## 1. Markdown-First（Markdown 优先）

**核心原则：** Markdown 文件是数据的最终真相，SQLite 只是索引。

```
┌─────────────────┐         ┌─────────────────┐
│  Markdown 文件   │◄───────►│   Obsidian      │
│  (Source of     │         │   (可查看编辑)   │
│   Truth)        │         └─────────────────┘
└────────┬────────┘
         │
         │ brain-core
         ▼
┌─────────────────┐
│    SQLite       │
│   (Index)       │
└────────┬────────┘
         │
         │ brain-cli / brain-pipeline
         ▼
┌─────────────────┐
│    查询/搜索     │
└─────────────────┘
```

**好处：**
- 用户可以直接用 Obsidian 查看和编辑数据
- Git 可以版本化所有数据
- 不依赖任何专有格式

**约束：**
- AI 绝不直接写 Markdown（通过 EventBuilder）
- 所有写入必须同时更新 SQLite 索引

## 2. AI Adapter 抽象

**核心思想：** AI 模型是可替换的，不绑定特定供应商。

```
┌─────────────────────────────────────┐
│        Pipeline Processor           │
│                                     │
│   let adapter = create_adapter(...);│
│   adapter.analyze(input);           │
└───────────────┬─────────────────────┘
                │
                │ ModelAdapter trait
                ▼
┌─────────────────────────────────────┐
│           ModelAdapter              │
│  ┌─────────────────────────────────┐│
│  │ fn analyze(&self, input) -> ...  ││
│  │ fn summarize(&self, text) -> ... ││
│  │ fn embed(&self, text) -> ...    ││
│  └─────────────────────────────────┘│
└───────────────┬─────────────────────┘
                │
        ┌───────┴───────┬───────────┐
        ▼               ▼           ▼
┌─────────────┐ ┌─────────────┐ ┌──────────┐
│   Ollama    │ │   OpenAI    │ │ MiniMax  │
│  Adapter    │ │   Adapter   │ │ Adapter  │
└─────────────┘ └─────────────┘ └──────────┘
```

**切换 AI 模型只需修改配置：**
```yaml
# brain.yaml
adapters:
  - type: ollama
    endpoint: http://localhost:11434
    model: qwen3.5:9b-q4_K_M
```

## 3. Event Builder 协议

**核心原则：** AI 输出 JSON，Builder 控制文件生成。

```
AI 模型
  │
  │  raw JSON (可能不稳定)
  ▼
EventBuilder
  │
  ├── 验证 AI 输出
  ├── 补充系统字段 (ID, 时间戳)
  ├── 转换/规范化数据
  │
  ▼
Event 结构体 (validated)
  │
  ▼
EventSerializer
  │
  ▼
Markdown 文件 (格式正确)
```

**好处：**
- 格式不会因 AI 输出不稳定而损坏
- 系统保持对数据格式的完全控制
- 可以添加 AI 无法提供的字段

## 4. Repository 模式

**核心思想：** 封装数据库访问，提供领域模型的持久化接口。

```rust
// 客户端代码
let repo = EventRepository::new(&conn);
repo.upsert(&event)?;

// 不直接写 SQL，而是通过 Repository
```

**Repository 封装的内容：**
- SQL 语句
- 事务处理
- 关联数据写入（Event → tags, event_entities 等）
- FTS 索引更新

## 5. 文件队列模式

**核心思想：** 用文件系统代替消息队列，实现持久化任务队列。

```
pipeline/queue/
├── pending/     # 任务文件
├── processing/  # 正在处理
└── done/        # 已完成
```

**状态转换：**
```rust
// 添加任务
std::fs::rename(
    temp_file,
    pending_path.join(task_id)
)?;

// 开始处理
std::fs::rename(
    pending_path.join(task_id),
    processing_path.join(task_id)
)?;

// 完成
std::fs::rename(
    processing_path.join(task_id),
    done_path.join(task_id)
)?;
```

**好处：**
- 进程重启后队列不丢失
- 无需运维额外的消息中间件
- 队列状态可直接检查

## 6. 字典系统（Taxonomy 一致性）

**核心思想：** 通过字典约束 AI 输出，保持术语一致性。

```
┌─────────────────────────────────────┐
│         DictSet                      │
│                                     │
│  device: [PC, iPhone, Server]       │
│  channel: [CLI, API, Web]           │
│  tags: [AI, Rust, project]          │
│  topics: [ml, programming, life]    │
└─────────────────────────────────────┘
        │
        │ Stage 2: 对齐
        ▼
┌─────────────────────────────────────┐
│  AI 原始输出     →    字典化输出      │
│  "旅游照片"          [travel, photo] │
│  "东京之旅"          [japan, travel] │
└─────────────────────────────────────┘
```

**新术语发现：**
```rust
struct NewDictEntries {
    event_types: Vec<String>,   // AI 发现的新类型
    event_subtypes: Vec<String>,
    tags: Vec<String>,         // AI 发现的新标签
    topics: Vec<String>,
}
```
- AI 分析时发现的新术语会追加到字典
- 下次分析时 AI 会考虑这些新术语
- 实现系统的「学习」能力

## 7. 双写保证一致性

**场景：** `brain add` 命令同时写入 Markdown 和 SQLite

```
brain add
    │
    ├── EventSerializer → Markdown 文件
    │
    └── EventRepository::upsert() → SQLite

    │
    ▼

brain-indexerd (守护进程)
    │
    ├── 检测到文件变化
    └── EventRepository::upsert() → SQLite (幂等)
```

**保证：**
- Markdown 是真相，SQLite 是索引
- `add` 命令直接写 SQLite（性能）
- `indexerd` 检测变化也会写（最终一致）
- upsert 是幂等的，多次执行结果相同
