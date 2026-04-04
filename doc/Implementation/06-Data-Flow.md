# 模块协作与数据流

## 整体数据流图

```
┌──────────────────────────────────────────────────────────────────────────┐
│                              用户操作                                     │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌────────────────────────────┼────────────────────────────┐
        │                            │                            │
        ▼                            ▼                            ▼
┌───────────────┐          ┌─────────────────┐          ┌─────────────────┐
│  brain-cli    │          │  brain-indexerd │          │ brain-pipeline │
│  (同步命令)    │          │  (守护进程)       │          │ (AI 处理)       │
└───────┬───────┘          └────────┬────────┘          └────────┬────────┘
        │                            │                            │
        │                            │                            │
        ▼                            ▼                            ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                           brain-core                                      │
│  ┌─────────┐  ┌───────────┐  ┌────────────┐  ┌────────────────────────┐  │
│  │ Config  │  │ Database  │  │ Markdown   │  │ AI Adapters           │  │
│  │         │  │           │  │ Parser/    │  │ (Ollama/OpenAI/       │  │
│  │         │  │           │  │ Serializer │  │  MiniMax)             │  │
│  └─────────┘  └───────────┘  └────────────┘  └────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                        存储层                                             │
│  ┌─────────────────────┐              ┌─────────────────────────────┐  │
│  │  Markdown 文件        │              │  SQLite 数据库              │  │
│  │  (Source of Truth)   │              │  (Index)                    │  │
│  │                     │              │                             │  │
│  │  events/{year}/     │              │  events 表                   │  │
│  │    {month}/         │              │  entities 表                 │  │
│  │    {id}.md          │              │  events_fts (FTS5)           │  │
│  │                     │              │  event_entities 表           │  │
│  │  entities/          │              │  tags 表                     │  │
│  │    {id}.md          │              │  logs 表                     │  │
│  └─────────────────────┘              └─────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────┘
```

## 场景一：手动添加事件

```
用户执行: brain add --type meeting --summary "项目会议"
    │
    ▼
brain-cli (add.rs)
    │
    ├── 生成事件 ID: evt-20260404-103000-xyz
    │
    ├── 创建 Event 结构体
    │     - type: meeting
    │     - time.start: 当前时间
    │     - ai.summary: "项目会议"
    │
    ├── EventSerializer::serialize() → Markdown 文本
    │
    ├── 写入文件: events/2026/04/evt-20260404-103000-xyz.md
    │
    └── EventRepository::upsert() → SQLite
            │
            ├── INSERT events 表
            ├── INSERT events_fts (FTS5)
            ├── INSERT event_entities
            └── INSERT tags

    │
    ▼
brain-indexerd (守护进程)
    │
    ├── 检测到 events/ 目录变化
    ├── process_file() 被调用
    ├── EventParser::parse() 解析文件
    │
    └── EventRepository::upsert() → 再次写入 SQLite
            (幂等操作，结果相同)
```

**关键点：**
- Markdown 文件是真相，SQLite 是索引
- `add` 命令会直接写 SQLite，但守护进程也会检测并更新
- 幂等 upsert 保证最终一致性

## 场景二：AI 自动处理图片

```
用户执行: brain ingest --file photo.jpg --type image --process
    │
    ▼
brain-cli (ingest.rs)
    │
    ├── 验证文件存在
    ├── 创建目标路径: data/raw/image/2026/04/04/
    ├── 复制文件: {timestamp}_CLI_photo.jpg
    │
    ├── 记录日志: Logger::log_ingest_file()
    │
    └── queue::add_task()
            │
            ├── 创建 PipelineTask { task: ImageCaption, input: ... }
            └── 写入 pipeline/queue/pending/{uuid}.yaml

    │
    ▼ (如果指定 --process)
brain-pipeline (主进程)
    │
    ├── 读取 pending/ 目录
    ├── 移动任务文件到 processing/
    │
    ├── 加载数据: 读取 photo.jpg
    │
    ├── 创建 AI 适配器 (默认 Ollama)
    │
    ├── DictSet::load() 加载字典
    │
    ├── adapter.analyze() → AI 分析
    │     │
    │     ├── Stage 1: 自由分析
    │     └── Stage 2: 字典对齐
    │
    ├── EventBuilder::build_from_analysis()
    │     │
    │     ├── 创建 Event 结构体
    │     ├── 设置 type = photo (根据任务类型)
    │     └── 设置 ai.summary 等字段
    │
    ├── EventSerializer::serialize() → Markdown
    │
    ├── 写入 events/2026/04/{id}.md
    │
    ├── Database::open() + EventRepository::upsert()
    │     │
    │     └── 索引到 SQLite
    │
    ├── DictSet 更新 (如有新术语)
    │     │
    │     └── 保存到 dicts/*.yaml
    │
    ├── 移动任务文件到 done/
    │
    └── 记录日志: Logger::log_ai_processing()
```

## 场景三：搜索事件

```
用户执行: brain search "项目会议"
    │
    ▼
brain-cli (search.rs)
    │
    ├── Database::open()
    │
    ├── EventRepository::search("项目会议")
    │     │
    │     └── SELECT * FROM events_fts WHERE events_fts MATCH '项目会议'
    │
    └── 显示结果
            - evt-20260404-103000-xyz
            - 时间: 2026-04-04 10:30:00
            - 类型: meeting
            - 摘要: 项目会议
            - 标签: [project, meeting]
```

## 场景四：查看月度时间线

```
用户执行: brain timeline 2026-03
    │
    ▼
brain-cli (timeline.rs)
    │
    ├── 解析月份: 2026-03
    ├── 计算时间范围:
    │     start: 2026-03-01T00:00:00Z
    │     end: 2026-03-31T23:59:59Z
    │
    ├── EventRepository::find_by_time_range(start, end)
    │     │
    │     └── SELECT * FROM events
    │         WHERE time_start >= start AND time_start <= end
    │         ORDER BY time_start
    │
    └── 按天分组显示
            2026-03-01:
              - 10:00 项目会议
              - 15:00 电话
            2026-03-02:
              ...
```

## 模块依赖关系

```
brain-cli ──────────────► brain-core
                             │
brain-indexerd ──────────┬──┤
                             │
brain-pipeline ──────────┬──┘

brain-core 内部依赖:
    │
    ├── models ──► 无外部依赖
    ├── db ──► models, rusqlite
    ├── markdown ──► models, serde_yaml, gray_matter
    ├── adapters ──► models, ureq (HTTP)
    ├── dicts ──► models, serde_yaml
    ├── logging ──► models, db
    └── config ──► models, directories
```

**依赖方向的重要性：**
- `brain-core` 是基础，不依赖其他三个 crate
- `brain-cli`、`brain-indexerd`、`brain-pipeline` 都依赖 `brain-core`
- 这保证了核心逻辑的可复用性和稳定性
