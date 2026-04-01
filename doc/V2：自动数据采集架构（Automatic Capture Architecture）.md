# V2：自动数据采集架构（Automatic Capture Architecture）

设计重点将严格围绕你之前的约束：

- ✅ 长期记忆优先 &#x20;
- ✅ 半主动 → 未来全自动 &#x20;
- ✅ 多类型数据 &#x20;
- ✅ Linux server 风格 &#x20;
- ✅ 本地优先 &#x20;
- ✅ 延迟批处理 &#x20;
- ✅ 标准格式 &#x20;
- ✅ 可逐步实现

# V2 总体目标

V1 已经解决：

```markdown 
Event Storage + Index + AI Pipeline
```


V2 要解决的是：

```text 
Event Storage + Index + AI Pipeline
```


***

# 一、V2 总体架构

这是自动采集的真实结构（建议不要偏离）：

```markdown 
            ┌────────────────────┐
            │   Capture Agents    │
            │ (手机/PC/浏览器等)  │
            └──────────┬─────────┘
                       ↓
            ┌────────────────────┐
            │  Sync Gateway       │ ⭐唯一入口
            └──────────┬─────────┘
                       ↓
            ┌────────────────────┐
            │   Raw Data Lake     │
            └──────────┬─────────┘
                       ↓
            ingest queue
                       ↓
                  AI Pipeline
```


核心思想：

> **所有设备不直接接触 AI 系统，只上传原始数据。**

这是长期稳定的关键。

***

# 二、Capture 分层模型（极重要）

不要把采集当成一个系统。

它其实是**4 类完全不同的问题**：

| 层级 | 名称               | 自动程度 |
| -- | ---------------- | ---- |
| L1 | Manual Capture   | 手动   |
| L2 | Assisted Capture | 半自动  |
| L3 | Passive Capture  | 被动   |
| L4 | Ambient Capture  | 环境级  |

你当前目标：

L2 → L3

***

# 三、统一输入原则（必须遵守）

所有采集必须最终变成：

```json 
{
  "timestamp": "...",
  "source": "...",
  "device": "...",
  "type": "...",
  "payload_path": "..."
}
```


这叫：

## Raw Ingest Record（RIR）

AI 不参与这一层。

***

# 四、Sync Gateway（V2 核心组件 ⭐）

这是整个自动化系统的大脑入口。

***

## 为什么必须存在？

否则你会得到：

- 不同设备格式不同 &#x20;
- 时间混乱 &#x20;
- 重复文件 &#x20;
- 数据污染 &#x20;

***

## Gateway 职责

```markdown 
接收上传
↓
校验
↓
标准化命名
↓
去重
↓
写入 Raw Data Lake
↓
生成 ingest task
```


***

## 推荐实现（极简）

一个本地 HTTP 服务：

`brain-gateway`

例如：

`POST /ingest`

上传：

- 图片 &#x20;
- 文字
- 音频 &#x20;
- 视频
- URL &#x20;

***

# 五、Raw Data Lake 设计（长期稳定关键）

必须 immutable（不可修改）。

***

## 命名规则（非常重要）

```text 
data/raw/YYYY/MM/DD/
```


例：

```markdown 
data/raw/2026/03/31/
    20260331T192211_phone_photo.jpg
```


***

### 文件名必须包含：

`时间 + 来源 + 类型`

因为：

👉 文件名本身就是 metadata fallback。

即使数据库毁灭仍可恢复。

***

# 六、Capture Agents（具体设计）

现在进入你真正关心的部分：

> 怎么自动采集？

***

## 6.1 手机 Agent（最关键）

未来 70% 数据来自手机。

***

### V2 阶段建议能力（按优先级）

***

## ⭐ Phase 1（立刻可实现）

无需开发 APP。

### 方法：

手机 → 自动同步目录 → Gateway

工具建议：

- Syncthing（强烈推荐） &#x20;
- 或 Tailscale + rsync &#x20;

***

### 同步内容：

```markdown 
DCIM/
Screenshots/
Recordings/
Exports/
```


手机只负责：

`产生文件 → 自动同步`

系统负责理解。

***

## ⭐ Phase 2（轻量 App）

未来你开发：

### 功能：

- Share → Send to Brain
- 一键收藏链接 &#x20;
- 手动录音 &#x20;
- 截屏发送
- 快速笔记 &#x20;

本质：

`Universal Capture Button`

***

上传格式：

```json 
{
  "type": "shared_link",
  "url": "...",
  "comment": "..."
}
```


***

## 6.2 PC Agent

PC 采集价值巨大但常被忽视。

***

### 浏览记录采集（高价值 ⭐⭐⭐⭐⭐）

来源：

- Chrome history sqlite &#x20;
- Firefox places.db &#x20;

定时任务：

```bash 
every 30 min:
    export new history
```


生成：

```json 
{
  "type": "web_visit",
  "url": "...",
  "title": "...",
  "time": ...
}
```


***

### 截图监听

监听：

`~/Pictures/Screenshots`

自动 ingest。

***

### CLI 捕获（非常适合你）

```markdown 
brain note "新的研究想法"
```


直接生成 raw text。

***

## 6.3 语音采集（建议策略）

不要实时监听（灾难）。

采用：

### 会话式录音

你主动触发：

`brain record`

结束后自动上传。

原因：

- 节省算力 &#x20;
- 减少噪声 &#x20;
- 隐私可控 &#x20;

***

## 6.4 Web 收藏统一入口 ⭐⭐⭐⭐⭐

极高价值设计。

***

你未来所有收藏必须经过：

`https://brain.local/save?url=...`

Gateway：

1. 下载网页 &#x20;
2. 保存 HTML &#x20;
3. 提取正文 &#x20;
4. 排队 AI 分析 &#x20;

***

# 七、去重系统（必须 V2 就存在）

否则半年后崩溃。

***

## 文件去重

计算：

`SHA256(file)`

数据库记录 hash。

重复 → 仅建立引用。

***

## 内容去重（高级）

后期：

- embedding 相似度 &#x20;
- 相似事件合并提示 &#x20;

***

# 八、Ingest Queue（连接 V1）

Gateway 最终只做一件事：

写入：

`/pipelines/queue/pending/`

任务：

```json 
{
  "raw_path": "...",
  "capture_type": "phone_photo"
}
```


然后 V1 pipeline 接管。

***

# 九、自动化等级路线图（现实可行）

这是你未来 12–18 个月自然演进路径：

***

## Stage A（现在）

✅ 手动导入 &#x20;

✅ Syncthing 同步 &#x20;

✅ CLI capture

成本：极低。

***

## Stage B（3个月）

✅ Gateway &#x20;

✅ 浏览记录自动采集 &#x20;

✅ 自动截图 ingest

自动化 ≈ 60%。

***

## Stage C（6–12个月）

✅ 手机 capture app &#x20;

✅ 统一分享入口 &#x20;

✅ 自动日志生成

自动化 ≈ 85%。

***

## Stage D（长期）

✅ 环境感知 &#x20;

✅ 主动记忆提示 &#x20;

✅ context recall

真正第二大脑出现。

***

# 十、V2 的一个关键哲学（非常重要）

你系统成功与否取决于：

> **Capture friction 是否趋近于 0。**

规则：

`任何超过3秒的输入行为都会被放弃。`

所以：

- 自动 > 半自动 > 手动 &#x20;
- 上传优先，理解延后
