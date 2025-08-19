# Project Manager CLI

一个用 Rust 编写的强大代码项目管理工具，支持扫描、分析和管理电脑中的各种代码项目。

## ✨ 功能特性

- 🔍 **智能项目扫描** - 自动识别 Git、Node.js、Python、Rust、Go、Java、C++ 等项目类型
- 📊 **详细统计信息** - 区分代码大小和依赖大小，提供完整的项目分析
- 🗂️ **Git 仓库分析** - 显示远程 URL、分支信息、提交历史和未提交更改
- ⚙️ **灵活配置** - 支持自定义忽略规则和扫描参数
- 🚀 **高性能扫描** - 异步并发处理，实时进度显示
- 🎯 **智能过滤** - 自动忽略系统目录、依赖目录和临时文件
- 📋 **多种输出格式** - 支持表格、JSON、CSV 等输出格式
- 🧹 **项目管理** - 支持清理依赖、删除项目等管理操作

## 🚀 快速开始

### 系统要求

- Rust 1.70 或更高版本
- Git（用于 Git 仓库分析功能）

### 安装依赖

确保你的系统已安装 Rust：

```bash
# 安装 Rust（如果还没有安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 验证 Rust 安装
rustc --version
cargo --version
```

### 构建项目

```bash
# 克隆项目（如果从远程仓库）
git clone <repository-url>
cd project-manager-cli

# 或者直接在项目目录中构建
cd /Users/smile/Documents/try-to-fly/project-manager-cli

# 安装依赖并构建
cargo build --release
```

### 运行项目

#### 开发模式运行

```bash
# 扫描当前目录
cargo run -- scan

# 扫描指定目录
cargo run -- scan ~/Documents ~/Projects

# 启动交互式 TUI 界面
cargo run -- tui

# 显示项目统计信息
cargo run -- stats ~/Documents

# 清理项目依赖
cargo run -- clean /path/to/project

# 查看帮助信息
cargo run -- --help
```

#### 生产模式运行

```bash
# 构建 release 版本
cargo build --release

# 运行构建好的二进制文件
./target/release/project-manager-cli scan ~/Documents
```

## 📦 打包和分发

### 方式一：本地构建

```bash
# 构建 release 版本
cargo build --release

# 二进制文件位置
ls -la target/release/project-manager-cli

# 复制到系统路径（可选）
sudo cp target/release/project-manager-cli /usr/local/bin/
```

### 方式二：使用 cargo install

```bash
# 从本地安装
cargo install --path .

# 安装后可在任何地方使用
project-manager-cli scan ~/Documents
```

### 方式三：交叉编译

```bash
# 安装交叉编译工具链
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add x86_64-unknown-linux-gnu

# 为 Windows 编译
cargo build --release --target x86_64-pc-windows-gnu

# 为 macOS 编译
cargo build --release --target x86_64-apple-darwin

# 为 Linux 编译
cargo build --release --target x86_64-unknown-linux-gnu
```

### 方式四：创建安装包

#### macOS (使用 cargo-bundle)

```bash
# 安装 cargo-bundle
cargo install cargo-bundle

# 在 Cargo.toml 中添加 bundle 配置
# [package.metadata.bundle]
# name = "Project Manager CLI"
# identifier = "com.example.project-manager-cli"

# 创建 macOS 应用包
cargo bundle --release
```

#### Linux (创建 DEB 包)

```bash
# 安装 cargo-deb
cargo install cargo-deb

# 创建 DEB 包
cargo deb

# 生成的包位置
ls -la target/debian/
```

#### Windows (创建 MSI 安装包)

```bash
# 安装 cargo-wix
cargo install cargo-wix

# 创建 WiX 配置
cargo wix init

# 构建 MSI 包
cargo wix --target x86_64-pc-windows-gnu
```

## 🛠️ 使用指南

### 基本命令

```bash
# 扫描项目
project-manager-cli scan [目录...]

# 启动 TUI 界面
project-manager-cli tui [目录...]

# 显示统计信息
project-manager-cli stats [目录...]

# 清理项目依赖
project-manager-cli clean <项目路径> --clean-type dependencies

# 删除项目到回收站
project-manager-cli delete <项目路径>

# 配置管理
project-manager-cli config show
project-manager-cli config edit
project-manager-cli config ignore <路径>
```

