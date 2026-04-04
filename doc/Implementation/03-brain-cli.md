# brain-cli：命令行工具

## 概述

`brain-cli` 是用户与系统交互的主要入口，提供一系列命令来查询、添加和管理事件。

## 命令列表

| 命令 | 功能 | 核心操作 |
|------|------|----------|
| `search` | 关键词搜索事件 | SQLite FTS 全文搜索 |
| `timeline` | 按月查看事件时间线 | 时间范围查询 |
| `today` | 查看今天的事件 | 当日时间范围查询 |
| `add` | 手动添加事件 | 写入 Markdown + 索引 |
| `ingest` | 摄入文件到 AI 处理队列 | 复制文件 + 添加 Task |
| `process` | 运行 AI 处理流水线 | 启动 brain-pipeline |
| `logs` | 查看系统日志 | 查询日志数据库 |
| `entity` | 实体管理 | 列出/查看实体 |
| `stats` | 统计信息 | 聚合查询 |

## 命令详解

### search — 全文搜索

```bash
brain search "关键词"
```

**工作流程：**
1. 打开 SQLite 数据库
2. 在 `events_fts` 表执行 FTS5 全文搜索
3. 返回匹配的事件（ID、时间、类型、摘要、标签）

### timeline — 月度时间线

```bash
brain timeline 2026-03
```

**工作流程：**
1. 解析月份参数
2. 计算该月的开始和结束时间戳
3. 查询该范围内的所有事件
4. 按天分组显示

### today — 今日事件

```bash
brain today
```

**工作流程：**
1. 获取当前 UTC 时间
2. 计算今天的开始（00:00:00）和结束（23:59:59）
3. 查询范围内事件并显示

### add — 手动添加事件

```bash
brain add --type meeting --summary "项目讨论会议"
brain add --type photo --summary "旅行照片" --tags travel,family
```

**支持的事件类型：**
- meeting, photo, note, activity, research, reading, exercise, meal, work, other

**工作流程：**
1. 生成事件 ID（`evt-YYYYMMDD-HHMMSS-xxx`）
2. 创建 Event 结构体
3. **写入 Markdown 文件**：`events/{year}/{month}/{id}.md`
4. **更新 SQLite**：EventRepository::upsert()
5. 输出创建的事件 ID 和文件路径

**双写保证：**
- Markdown 文件是真相（可被 Obsidian 打开）
- SQLite 索引保证快速查询

### ingest — 文件摄入

```bash
brain ingest --file /path/to/photo.jpg --type image
brain ingest --file /path/to/audio.m4a --type audio --process
```

**数据类型：** image, audio, video, text, document

**工作流程：**
1. 验证源文件存在
2. 创建目标路径：`data/raw/{type}/{year}/{month}/{day}/`
3. 复制文件到数据湖
4. 记录摄入日志
5. 添加处理任务到 `pipeline/queue/pending/`
6. 如果指定 `--process`：立即启动 AI 处理

### process — 运行 AI 流水线

```bash
brain process
brain process --limit 10
```

**工作流程：**
1. 查找 `brain-pipeline` 二进制文件
2. 启动子进程执行 `brain-pipeline process`
3. 等待完成并检查退出状态

### logs — 日志查看

```bash
brain logs
brain logs --log-type ai_processing
brain logs --target-type event --target-id evt-20260331-001
brain logs --stats --days 7
```

**日志类型过滤：** crud, ai_processing, pipeline, system, tag, cognition, evaluation

**目标类型过滤：** event, entity, tag, pipeline_task, config, system

**`--stats` 显示 AI 处理统计：**
- 总操作数
- 成功/失败次数
- 平均耗时
- 成功率

### entity — 实体管理

```bash
# 列出所有实体
brain entity list

# 按类型筛选
brain entity list --type person

# 查看实体详情
brain entity show person-john
```

**实体类型：** person, organization, project, artifact, concept, topic, activity, goal, skill, place, device, resource, memory_cluster, state

### stats — 统计信息

```bash
brain stats
```

**显示内容：**
- 总事件数
- 事件类型分布
- 总实体数
- 实体类型分布
- Top 10 标签

## 架构

```
brain-cli (main.rs)
├── commands/
│   ├── search.rs      # 搜索命令
│   ├── timeline.rs    # 时间线命令
│   ├── today.rs       # 今日命令
│   ├── add.rs         # 添加命令
│   ├── ingest.rs      # 摄入命令
│   ├── process.rs     # 处理命令
│   ├── logs.rs        # 日志命令
│   ├── entity.rs      # 实体命令 (list, show)
│   └── stats.rs       # 统计命令
└── main.rs            # 命令解析入口
```

**依赖 brain-core 的部分：**
- `BrainConfig` — 加载配置和路径
- `Database` — 数据库连接
- `EventRepository` — 事件查询和写入
- `EntityRepository` — 实体查询
- `EventSerializer` — Markdown 生成
- `Logger` — 日志记录
- `DictSet` — 字典查询（ingest 时使用）

## 与其他模块的交互

```
brain-cli
    │
    ├───► brain-core (直接调用)
    │     • BrainConfig::load()
    │     • Database::open()
    │     • EventRepository / EntityRepository
    │     • EventSerializer
    │     • Logger
    │
    └───► brain-pipeline (进程间调用)
          • brain-pipeline binary (通过 subprocess)
          • queue/pending/ 目录文件操作
```
