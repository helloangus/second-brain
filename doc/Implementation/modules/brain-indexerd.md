# brain-indexerd 索引守护进程

位置：`crates/brain-indexerd/src/`

## 模块结构

```
brain-indexerd/src/
├── main.rs           # 主入口
└── processor.rs      # 文件处理器
```

---

## 功能概述

**职责：**
- 监控 `events/` 和 `entities/` 目录的文件变化
- 文件变更时自动更新 SQLite 索引
- 启动时全量索引已有文件

**使用的 crate：**
- `notify` - 文件系统监听
- `tokio` - 异步运行时
- `walkdir` - 目录遍历

---

## 核心组件

### EventProcessor
`processor.rs`

负责解析 markdown 文件并更新数据库。

```rust
pub struct EventProcessor<'a> {
    db: &'a Database,
}
```

**主要方法：**

| 方法 | 说明 |
|------|------|
| `process_file(path)` | 处理创建/修改事件 |
| `remove_file(path)` | 处理文件删除 |

**文件类型判断：**
```rust
fn is_event_file(&self, path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "events")
}

fn is_entity_file(&self, path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "entities")
}
```

---

## 运行流程

### 1. 初始化

```rust
// 加载配置
let config = BrainConfig::load()?;

// 打开数据库
let db = Database::open(&config.db_path)?;

// 全量索引已有文件
processor::index_existing_files(&db, &events_path, &entities_path)?;
```

### 2. 启动文件系统监听

```rust
// 在独立线程中运行
std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_watcher(events_path, entities_path, state).await
    });
});
```

### 3. 监听循环

```rust
async fn run_watcher(events_path, entities_path, state) {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(2)),
    )?;

    // 监听两个目录
    watcher.watch(&events_path, RecursiveMode::Recursive)?;
    watcher.watch(&entities_path, RecursiveMode::Recursive)?;

    // 处理事件
    for event in rx {
        handle_event(&event, &state).await;
    }
}
```

### 4. 事件处理

```rust
async fn handle_event(event: &Event, state: &Arc<Mutex<IndexerState>>) {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            processor.process_file(&path)?;
        }
        EventKind::Remove(_) => {
            processor.remove_file(&path)?;
        }
        _ => {}
    }
}
```

---

## 全量索引

`processor.rs`

**index_existing_files 函数：**

启动时遍历 `events/` 和 `entities/` 目录，处理所有 `.md` 文件：

```rust
pub fn index_existing_files(
    db: &Database,
    events_path: &Path,
    entities_path: &Path,
) -> Result<(), brain_core::Error>
```

**处理流程：**
```
walkdir::WalkDir 遍历目录
    ↓
过滤 .md 文件
    ↓
读取文件内容
    ↓
EventParser::parse / EntityParser::parse
    ↓
EventRepository::upsert / EntityRepository::upsert
```

**错误处理：**
- 解析失败 → 记录警告，继续处理
- 写入失败 → 记录错误，继续处理

---

## 数据库操作

### process_event

```rust
fn process_event(&self, content: &str, conn: &Connection) -> Result<(), brain_core::Error> {
    match EventParser::parse(content) {
        Ok(event) => {
            let repo = EventRepository::new(conn);
            repo.upsert(&event)?;
            info!("Indexed event: {}", event.id);
        }
        Err(e) => {
            warn!("Failed to parse event file: {}", e);
        }
    }
    Ok(())
}
```

### remove_file

```rust
fn remove_file(&self, path: &Path) -> Result<(), brain_core::Error> {
    if let Some(id) = self.extract_id(path) {
        if self.is_event_file(path) {
            let repo = EventRepository::new(conn);
            repo.delete(&id)?;
        } else if self.is_entity_file(path) {
            let repo = EntityRepository::new(conn);
            repo.delete(&id)?;
        }
    }
    Ok(())
}
```

---

## 配置

通过 `BrainConfig` 获取路径：

```rust
struct IndexerState {
    db: Database,
    events_path: PathBuf,
    entities_path: PathBuf,
}
```

**典型配置：**
```yaml
db_path: index/events.db
events_path: events
entities_path: entities
```

---

## 限制与注意事项

1. **轮询间隔：** 使用 2 秒轮询间隔，可能有短暂延迟
2. **无重试机制：** 解析/写入失败直接跳过
3. **单向同步：** markdown → 数据库，删除数据库记录不会删除文件
4. **无事件去重：** 同一文件的多次快速修改可能触发多次处理
