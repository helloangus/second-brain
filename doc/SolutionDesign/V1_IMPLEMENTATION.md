# V1 Architecture - 完整实现方案

> 基于 V1 Architecture 的实现规划
> 扩展性设计：V2-V6 可逐层接入
> 当前版本: V1.0

---

## 一、系统目标

**V1 核心目标**：建立可工作的记忆存储和检索系统

```
数据采集 → Event存储 → AI分析 → 索引 → CLI查询
```

**扩展预留**：V2-V6 可在V1基础上逐步接入

---

## 二、核心约束（严格遵守）

| 约束 | 实现 |
|------|------|
| Local-first | 数据本地存储 |
| Git化 | events/, entities/, config/ 纳入Git |
| Markdown真相 | YAML frontmatter |
| Linux server风格 | CLI优先，systemd |
| 自动化优先 | 夜间批处理 |
| AI可替换 | Adapter层 |
| 扩展性 | 接口预留 |

---

## 三、目录结构

```
second-brain/
├── data/
│   └── raw/
│       ├── image/
│       ├── audio/
│       ├── video/
│       └── text/
│
├── events/
│   └── {year}/
│       └── {month}/
│           └── evt-*.md
│
├── entities/
│   ├── people/
│   ├── places/
│   ├── topics/
│   └── projects/
│
├── index/
│   ├── events.db
│   └── events.db-wal
│
├── pipelines/
│   ├── queue/
│   │   ├── pending/
│   │   ├── processing/
│   │   └── done/
│   └── scripts/
│
├── scripts/
│   ├── brain-indexerd
│   └── brain-pipeline
│
├── adapters/           # V1: 预留给V3/V6
│   └──
├── config/
│   ├── schema.yaml
│   └── brain.yaml
│
└── golden_events/     # V4: 预留
```

---

## 四、核心数据模型

### 4.1 Event Markdown

（使用用户提供的schema: event/v1）

```markdown
---
schema: event/v1

id: evt-20260331-194522-a3f

type: meeting
subtype: research_discussion

time:
  start: 2026-03-31T19:45:22+09:00
  end: 2026-03-31T21:10:00+09:00
  timezone: Asia/Tokyo

created_at: 2026-03-31T21:30:01Z
ingested_at: 2026-03-31T21:31:10Z

source:
  device: phone
  channel: manual_note
  capture_agent: mobile_share

status: auto
confidence: 0.87

---

entities:
  people:
    - person-angus
    - person-zhangsan

  projects:
    - proj-revmm

  concepts:
    - concept-gpu-virtualization

  places:
    - place-home-office

tags:
  - research
  - gpu
  - discussion

---

raw_refs:
  - ../../data/raw/audio/2026/03/31/meeting.m4a
  - ../../data/raw/text/note.txt

derived_refs:
  transcript: ../../derived/transcripts/evt-xxx.md
  embedding: ../../derived/embeddings/evt-xxx.vec

---

ai:
  summary: >
    讨论 ReVMM 中 GPU 虚拟化中断路径设计，
    确认采用 mdev-based routing。

  topics:
    - gpu virtualization
    - interrupt routing

  sentiment: neutral

  extraction_version: 3

---

relations:
  inferred_from:
    - evt-20260328-091200-b2d

---

graph_hints:
  importance: 0.72
  recurrence: false

schema_version: 1
---

```

### 4.2 Entity Markdown

（使用用户提供的schema: entity/v1）

```markdown
---
schema: entity/v1

id: concept-gpu-virtualization
type: concept

label: GPU Virtualization
aliases:
  - gpu virt
  - gpu-virtualisation
  - GPU虚拟化

created_at: 2025-05-08
updated_at: 2026-03-31

status: active
confidence: 0.93

---

classification:
  domain: virtualization
  parent:
    - concept-virtualization

---

identity:
  description: >
    在虚拟化环境中对 GPU 资源进行共享、
    隔离与调度的技术集合。

---

multimodal:
  images:
    - ../../media/entities/gpu-diagram.png

  voices: []
  embeddings:
    text: ../../derived/entity_embeddings/concept-gpu-virt.vec

---

links:
  wikipedia: https://en.wikipedia.org/wiki/GPU_virtualization
  papers:
    - doi:xxxxx

---

evolution:
  merged_from:
    - concept-gpu-virt
    - concept-gpu-sharing

---

metrics:
  event_count: 128
  last_seen: 2026-03-31
  activity_score: 0.81

schema_version: 1
---

```

### 4.3 SQLite Schema

