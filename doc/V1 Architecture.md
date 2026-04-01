# V1 Architecture

严格符合你已经确定的约束：

✅ Local-first &#x20;

✅ Git 化 &#x20;

✅ Markdown 为真相 &#x20;

✅ Linux server 风格 &#x20;

✅ 自动化优先 &#x20;

✅ AI 可替换 &#x20;

✅ 可扩展到几十万 events &#x20;

***

# 总体目标（重新定义一句话）

> 构建一个**Event-driven Personal Cognitive System**
> —— 所有生活数据 → Event → 可计算记忆。

***

# 一、系统整体架构（V1）

这是你系统未来 80% 时间都会保持不变的骨架：

```markdown 
                 ┌──────────────────┐
                 │ Capture Layer     │
                 │ (手机/PC/导入)    │
                 └────────┬─────────┘
                          ↓
                 ┌──────────────────┐
                 │ Raw Data Lake     │
                 │ immutable files   │
                 └────────┬─────────┘
                          ↓
                 ┌──────────────────┐
                 │ AI Pipeline       │
                 │ (batch analyze)   │
                 └────────┬─────────┘
                          ↓
                 ┌──────────────────┐
                 │ Event Store       │ ⭐ Markdown + Git
                 └────────┬─────────┘
                          ↓
                 ┌──────────────────┐
                 │ Indexer Daemon    │ ⭐核心
                 └────────┬─────────┘
                          ↓
                 ┌──────────────────┐
                 │ SQLite Index      │
                 └────────┬─────────┘
                          ↓
        ┌─────────────────┼─────────────────┐
        ↓                 ↓                 ↓
    Obsidian        CLI Search        Future UI
```


***

# 二、目录结构（直接可用）

建议你**从第一天就固定**：

```markdown 
second-brain/
│
├── data/
│   └── raw/
│       ├── image/
│       ├── audio/
│       ├── video/
│       ├── text/
│       └── web/
│
├── events/
│   └── 2026/
│
├── entities/
│   ├── people/
│   ├── places/
│   ├── topics/
│   └── projects/
│
├── index/
│   └── events.db
│
├── pipelines/
│
├── scripts/
│
└── config/
```


***

# 三、核心数据模型（最重要部分）

## 1️⃣ Event = 系统唯一核心对象

每个 event 一个 markdown。

### 标准格式

```markdown 
---
id: evt-20260331-001
time: 2026-03-31T19:22:11
type: photo

source: phone_camera

entities:
  people: []
  places: [home]
  topics: [daily_life]

raw_refs:
  - ../../data/raw/image/2026-03-31T19-22-11.jpg

ai:
  summary: 晚餐照片
  tags: [food, dinner]
  confidence: 0.82

status: auto
---

## Summary
AI生成摘要

## Notes
（人工补充）
```


***

## 为什么必须这样？

因为 YAML frontmatter：

- 易解析 &#x20;
- Obsidian 支持 &#x20;
- CLI 可处理 &#x20;
- Git diff 清晰 &#x20;

***

# 四、Entity 设计（这是第二大脑真正“活”的原因）

你已经决定：

> 人物、任务、长期事件必须长期存在。

所以 entity 也必须 markdown 化。

***

## 示例：人物实体

`entities/people/zhangsan.md`

```markdown 
---
id: person-001
type: person
aliases: [张三]
---

# 张三

## Related Events
（自动生成）
```


***

### Event 中只引用 ID：

```markdown 
entities:
  people: [person-001]
```


这一步极其关键：

👉 你正在建立**个人知识图谱**。

***

# 五、Index Layer（解决性能问题）

现在进入你最关心的部分。

***

## SQLite schema

```sql 
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    time INTEGER,
    type TEXT,
    summary TEXT,
    path TEXT
);

CREATE TABLE event_entities (
    event_id TEXT,
    entity_id TEXT
);

CREATE TABLE tags (
    event_id TEXT,
    tag TEXT
);

CREATE VIRTUAL TABLE events_fts USING fts5(
    id,
    content
);
```


***

## 为什么这样设计？

查询：

> “找张三相关的晚上事件”

变成：

```sql 
SELECT e.path
FROM events e
JOIN event_entities ee
ON e.id = ee.event_id
WHERE ee.entity_id='person-001'
AND e.time > ...
```


毫秒级。

***

# 六、Indexer Daemon（系统灵魂）

你必须有一个后台程序：

`brain-indexerd`

职责：

```text 
监听 events/ 变化
      ↓
解析 YAML
      ↓
更新 SQLite
```


***

## 工作流程

```markdown 
git pull / 新文件
        ↓
filesystem watcher
        ↓
parse frontmatter
        ↓
update db
```


***

## 推荐实现（符合你的背景）

👉 强烈建议 Rust 实现。

原因：

- sqlite-rs &#x20;
- 高性能 &#x20;
- 长期运行稳定 &#x20;

