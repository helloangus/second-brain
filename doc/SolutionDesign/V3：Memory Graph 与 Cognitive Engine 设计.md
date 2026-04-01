# V3：Memory Graph 与 Cognitive Engine 设计

这一部分是你的「第二大脑」从**数据仓库 → 真正认知系统** 的跃迁层。

如果说：

- **V1 = 记忆存储（能记住）** &#x20;
- **V2 = 自动感知（能收集）** &#x20;

那么：

> **V3 = 理解 + 联想 + 思考（能“想”）**

这一层决定系统是否真的成为*Second Brain*，而不是 AI 文件管理器。

***

# V3 总体目标

你的核心理念已经非常接近认知科学模型：

> 一切都是 Event，时间是属性，实体长期存在。

V3 要实现：

```markdown 
Event 集合
      ↓
结构化关系
      ↓
Memory Graph
      ↓
Cognitive Engine
      ↓
主动联想 & 思考辅助
```


***

# 一、Memory Graph（记忆图谱）

## 1.1 为什么不能只靠标签？

标签只能表达：

`属于`

但人的记忆是：

```markdown 
参与
影响
因果
持续
重复
相似
演化
```


例如：

```markdown 
参与
影响
因果
持续
重复
相似
演化
```


这是图，不是分类。

***

## 1.2 Graph 的真实节点类型

你已经隐含定义了四类核心节点：

| Node    | 是否长期存在 |
| ------- | ------ |
| Event   | ❌（瞬时）  |
| Person  | ✅      |
| Project | ✅      |
| Concept | ✅      |
| Place   | ✅      |

建议正式化：

***

## Node Types

```markdown 
Entity (长期存在)
    ├── Person
    ├── Project
    ├── Concept
    ├── Place
    └── Object
```


Event 永远是：

`Edge-rich transient node`

***

## 1.3 Graph 结构（核心模型）

不是 Neo4j 式复杂图。

而是：

> **Event-centered bipartite graph**

结构：

`Entity ← participates → Event ← relates → Entity`

***

### 示例

```markdown 
[你]
   │
participated
   │
[讨论GPU虚拟化]
   │
mentions
   │
[GVT-g]
```


***

## 1.4 为什么 Event 必须在中心？

否则你会得到：

- 主观总结污染事实 &#x20;
- AI hallucination 扩散 &#x20;

Event 是：

`不可修改的历史事实`

这是系统稳定性的根。

***

# 二、Memory Graph 存储设计

⚠️ 不建议上 Neo4j。

原因：

- 运维复杂 &#x20;
- Git 不兼容 &#x20;
- 不可恢复 &#x20;
- 锁定技术栈

***

## 推荐方案（极稳定）

仍然：

`Markdown + SQLite`

但增加：

### graph\_edges 表

```sql 
CREATE TABLE edges (
    src TEXT,
    dst TEXT,
    relation TEXT,
    weight REAL,
    event_id TEXT
);
```


***

### 关键思想

图关系**来自 Event**。

不是 AI 随便生成。

***

例：

```yaml 
event:
  entities:
    - person: you
    - concept: gpu virtualization
```


自动生成：

`you --discussed--> gpu_virtualization`

***

# 三、关系类型（必须受控）

不要让 AI 发明关系。

必须有限集合。

***

## 建议关系集合

```markdown 
participated_in
mentioned
created
worked_on
learned
related_to
caused
continued_as
inspired
```


原因：

👉 图谱稳定 > 表达力。

***

# 四、Graph 构建 Pipeline

这是自动发生的。

***

```markdown 
New Event
    ↓
Entity Extraction
    ↓
Relation Inference
    ↓
Edge Builder
    ↓
Graph Update
```


***

## Relation 推断规则（重要）

优先：

`规则 > AI`

例如：

| 条件                     | 关系                 |
| ---------------------- | ------------------ |
| Person + Meeting Event | participated\\\_in |
| Concept in text        | mentioned          |
| Project tag            | worked\\\_on       |

AI 只用于：

`无法规则化的弱关系`

***

# 五、Cognitive Engine（认知引擎）

现在进入真正的核心。

Memory Graph 只是数据库。

Cognitive Engine 才是“大脑”。

***

## 5.1 Cognitive Engine 的职责

不是聊天。

而是持续运行：

```markdown 
观察记忆变化
发现模式
生成认知输出
```


***

它运行类似：

`cron + reasoning agent`

***

## 5.2 四种认知能力（建议阶段实现）

***

## ① Temporal Reflection（时间反思）

每天：

```markdown 
今日 events
    ↓
聚类
    ↓
生成 daily log
```


不是总结。

而是：

`发现主题趋势`

***

输出：

`logs/2026-03-31.md`

***

## ② Pattern Detection（模式发现 ⭐）

例如系统发现：

```markdown 
连续3天：
GPU + 深夜 + 音乐
```


生成：

```yaml 
pattern:
  type: recurring_context
```


这是真正的“第二大脑”。

***

## ③ Memory Recall（主动联想）

当你查询：

`GPU virtualization`

系统不仅返回匹配。

还返回：

```markdown 
历史高关联事件
相关人物
过去类似阶段
```


***

算法：

- semantic search &#x20;
- graph expansion (2 hops)

***

## ④ Cognitive Suggestions（半主动）

符合你 Q2 的“半主动”。

例如：

```markdown 
你又开始研究GPU虚拟化，
上次停在：
2025-05 实验设计阶段。
```


这是：

`context resurrection`

***

# 六、Memory Activation 模型（关键创新点）

人类记忆不是搜索触发。

而是：

`activation spreading`

***

模型：

```markdown 
Query Node
    ↓
邻接传播
    ↓
权重衰减
    ↓
最活跃子图
```


***

评分函数示意：

```python 
score =
semantic_similarity
+ graph_distance_weight
+ recency_decay
+ repetition_boost
```


***

# 七、为什么这套设计会长期成立

因为它满足：

| 原则    | 是否满足 |
| ----- | ---- |
| 可恢复   | ✅    |
| 模型可替换 | ✅    |
| 低算力   | ✅    |
| 可解释   | ✅    |
| 渐进升级  | ✅    |
| Git友好 | ✅    |

***

# 八、V3 完成后的能力（你将获得）

系统将开始表现出：

- 记住长期项目上下文 &#x20;
- 自动连接生活与研究 &#x20;
- 发现重复行为模式 &#x20;
- 帮助思考而不是回答 &#x20;

换句话说：

> 从**信息系统 → 认知外骨骼**。