```sql
-- Event表：匹配 schema: event/v1
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    schema_version INTEGER DEFAULT 1,
    
    -- 时间信息
    time_start INTEGER NOT NULL,    -- Unix timestamp
    time_end INTEGER,
    timezone TEXT DEFAULT 'UTC',
    
    -- 类型
    type TEXT NOT NULL,
    subtype TEXT,
    
    -- 来源
    source_device TEXT,
    source_channel TEXT,
    source_capture_agent TEXT,
    
    -- 状态
    status TEXT DEFAULT 'auto',
    confidence REAL DEFAULT 0.5,
    
    -- AI分析
    ai_summary TEXT,
    ai_topics TEXT,              -- JSON array
    ai_sentiment TEXT,
    extraction_version INTEGER,
    
    -- 图谱 hints
    importance REAL,
    recurrence INTEGER,
    
    -- 系统
    created_at INTEGER,
    ingested_at INTEGER,
    updated_at INTEGER
);

-- Entity表：匹配 schema: entity/v1
CREATE TABLE entities (
    id TEXT PRIMARY KEY,
    schema_version INTEGER DEFAULT 1,
    
    -- 基本信息
    type TEXT NOT NULL,
    label TEXT NOT NULL,
    aliases TEXT,               -- JSON array
    
    -- 状态
    status TEXT DEFAULT 'active',
    confidence REAL DEFAULT 0.5,
    
    -- 分类
    classification_domain TEXT,
    classification_parent TEXT,
    
    -- 描述
    identity_description TEXT,
    
    -- 链接
    links TEXT,                  -- JSON: {wikipedia, papers}
    
    -- 进化
    merged_from TEXT,             -- JSON array
    
    -- 指标
    event_count INTEGER DEFAULT 0,
    last_seen INTEGER,
    activity_score REAL,
    
    -- 系统
    created_at INTEGER,
    updated_at INTEGER
);

-- Event-Entity关联：匹配 entities字段
CREATE TABLE event_entities (
    event_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT,           -- people|projects|concepts|places
    relation TEXT,
    PRIMARY KEY (event_id, entity_id, relation)
);

-- Tags表
CREATE TABLE tags (
    event_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    PRIMARY KEY (event_id, tag)
);

-- Relations表：记录事件间关系
CREATE TABLE event_relations (
    event_id TEXT NOT NULL,
    rel_type TEXT NOT NULL,
    target_event_id TEXT NOT NULL,
    PRIMARY KEY (event_id, rel_type, target_event_id)
);

-- 全文搜索
CREATE VIRTUAL TABLE events_fts USING fts5(
    id,
    ai_summary,
    content
);

-- 索引
CREATE INDEX idx_events_time_start ON events(time_start);
CREATE INDEX idx_events_type ON events(type);
CREATE INDEX idx_entities_type ON entities(type);
CREATE INDEX idx_entities_label ON entities(label);
```

CREATE VIRTUAL TABLE events_fts USING fts5(
    id,
    summary,
    content
);

-- 索引优化
CREATE INDEX idx_events_time ON events(time);
CREATE INDEX idx_events_type ON events(type);
CREATE INDEX idx_entities_type ON entities(type);
```

---

## 五、实现阶段

### Phase 1: 基础设施（Day 1-2）

#### Step 1.1 创建目录结构

```bash
mkdir -p second-brain/{data/raw/{image,audio,video,text},events/{2026},entities/{people,places,topics,projects},index,pipelines/queue/{pending,processing,done},scripts,adapters,config,golden_events}
```

#### Step 1.2 初始化Git

```bash
cd second-brain
git init
# .gitignore 排除 data/raw/
```

#### Step 1.3 创建schema.yaml

```yaml
version: 1
event:
  required_fields:
    - id
    - time
    - type
  optional_fields:
    - source
    - entities
    - raw_refs
    - ai
    - status
entity_types:
  - person
  - organization
  - project
  - artifact
  - concept
  - topic
  - activity
  - goal
  - skill
  - place
  - device
  - resource
  - memory_cluster
  - state

```

---

### Phase 2: 手动Event创建（Day 2-3）

创建10个示例Event markdown：

```
events/2026/03/
├── evt-20260328-001.md    # 早餐
├── evt-20260328-002.md    # 工作会议
├── evt-20260329-001.md    # 学习笔记
├── evt-20260329-002.md    # 阅读
├── evt-20260330-001.md    # 代码实验
├── evt-20260330-002.md    # 健身
├── evt-20260331-001.md    # 晚餐
...
```

验证Obsidian打开效果。

---

### Phase 3: Indexer Daemon（Day 3-5）

#### 3.1 核心功能

```rust
// brain-indexerd
// 职责：监听events/变化 → 更新SQLite

