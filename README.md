# Second Brain - Source Code

个人认知系统的 Rust 源码仓库。基于事件驱动架构，所有生命数据通过 Event 流入可计算的记忆系统。

## 下载预编译版本

推荐直接下载 GitHub Releases 中的预编译二进制：
https://github.com/helloangus/second-brain/releases

## 从源码编译

### 前置要求

- Rust 1.70+ (通过 [rustup](https://rustup.rs/) 安装)
- SQLite 开发库

### Ubuntu/Debian

```bash
sudo apt install libsqlite3-dev
```

### macOS

```bash
brew install sqlite3
```

### 编译

```bash
# 克隆仓库
git clone https://github.com/helloangus/second-brain.git
cd brain-src

# Debug 构建
cargo build

# Release 构建 (推荐，二进制位于 target/release/brain)
cargo build --release

# 运行 CLI
./target/release/brain --help

# 运行测试
cargo test
```

### 交叉编译 (Linux → ARM64)

```bash
# 安装交叉编译工具链
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu

# 编译
cargo build --release --target aarch64-unknown-linux-gnu
```

## 工作空间结构

```
brain-src/
├── Cargo.toml              # Workspace 根配置
├── crates/
│   ├── brain-core/         # 核心库
│   │   ├── src/
│   │   │   ├── adapters/  # AI 适配器 (Ollama, OpenAI)
│   │   │   ├── db/        # 数据库层 (SQLite, Repository)
│   │   │   ├── markdown/  # Markdown 解析/序列化
│   │   │   └── models/     # 数据模型 (Event, Entity, Task, Tag)
│   │   └── Cargo.toml
│   ├── brain-cli/          # CLI 工具
│   │   ├── src/
│   │   │   ├── main.rs    # CLI 定义
│   │   │   └── commands/  # search, timeline, today, add, entity, stats
│   │   └── Cargo.toml
│   ├── brain-indexerd/    # 文件系统索引守护进程
│   │   ├── src/
│   │   │   ├── main.rs    # 守护进程入口
│   │   │   └── processor.rs  # 文件处理逻辑
│   │   └── Cargo.toml
│   └── brain-pipeline/    # AI 处理流水线
│       ├── src/
│       │   ├── main.rs    # CLI 定义
│       │   ├── builder.rs # Event 构建器
│       │   ├── processor.rs # 任务处理器
│       │   └── queue.rs   # 队列管理
│       └── Cargo.toml
└── doc/
    ├── Implementation/     # 实现文档
    └── SolutionDesign/     # 设计文档 (V1-V7)
```

## 核心设计原则

- **Local-first**: 数据本地存储，不依赖云服务
- **Markdown 真相**: 所有数据以 Markdown + YAML frontmatter 存储
- **Git 版本控制**: events/ 和 entities/ 纳入 Git 管理
- **AI 适配器抽象**: `ModelAdapter` trait 支持灵活切换 AI 模型
- **Event Builder 协议**: AI 只输出 JSON，由 Builder 控制文件生成

## 核心模块

### brain-core
- `BrainConfig` - 配置管理
- `Database` - SQLite 连接管理
- `EventRepository`, `EntityRepository`, `TagRepository` - 数据访问
- `EventParser`, `EventSerializer` - Markdown 解析/序列化
- `ModelAdapter` trait - AI 模型统一接口

### brain-cli
- `search` - FTS5 全文搜索
- `timeline` - 月度时间线视图
- `today` - 今日事件
- `add` - 添加新事件
- `entity list|show` - 实体管理
- `stats` - 统计信息

### brain-indexerd
- 文件系统监听 (notify crate)
- 自动索引更新
- 启动时全量索引

### brain-pipeline
- 文件队列 (pending/processing/done)
- AI 模型调用
- Event 自动生成

## 发布新版本

```bash
# 打标签触发 CI/CD
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions 会自动编译并创建 Release。

## 许可证

MIT