### 扫描选项

```bash
# 指定最大扫描深度
project-manager-cli scan ~/Documents --depth 5

# 指定输出格式
project-manager-cli scan ~/Documents --format json

# 保存结果到文件
project-manager-cli scan ~/Documents --output results.json

# 使用自定义配置文件
project-manager-cli --config custom-config.toml scan ~/Documents
```

### 配置文件

默认配置文件位置：
- macOS: `~/Library/Application Support/project-manager-cli/config.toml`
- Linux: `~/.config/project-manager-cli/config.toml`
- Windows: `%APPDATA%\project-manager-cli\config.toml`

示例配置：

```toml
# 扫描路径
scan_paths = [
    "~/Documents",
    "~/Projects",
    "~/Code"
]

[ignore]
# 忽略的目录
directories = [
    "node_modules",
    "target",
    ".git",
    "__pycache__"
]

# 忽略的文件扩展名
extensions = [
    "log",
    "tmp",
    "cache"
]

# 手动忽略的项目路径
projects = []

[scan]
# 最大扫描深度
max_depth = 10
# 是否跟随符号链接
follow_symlinks = false
# 并发扫描线程数
concurrent_scans = 4
# 是否扫描隐藏目录
scan_hidden = false

[display]
# 默认排序字段
default_sort = "LastModified"
# 大小显示单位
size_unit = "Auto"
# 时间格式
time_format = "%Y-%m-%d %H:%M:%S"
# 是否显示隐藏项目
show_hidden = false
```

## 🧪 开发和测试

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test scanner
cargo test git_analyzer
cargo test size_calculator

# 运行测试并显示输出
cargo test -- --nocapture

# 运行基准测试
cargo bench
```

### 代码检查

```bash
# 代码格式化
cargo fmt

# 代码检查
cargo clippy

# 安全审计
cargo audit
```

### 性能分析

```bash
# 安装性能分析工具
cargo install cargo-flamegraph

# 生成火焰图
cargo flamegraph --bin project-manager-cli -- scan ~/Documents
```

## 📁 项目结构

```
project-manager-cli/
├── src/
│   ├── main.rs              # 程序入口
│   ├── cli.rs               # CLI 参数定义
│   ├── config/              # 配置管理
│   │   ├── mod.rs
│   │   ├── settings.rs      # 配置文件解析
│   │   └── defaults.rs      # 默认配置
│   ├── scanner/             # 项目扫描模块
│   │   ├── mod.rs
│   │   ├── project_detector.rs  # 项目类型检测
│   │   ├── git_analyzer.rs      # Git 仓库分析
│   │   ├── size_calculator.rs   # 大小计算
│   │   └── file_walker.rs       # 文件遍历
│   ├── models/              # 数据模型
│   │   ├── mod.rs
│   │   ├── project.rs       # 项目信息结构
│   │   └── scan_result.rs   # 扫描结果
│   ├── tui/                 # TUI 界面（待实现）
│   ├── operations/          # 操作模块（待实现）
│   └── utils/               # 工具函数
├── Cargo.toml               # 项目配置
├── README.md                # 项目文档
└── tests/                   # 集成测试
```

## 🤝 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🔧 故障排除

### 常见问题

1. **编译错误**
   ```bash
   # 更新 Rust 工具链
   rustup update
   
   # 清理构建缓存
   cargo clean
   cargo build --release
   ```

2. **权限问题**
   ```bash
   # macOS/Linux 给予执行权限
   chmod +x target/release/project-manager-cli
   ```

3. **依赖问题**
   ```bash
   # 重新获取依赖
   cargo update
   ```

4. **Git 分析失败**
   - 确保系统已安装 Git
   - 检查目录是否为有效的 Git 仓库

### 性能优化

- 使用 `--depth` 参数限制扫描深度
- 在配置文件中添加更多忽略规则
- 调整 `concurrent_scans` 参数优化并发性能

## 📞 支持

如有问题或建议，请：
- 创建 [Issue](../../issues)
- 发送邮件至 [your-email@example.com]
- 查看 [Wiki](../../wiki) 了解更多信息

---

**Happy coding! 🚀**