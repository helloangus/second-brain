# brain-indexerd：文件系统监控守护进程

## 概述

`brain-indexerd` 是一个后台守护进程，持续监控 `events/` 和 `entities/` 目录。当 Markdown 文件发生变化时，自动更新 SQLite 索引，保证查询结果的实时性。

## 工作流程

```
启动
  │
  ▼
加载配置 (BrainConfig::load)
  │
  ▼
运行初始索引 (index_existing_files)
  │  扫描现有所有 .md 文件
  │  解析并写入 SQLite
  │
  ▼
启动文件系统监控 (notify crate)
  │
  ▼
循环等待文件系统事件
  │
  ├── Create/Modify ──► process_file() ──► upsert 到 SQLite
  │
  └── Remove ──► remove_file() ──► delete 从 SQLite
```

## 核心技术

### notify crate

使用 `notify` 库实现跨平台文件监控：

```rust
let mut watcher = RecommendedWatcher::new(
    move |res: Result<Event, notify::Error>| { ... },
    Config::default().with_poll_interval(Duration::from_secs(2)),
)?;
watcher.watch(&events_path, RecursiveMode::Recursive)?;
watcher.watch(&entities_path, RecursiveMode::Recursive)?;
```

**配置说明：**
- `RecommendedWatcher`：平台最佳实现（Linux 用 inotify，macOS 用 FSEvents）
- `poll_interval = 2s`：当原生事件不可用时的轮询间隔
- `RecursiveMode::Recursive`：监控所有子目录

### 事件处理

| 文件系统事件 | 处理方式 |
|-------------|---------|
| Create | 解析 Markdown，upsert 到 SQLite |
| Modify | 重新解析 Markdown，upsert 到 SQLite |
| Remove | 从 SQLite 删除对应记录 |

### 初始索引

启动时，先对现有文件建立索引，确保数据库与文件系统一致：
1. 递归遍历 `events/` 和 `entities/` 目录
2. 对每个 `.md` 文件调用 `process_file()`
3. 使用 `EventRepository::upsert()` 或 `EntityRepository::upsert()`

## 与 brain-core 的交互

```
brain-indexerd
    │
    └─── brain-core
          ├── BrainConfig::load()          # 加载配置
          ├── Database::open()             # 打开 SQLite
          ├── EventParser                  # 解析事件 Markdown
          ├── EntityParser                 # 解析实体 Markdown
          ├── EventRepository::upsert()    # 写入事件
          ├── EventRepository::delete()    # 删除事件
          ├── EntityRepository::upsert()   # 写入实体
          └── EntityRepository::delete()   # 删除实体
```

## 部署方式

建议使用 systemd 管理：

```ini
[Unit]
Description=Brain Indexer Daemon

[Service]
ExecStart=/usr/local/bin/brain-indexerd
WorkingDirectory=/home/user/second-brain
Restart=always

[Install]
WantedBy=multi-user.target
```

## 设计考量

**为何需要实时索引？**
- CLI 的 `search`、`timeline`、`today` 命令依赖 SQLite 查询
- 如果索引不实时，用户会看到过期数据

**为何选择文件系统监控而非轮询？**
- 更高效：只在变化时更新
- 更及时：事件驱动，接近实时

**poll_interval = 2s 的意义：**
- 作为 fallback，当底层事件机制不可用时
- 2 秒足够短，保证基本实时性
