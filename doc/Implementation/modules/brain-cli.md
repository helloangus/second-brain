# brain-cli 命令行工具

位置：`crates/brain-cli/src/`

## 模块结构

```
brain-cli/src/
├── main.rs           # 主入口，CLI 定义
└── commands/         # 命令实现
    ├── mod.rs
    ├── search.rs     # 全文搜索
    ├── timeline.rs   # 时间线视图
    ├── today.rs      # 今日事件
    ├── add.rs        # 添加事件
    ├── entity.rs     # 实体管理
    ├── stats.rs      # 统计信息
    ├── process.rs    # 处理流水线任务
    ├── logs.rs       # 查看日志
    └── ingest.rs     # 摄入原始数据
```

---

## 命令列表

```
brain search <keyword>              # 全文搜索事件
brain timeline <YYYY-MM>            # 显示月度时间线
brain today                         # 显示今日事件
brain add --type <type> --summary <text> [--tags <tags>]  # 添加事件
brain entity list [--type <type>]   # 列出实体
brain entity show <id>              # 显示实体详情
brain stats                         # 显示统计信息
brain process [--limit <n>]         # 处理 AI 流水线任务
brain logs [--date <date>]          # 查看日志
brain ingest --type <type> --path <path>  # 摄入原始数据
```

---

## 命令详解

### search - 搜索事件

**用法：**
```bash
brain search <keyword>
```

**实现：** `commands/search.rs`

**功能：**
- 使用 SQLite FTS5 全文搜索
- 搜索 `events_fts` 表的 `id`, `ai_summary`, `content` 字段

**输出示例：**
```
Searching for: gpu
==================================================
[2026-03-31 14:30] evt-20260331-001
  Type: meeting
  Summary: 讨论GPU虚拟化技术
  Tags: research, gpu

[2026-03-30 10:15] evt-20260330-002
  Type: work
  Summary: GPU测试
  Tags: testing

Found 2 event(s)
```

---

### timeline - 时间线

**用法：**
```bash
brain timeline <YYYY-MM>
```

**实现：** `commands/timeline.rs`

**功能：**
- 查询指定月份的所有事件
- 按日期分组显示

**输出示例：**
```
Timeline: 2026-03
==================================================

2026-03-31
------------------------------
  14:30 [meeting] 讨论GPU虚拟化
  19:22 [photo] 晚餐照片

2026-03-30
------------------------------
  10:15 [work] GPU测试

Total: 5 event(s)
```

---

### today - 今日事件

**用法：**
```bash
brain today
```

**实现：** `commands/today.rs`

**功能：**
- 查询当前 UTC 日期的所有事件
- 显示时间范围（开始-结束）

**输出示例：**
```
Today's Events: 2026-04-01
==================================================
[09:00 - 10:30] (meeting)
  团队会议讨论项目进度

  Tags: work, team

[14:00] (note)
  学习Rust异步编程

Total: 3 event(s)
```

---

### add - 添加事件

**用法：**
```bash
brain add --type <type> --summary <text> [--tags <tags>]
```

**参数：**
- `--type`: 事件类型（meeting, photo, note, activity, research, reading, exercise, meal, work, other）
- `--summary`: 事件摘要
- `--tags`: 逗号分隔的标签（可选）

**实现：** `commands/add.rs`

**处理流程：**
1. 创建 `Event` 结构体
2. 生成事件 ID (`evt-YYYYMMDD-HHMMSS-xxx`)
3. 序列化为 markdown
4. 写入文件：`events/{YYYY}/{MM}/{id}.md`
5. 插入 SQLite 数据库

**输出示例：**
```
Created event: evt-20260401-143022-a3f
File: events/2026/04/evt-20260401-143022-a3f.md
```

---

### entity list - 列出实体

**用法：**
```bash
brain entity list [--type <type>]
```

**参数：**
- `--type`: 按类型过滤（可选），如 `person`, `place`, `project`

**实现：** `commands/entity.rs`

**功能：**
- 列出所有实体或指定类型的实体
- 按类型分组显示
- 显示实体 ID、标签、状态

**输出示例：**
```
Entities
==================================================

person (3)
------------------------------
  person-angus - Angus
  person-zhangsan - 张三
  person-lisi - 李四

place (2)
------------------------------
  place-home-office - Home Office
  place-company - Company

Total: 8 entity/entities
```

---

### entity show - 显示实体详情

**用法：**
```bash
brain entity show <id>
```

**参数：**
- `id`: 实体 ID

**实现：** `commands/entity.rs`

**显示内容：**
- 基本信息（类型、标签、别名、状态）
- 分类信息（domain、parent）
- 描述和摘要
- 使用指标（事件数、最后出现时间、活动分数）

**输出示例：**
```
Entity: person-angus
==================================================
Type: person
Label: Angus
Status: active
Confidence: 0.95
Domain: technology
Description: Software engineer passionate about AI

Metrics:
  Event count: 42
  Last seen: 2026-04-01 10:30
  Activity score: 0.85
```

---

### stats - 统计信息

**用法：**
```bash
brain stats
```

**实现：** `commands/stats.rs`

**显示内容：**
- 事件总数和类型分布
- 实体总数和类型分布
- Top 10 热门标签

**输出示例：**
```
Second Brain Statistics
==================================================

Events: 156
  By type:
    note: 45
    meeting: 32
    work: 28
    photo: 24
    activity: 15
    research: 8
    other: 4

Entities: 89
  By type:
    person: 15
    place: 12
    topic: 24
    project: 8
    concept: 18
    skill: 7
    other: 5

Top tags:
  work: 38
  research: 29
  learning: 24
  meeting: 21
  daily: 18
```

---

### process - 处理 AI 流水线任务

**用法：**
```bash
brain process [--limit <n>]
```

**参数：**
- `--limit`: 限制处理任务数量（可选）

**实现：** `commands/process.rs`

**功能：**
- 处理 pipelines/queue/pending 中的任务
- 调用 AI 模型分析
- 生成事件并写入文件系统

---

### logs - 查看日志

**用法：**
```bash
brain logs [--date <date>]
```

**参数：**
- `--date`: 指定日期（可选），默认为今天

**实现：** `commands/logs.rs`

---

### ingest - 摄入原始数据

**用法：**
```bash
brain ingest --type <type> --path <path> [--source <source>]
```

**参数：**
- `--type`: 数据类型（image, audio, video, text）
- `--path`: 文件路径
- `--source`: 来源（可选）

**实现：** `commands/ingest.rs`

**功能：**
- 将原始数据文件添加到 AI 处理队列
- 创建 pipeline 任务文件

---

## CLI 定义

`main.rs`

使用 `clap` crate 进行参数解析：

```rust
#[derive(Parser)]
#[command(name = "brain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Search { keyword: String },
    Timeline { month: String },
    Today,
    Add {
        #[arg(long)]
        type_: String,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        tags: Option<String>,
    },
    Entity {
        #[command(subcommand)]
        command: EntityCommands,
    },
    Process {
        #[arg(long)]
        limit: Option<usize>,
    },
    Logs {
        #[arg(long)]
        date: Option<String>,
    },
    Ingest {
        #[arg(long)]
        type_: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        source: Option<String>,
    },
    Stats,
}
```

---

## 配置加载

所有命令都通过 `BrainConfig::load()` 加载配置：

```rust
let config = BrainConfig::load()?;
let db = Database::open(&config.db_path)?;
let conn = db.connection();
```
