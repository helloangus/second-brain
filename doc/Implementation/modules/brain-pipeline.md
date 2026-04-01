# brain-pipeline AI 处理流水线

位置：`crates/brain-pipeline/src/`

## 模块结构

```
brain-pipeline/src/
├── lib.rs           # 库接口
├── main.rs          # 主入口，CLI 定义
├── builder.rs       # Event 构建器
├── processor.rs     # 任务处理器
└── queue.rs         # 队列管理
```

---

## 功能概述

**职责：**
- 管理 AI 处理任务队列
- 调用 AI 模型分析原始数据
- 生成事件并写入文件系统

**核心原则：**
> **AI 永不直接写 markdown**
>
> 所有 AI 输出必须经过 `EventBuilder` 验证和转换

---

## 命令行接口

`main.rs`

使用 `clap` crate 进行参数解析：

```rust
#[derive(Parser)]
#[command(name = "brain-pipeline")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process pending tasks
    Process {
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Show queue status
    Status,
}
```

### 命令示例

```bash
# 处理所有待处理任务
./brain-pipeline process

# 只处理 3 个任务
./brain-pipeline process --limit 3

# 查看队列状态
./brain-pipeline status
```

---

## 队列结构

```
pipelines/queue/
├── pending/         # 待处理任务
├── processing/      # 处理中任务
└── done/           # 已完成任务
```

**任务文件格式：** YAML

```yaml
id: "a1b2c3d4"
task: image_caption
input:
  path: "/path/to/image.jpg"
  source: "phone_camera"
  metadata: {}
output: null
status: pending
```

---

## 核心组件

### EventBuilder
`builder.rs`

**关键原则：** 验证和转换 AI 输出，不允许 AI 直接写文件

```rust
pub struct EventBuilder;

impl EventBuilder {
    pub fn build_from_analysis(
        input_path: &str,
        task_type: &TaskType,
        output: &PipelineOutput,
        source: &Option<String>,
    ) -> Result<Event, Box<dyn std::error::Error>>
}
```

**处理逻辑：**

1. **ID 生成：** `Event::generate_id()`
2. **类型映射：** TaskType → EventType

| TaskType | EventType |
|----------|-----------|
| ImageCaption, FaceDetection, Ocr | Photo |
| Asr, SpeakerDiarization | Activity |
| Embedding, Reasoning, Summarize, Tagging | Research |
| 其他 | Other |

3. **摘要生成：** 使用 AI 输出或默认描述
4. **标签处理：** 直接使用 AI 返回的 tags
5. **实体处理：** AI 输出的 entities 作为 topics

**禁止的行为：**
- AI 直接写入文件系统
- AI 生成不符合 schema 的 markdown
- AI 访问敏感配置

---

### Queue
`queue.rs`

**show_status 函数：**

```bash
$ ./brain-pipeline status
Pipeline Queue Status
==================================================
Pending:    5
Processing: 0
Done:       42
```

---

### Processor
`processor.rs`

**process_queue 函数：**

```rust
pub async fn process_queue(
    config: &BrainConfig,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>>
```

**处理流程：**

```
1. 读取 pending/ 目录中的任务文件
2. 按文件名排序（ oldest first）
3. 应用 limit 限制处理数量
4. 获取第一个配置的 AI 适配器
5. 遍历任务：
   a. 移动到 processing/
   b. 调用适配器分析
   c. EventBuilder 构建事件
   d. 写入 markdown 文件
   e. 插入数据库
   f. 移动到 done/
```

**process_task 函数：**

```rust
async fn process_task(
    task: &PipelineTask,
    adapter: &dyn ModelAdapter,
    config: &BrainConfig,
) -> Result<(), Box<dyn std::error::Error>>
```

**步骤：**

1. **创建适配器输入：**

```rust
let input = RawDataInput {
    data_type: task.data_type(),
    path: task.input.path.clone(),
    metadata: task.input.metadata.clone(),
};
```

2. **AI 分析：**

```rust
if adapter.supports(&task.data_type()) {
    let analysis = adapter.analyze(&input)?;
    // 转换输出
} else {
    // 使用默认输出
}
```

3. **构建事件：**

```rust
let event = EventBuilder::build_from_analysis(
    &task.input.path,
    &task.task,
    &output,
    &task.input.source,
)?;
```

4. **保存：**

```rust
// 写入文件
let event_path = config.events_path
    .join(&year)
    .join(&month)
    .join(format!("{}.md", event.id));
fs::write(&event_path, markdown)?;

// 写入数据库
let repo = EventRepository::new(&conn);
repo.upsert(&event)?;
```

---

## 适配器配置

**优先级：**
1. 使用配置文件中定义的第一个适配器
2. 默认使用 Ollama（localhost:11434, llama3）

```rust
let adapter_config = config
    .adapters
    .first()
    .cloned()
    .unwrap_or_else(|| AdapterConfig::ollama("http://localhost:11434", "llama3"));

let adapter = create_adapter(&adapter_config)?;
```

---

## 错误处理

| 场景 | 处理方式 |
|------|---------|
| 任务文件解析失败 | 跳过，记录 error |
| AI 适配器不支持该类型 | 使用默认输出继续 |
| AI 分析失败 | 移动到 processing/，保留重试 |
| 写入文件失败 | 记录 error，保持 processing 状态 |
| 数据库写入失败 | 记录 error，保持 processing 状态 |

---

## 数据流图

```
原始数据文件
    ↓
pipeline add --task xxx --input <file>    (在 brain-cli 中)
    ↓
任务.yaml 写入 pending/
    ↓
brain-pipeline process
    ↓
任务.yaml 移动到 processing/
    ↓
ModelAdapter.analyze()
    ↓
返回 AnalysisOutput { summary, tags, entities, confidence }
    ↓
EventBuilder.build_from_analysis()
    ↓
Event { id, type, time, ai_summary, tags, ... }
    ↓
┌───────────────────────────────────────┐
│ EventSerializer.serialize()           │
│     ↓                                  │
│ markdown文件 → events/{YYYY}/{MM}/     │
│     ↓                                  │
│ EventRepository.upsert() → SQLite     │
└───────────────────────────────────────┘
    ↓
任务.yaml 移动到 done/
```

---

## 限制与注意事项

1. **单适配器：** 只使用配置的第一个适配器
2. **无自动重试：** processing 中的任务需要手动处理
3. **Ollama 依赖：** 默认使用 Ollama，服务不可用时任务会失败
4. **文件路径：** AI 不知道绝对路径，输出使用相对路径
5. **事件-实体分离：** AI 输出的 entities 作为 topics，不是独立实体
