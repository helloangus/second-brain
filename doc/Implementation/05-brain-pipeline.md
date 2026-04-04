# brain-pipeline：AI 处理流水线

## 概述

`brain-pipeline` 是系统的 AI 处理引擎，负责将原始数据（图片、音频、文本等）转化为结构化的事件。它从队列中取出任务，调用 AI 模型分析，然后将结果写入系统。

## 核心概念：队列架构

任务通过文件系统队列管理，而不是内存队列或消息中间件：

```
pipeline/queue/
├── pending/     # 等待处理的任务
├── processing/  # 正在处理的任务
└── done/        # 已完成的任务
```

**任务流转：**
```
pending/  ──(开始处理)──►  processing/  ──(成功)──►  done/
                                     └──(失败)──►  (留在 processing/)
```

**为何用文件队列？**
- **持久化**：进程重启后任务不丢失
- **简单**：无需 Redis 或其他消息中间件
- **可检查**：可随时查看队列状态

## PipelineTask 结构

每个任务是一个 YAML 文件：

```yaml
id: abc12345                    # 短 UUID
task: ImageCaption              # 任务类型
status: Pending                  # 状态
input:
  path: ../../data/raw/image/photo.jpg
  channel: CLI
  device: PC
  capture_agent: manual_entry
  data_type: Image
  metadata: {}
output:                         # 处理完成后填充
  summary: "照片拍摄于东京..."
  type: photo
  tags: ["travel", "japan"]
  ...
```

**TaskType 任务类型：**
| TaskType | 适用数据 | 说明 |
|----------|---------|------|
| ImageCaption | 图片 | 生成图片描述 |
| FaceDetection | 图片 | 人脸检测 |
| Ocr | 图片 | 文字识别 |
| Asr | 音频 | 语音转文字 |
| SpeakerDiarization | 音频 | 说话人分离 |
| Embedding | 文本 | 生成嵌入向量 |
| Reasoning | 文本 | 推理分析 |
| Routing | 文本 | 任务路由 |
| Summarize | 文本 | 文本摘要 |
| Tagging | 文本 | 标签生成 |

## 处理流程

```
1. 收集任务
   │
   ▼
   读取 pending/ 目录
   按文件名排序（ oldest first）
   可选 limit 限制数量
   │
   ▼
2. 准备处理
   │
   ├── 加载字典 (DictSet)
   ├── 创建 AI 适配器 (Ollama/OpenAI/MiniMax)
   │
   ▼
3. 处理每个任务
   │
   ├── 移动到 processing/ (原子操作)
   │
   ├── 加载原始数据文件
   │
   ├── 调用 AI 分析
   │     │
   │     ├── Stage 1: 自由分析
   │     └── Stage 2: 字典对齐
   │
   ├── EventBuilder 生成 Event
   │
   ├── 写入 events/{year}/{month}/{id}.md
   │
   ├── 索引到 SQLite (EventRepository::upsert)
   │
   ├── 更新字典 (如有新术语发现)
   │
   ├── 移动到 done/
   │
   └── 记录日志
   │
   ▼
4. 完成
```

## AI 分析双阶段

### Stage 1：自由分析

AI 模型自由发挥，从数据中提取信息，不受约束：

```
Prompt:
分析这张图片，提取：
- 描述/摘要
- 事件类型
- 标签
- 涉及实体
- 置信度
```

### Stage 2：字典对齐

将 Stage 1 的结果与已有字典匹配，保证术语一致性：

```
Prompt:
将以下标签与字典对齐：
- 已有标签: [travel, japan, photo]
- 待对齐: ["旅游照片", "东京之旅"]
→ 对齐为: [travel, japan]
```

**新发现的术语**会被追加到字典文件，供后续使用。

## EventBuilder 协议

**核心原则：AI 绝不直接写 Markdown**

```
AI 输出 JSON
    │
    ▼
EventBuilder 验证并转换
    │
    ▼
Event 结构体
    │
    ▼
EventSerializer 生成 Markdown
    │
    ▼
写入文件
```

这种设计保证：
- 格式不会因 AI 输出不稳定而漂移
- Builder 可以添加系统字段（ID、时间戳等）
- 可以在写入前进行验证和修正

## 与 brain-core 的交互

```
brain-pipeline
    │
    └─── brain-core
          ├── BrainConfig                      # 配置
          ├── Database / EventRepository      # 索引
          ├── EventParser                     # (调试用)
          ├── EventSerializer                 # Markdown 生成
          ├── ModelAdapter trait              # AI 接口
          ├── OllamaAdapter / OpenAIAdapter / MiniMaxAdapter
          ├── DictSet / DictContext           # 字典
          └── Logger                          # 日志
```

## 调度方式

建议使用 cron 定时触发夜间处理：

```bash
# crontab -e
0 2 * * * /usr/local/bin/brain-pipeline process
```

或者手动触发：

```bash
brain process
brain process --limit 10
```