推荐库：

| 功能       | Rust crate    |
| -------- | ------------- |
| 文件监听     | notify        |
| markdown | gray-matter   |
| yaml     | serde\\\_yaml |
| sqlite   | rusqlite      |
| fts      | sqlite 内置     |

***

## Indexer 伪代码

```rust 
loop {
    watch(events_dir);

    for changed_file {
        let event = parse_markdown(file);

        db.upsert(event);
        db.update_entities(event);
        db.update_fts(event);
    }
}
```


***

# 七、AI Pipeline（延迟批处理）

符合你低功耗机器需求。

***

## Pipeline 设计

```markdown 
raw data
   ↓
ingest queue
   ↓
light analysis
   ↓
heavy analysis (夜间)
   ↓
event generation
```


## 队列设计

直接文件队列：

```markdown 
/pipelines/queue/
    pending/
    processing/
    done/
```


一个任务：

```yaml 
task: image.analyze
input:
  path: xxx.jpg
output:
  summary
  tags
  entities
```


## Event 生成协议

AI 不能随便生成 event。

必须遵守协议。

***

## Event Creation Contract

AI 输出必须是：

```json 
{
  "event": {
    "time": "...",
    "type": "...",
    "summary": "...",
    "entities": {},
    "tags": [],
    "confidence": 0.0
  }
}
```


然后：

```markdown 
AI NEVER writes markdown directly.
```


⚠️ AI 不允许直接写文件。

***

## 为什么？

否则会出现：

- 格式漂移 &#x20;
- YAML 损坏 &#x20;
- Git chaos &#x20;
- schema 崩坏 &#x20;

***

## 正确流程

```markdown 
AI → JSON
      ↓
Event Builder（你控制）
      ↓
Markdown Generator
```


只有 builder 能写 md。

***

## Event Builder

职责：

```markdown 
1. 校验 AI 输出
2. 补充 metadata
3. 生成 ID
4. 写 markdown
5. git add
```


***

### ID 生成规则

```text 
evt-YYYYMMDD-HHMMSS-rand
```


例：

```text 
evt-20260331-194522-a3f
```


优点：

- 时间排序 &#x20;
- 全球唯一 &#x20;
- 无数据库依赖

***

# 八、AI 抽象层

你已经提出非常高级的要求：

> 模型必须可替换。

必须引入：

## Model Adapter Layer

```markdown 
AI Task
   ↓
Adapter Interface
   ↓
具体模型
```


***

### 统一接口（概念）

```yaml 
task: image_tagging
input: image_path
output:
  tags: []
  summary: ""
```


***

### Adapter 示例

```markdown 
adapters/
   ollama_adapter
   vllm_adapter
   openai_adapter
```


Pipeline 永远不知道模型是谁。

***

# 九、查询模型（真正的“第二大脑体验”）

查询不应该是：

keyword search

而是：

***

## Query → Event Graph Traversal

例如：

> “上次和张三讨论虚拟化是什么时候？”

执行：

```sql 
entity = 张三
AND topic = virtualization
ORDER BY time DESC
LIMIT 1
```


得到 event → 打开 markdown。

***

# 十、Git 化策略（非常重要）

不要全仓库 git。

正确方式：

## 正确方式

```markdown 
Git 管理：
✅ events/
✅ entities/
✅ config/

不管理：
❌ data/raw/
```


raw 用：

```markdown 
git lfs
# 或
rsync backup
```


***

# 十一、CLI 优先（符合 Linux Server 风格）

未来核心命令应该像：

```markdown 
brain add image.jpg
brain search "张三"
brain timeline 2026-03
brain today
```


UI 是后话。

***

# 十一、系统运行模式（未来真实使用方式）

你每天实际体验将是：

***

### 白天

手机产生数据：

- 照片 &#x20;
- 录音 &#x20;
- 浏览 &#x20;

⬇ 自动同步 home server

***

### 夜间

系统自动：

```markdown 
分析数据
生成 events
更新 index
建立关联
```


***

### 第二天

你打开：

```bash 
brain search 张三 虚拟化
```


立刻得到：

```markdown 
2026-03-12 深夜讨论GPU虚拟化
2026-02-21 实验设计讨论
```


这时你已经拥有：

> **外置长期记忆系统**

***

# 十、你现在可以立刻开始的 Phase 1（两周目标）

只做这 5 件事：

***

## ✅ Step 1

创建目录结构。

***

## ✅ Step 2

手动写 10 个 event markdown。

不要写代码。

感受模型。

***

## ✅ Step 3

写最小 indexer：

`parse md → sqlite`

仅支持：

- id &#x20;
- time &#x20;
- summary &#x20;

***

## ✅ Step 4

实现：

`brain search keyword`

CLI。

***

## ✅ Step 5

Obsidian 打开 events 目录。

你会第一次看到“第二大脑”。
