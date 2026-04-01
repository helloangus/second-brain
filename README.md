# Second Brain - Source Code

个人认知系统的 Rust 源码仓库。

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

# Release 构建 (推荐)
cargo build --release

# 二进制位置: target/release/brain
```

### 交叉编译 (Linux → ARM64)

```bash
# 安装交叉编译工具链
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu

# 编译
cargo build --release --target aarch64-unknown-linux-gnu
```

## 发布新版本

```bash
# 打标签触发 CI/CD
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions 会自动编译并创建 Release。

## 项目结构

- `crates/brain-core/` - 核心库 (模型、数据库、Markdown 解析)
- `crates/brain-cli/` - 命令行工具
- `crates/brain-indexerd/` - 文件系统索引守护进程
- `crates/brain-pipeline/` - AI 处理流水线

## 许可证

MIT