loop {
    for event in watcher.poll() {
        let md = parse_frontmatter(event)?;
        db.upsert_event(&md)?;
        db.update_fts(&md.summary)?;
    }
}
```

#### 3.2 技术栈

| 功能 | 库 |
|------|-----|
| 文件监听 | notify |
| YAML解析 | serde_yaml |
| SQLite | rusqlite |
| FTS | sqlite内置 |

#### 3.3 部署

```bash
# systemd service
[Unit]
Description=Brain Indexer Daemon

[Service]
ExecStart=/usr/local/bin/brain-indexerd
WorkingDirectory=/home/user/second-brain

[Install]
WantedBy=multi-user.target
```

---

### Phase 4: CLI工具（Day 5-7）

#### 4.1 核心命令

```bash
# 搜索
brain search "关键词"

# 时间线
brain timeline 2026-03

# 今日
brain today

# 添加（手动）
brain add --type photo --summary "描述" [--tag x,y]

# 实体查询
brain entity list
brain entity show <id>
```

#### 4.2 CLI架构

```
brain
├── main.rs
├── commands/
│   ├── search.rs
│   ├── timeline.rs
│   ├── add.rs
│   └── entity.rs
├── db/
│   └── mod.rs
└── config.rs
```

---

### Phase 5: AI Pipeline（Day 7-10）

#### 5.1 架构设计

```
raw data
   ↓
ingest queue (pending/)
   ↓
AI分析 (夜间cron)
   ↓
JSON输出 → Event Builder → Markdown
   ↓
Indexer更新
```

#### 5.2 模型Adapter（预留扩展）

```rust
trait ModelAdapter {
    fn analyze(&self, input: &Input) -> Result<Analysis>;
}

struct OllamaAdapter;
struct OpenAIAdapter;
```

#### 5.3 Event Builder

**约束**：AI绝不直接写MD

```rust
// AI输出
{
  "event": {
    "time": "...",
    "type": "photo",
    "summary": "晚餐",
    "tags": ["food"],
    "confidence": 0.85
  }
}

// Builder生成MD
builder.create(event_json) → evt-*.md
```

#### 5.4 夜间任务

```bash
# crontab
0 2 * * * brain-pipeline process
```

---

### Phase 6: 扩展预留接口（设计时预留）

#### 6.1 Capture接口（V2预留）

```bash
# Gateway未来会调用
POST /internal/ingest
  input: file
  source: mobile|pc|browser
```

#### 6.2 Graph接口（V3预留）

```sql
-- 预留edges表
CREATE TABLE edges (
    src TEXT,
    dst TEXT,
    relation TEXT,
    weight REAL,
    event_id TEXT
);
```

#### 6.3 Evolution接口（V4预留）

```bash
# 预留目录
/golden_events/        # Golden Memory Set
/evolution/proposals/  # 进化提案
```

#### 6.4 Proactive接口（V5预留）

```bash
# 预留输出
/logs/daily/
/insights/
```

---

## 六、工程进度

| Day | 任务 | 交付 |
|-----|------|------|
| 1 | 目录结构+Git | 可用目录 |
| 2 | Schema+config | schema.yaml |
| 2-3 | 10个Event示例 | MD文件 |
| 3-5 | Indexer Daemon | 可运行服务 |
| 5-7 | CLI工具 | brain命令 |
| 7-10 | AI Pipeline | 夜间任务 |

---

## 七、验收标准

- [ ] 目录结构就绪
- [ ] Git管理events/
- [ ] 10+ Event示例
- [ ] Indexer实时索引
- [ ] `brain search`工作
- [ ] `brain timeline`视图
- [ ] AI Pipeline夜间运行
- [ ] Obsidian可打开

---

## 八、关键决策

| 决策 | 理由 |
|------|------|
| SQLite不用Neo4j | 运维简单，Git兼容 |
| Markdown真相 | Obsidian兼容 |
| AI不直接写MD | 防格式漂移 |
| CLI优先 | Linux风格 |
| 夜间批处理 | 低功耗 |

---

## 九、下一步

**今天**：
1. 创建目录结构
2. 初始化Git
3. 创建schema.yaml

**明天**：
4. 手动创建10个Event

**后天**：
5. 实现最小Indexer