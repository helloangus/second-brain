# V5：Proactive Cognition Layer（主动认知层）

这是整个「第二大脑」真正跨越工具边界的一层 —— &#x20;

从：

`你 → 主动查询系统`

变成：

`系统 → 在正确时机参与思考`

注意：

> V5 不是“提醒系统”，而是**认知参与系统（Cognitive Participation）**。

如果设计错误，它会变成烦人的通知机器；
设计正确，它会成为一种**外部前额叶（external prefrontal cortex）**。

***

# V5 总体定位

到目前为止：

| 层  | 能力     |
| -- | ------ |
| V1 | 存储     |
| V2 | 自动采集   |
| V3 | 记忆理解   |
| V4 | 自进化    |
| V5 | 主动思考 ⭐ |

***

## V5 的核心问题

什么时候 AI 应该主动出现？

错误答案：

`有新数据 → 提醒`

正确答案：

`出现“认知价值窗口”（Cognitive Opportunity Window）`

***

# 一、Proactive Cognition 的原则

必须同时满足：

```markdown 
Relevant（相关）
Timely（时机正确）
Low-friction（低打扰）
Actionable（可行动）
```


缺一个就不要触发。

***

# 二、主动认知的触发模型（核心）

系统不基于事件触发。

而基于：

## Cognitive Signals（认知信号）

***

### 信号类型（建议固定集合）

| Signal        | 含义       |
| ------------- | -------- |
| Repetition    | 你反复接触同概念 |
| Accumulation  | 某主题快速增长  |
| Gap           | 长期未完成    |
| Transition    | 人生阶段变化   |
| Context Match | 当前情境匹配过去 |
| Anomaly       | 行为异常     |
| Resurface     | 应被重新想起   |

***

## 示例

### Repetition Signal

7 天内： &#x20;
`GPU virtualization 出现 11 次`

触发：

`你是否正在形成新的研究重点？`

***

### Gap Signal

```markdown 
Project X:
最后活动：42 天前
未标记完成
```


触发：

`是否需要关闭或恢复？`

***

# 三、Proactive Engine 架构

```markdown 
Memory Graph
      ↓
Signal Detector
      ↓
Cognitive Evaluator
      ↓
Intervention Generator
      ↓
Delivery Layer
```


***

## 3.1 Signal Detector

周期性运行（建议 nightly batch）。

输入：

`Events + Graph + Time`

输出：

```json 
signal {
  type
  strength
  entities
}
```


***

### 示例

```json 
{
  "type": "accumulation",
  "entity": "Rust virtualization",
  "strength": 0.83
}
```


***

## 3.2 Cognitive Evaluator（关键过滤层）

90% 信号必须被丢弃。

否则：

`系统 = 噪声制造机`

***

评估函数：

```python 
score =
 relevance × novelty × actionability × confidence
```


只有：

`score > threshold`

才允许出现。

***

# 四、Intervention Types（干预类型）

不是所有主动行为都一样。

建议限制为 5 类。

***

## 1️⃣ Reflection（反思）

最重要类型。

生成：

```text 
你最近持续在研究 X，
是否希望整理阶段性认知？
```


输出：

- 自动生成 reflection draft &#x20;
- 可写入 journal &#x20;

***

## 2️⃣ Connection（连接发现）

跨时间联想。

```markdown 
你现在的 GPU 研究与
2024 年的 ARM MMU 工作高度相关。
```


这是第二大脑最“魔法”的能力。

***

## 3️⃣ Recall（记忆唤醒）

基于时间或情境。

`去年今天你在东京开始某项目。`

***

## 4️⃣ Focus Correction（注意力校正）

检测分散：

`过去3天主题切换频率异常高。`

***

## 5️⃣ Idea Synthesis（思想合成）

当图谱密度足够：

`自动提出潜在研究方向。`

（非常高阶）

***

# 五、Delivery Layer（如何出现）

这是成败关键。

***

## ❌ 不要：

- 手机通知轰炸 &#x20;
- 即时打断 &#x20;

***

## ✅ 推荐顺序

### Level 1（默认）

每日 Digest：

```markdown 
obsidian/daily/AI Digest.md
```


你主动查看。

***

### Level 2

打开 Obsidian 时显示：

`Today Cognitive Insights`

***

### Level 3（未来）

上下文感知提示：

`打开某项目 → 显示相关记忆`

***

# 六、Cognitive Budget（必须设计 ⭐）

系统每天允许主动次数有限。

例如：

```yaml 
daily_budget:
  reflections: 1
  connections: 2
  recalls: 2
```


否则长期必被关闭。

***

# 七、长期学习你的偏好（Meta Learning）

系统记录：

```markdown 
你接受了哪些建议
忽略了哪些
```


更新：

`intervention_policy.json`

逐渐学会：

`什么时间不要打扰你`

***

# 八、与 V3 Memory Graph 的关系

V5 本质是：

`Graph Dynamics Analysis`

不是 LLM 聊天。

LLM 只负责：

`解释结果`

推理来自：

- 图结构 &#x20;
- 时间模式 &#x20;
- 行为统计 &#x20;

***

# 九、V5 的最小可实现版本（强烈建议）

不要一开始做复杂。

### MVP：

仅实现：

- Repetition Signal &#x20;
- Daily Digest

你已经会感受到差异。

***

## MVP Pipeline

```markdown 
nightly job
   ↓
count entity frequency
   ↓
detect spike
   ↓
generate insight
   ↓
append digest.md
```


***

# 十、V5 达成后的系统形态

你的第二大脑将变为：

```markdown 
Passive Archive      ❌
Search Engine        ❌
AI Assistant         ❌

Cognitive Partner    ✅
```


它不会替你思考。

但会：

```markdown 
在你即将产生思考时，
把正确的记忆递到你面前。
```
