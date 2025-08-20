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

### 安装方式

#### 方式一：自动安装脚本（推荐）

使用我们提供的安装脚本，自动下载并安装适合您系统的预编译版本：

```bash
# 使用 curl（推荐）
curl -fsSL https://raw.githubusercontent.com/try-to-fly/project-manager-cli/main/install.sh | bash

# 或者使用 wget
wget -qO- https://raw.githubusercontent.com/try-to-fly/project-manager-cli/main/install.sh | bash

# 也可以先下载脚本查看内容
curl -fsSL https://raw.githubusercontent.com/try-to-fly/project-manager-cli/main/install.sh -o install.sh
chmod +x install.sh
./install.sh
```

安装脚本会：
- 自动检测您的操作系统和架构（支持 Linux、macOS、Windows）
- 下载对应的预编译二进制文件
- 安装到 `~/.local/bin` 或 `~/bin`（无需 sudo 权限）
- 提示您配置 PATH 环境变量（如需要）

#### 方式二：直接下载预编译版本

从 [Releases](https://github.com/try-to-fly/project-manager-cli/releases) 页面下载适合您系统的版本：

**macOS：**
```bash
# Intel Mac
curl -L https://github.com/try-to-fly/project-manager-cli/releases/latest/download/project-manager-cli-macos-intel.tar.gz -o pm-cli.tar.gz

# Apple Silicon (M1/M2/M3)
curl -L https://github.com/try-to-fly/project-manager-cli/releases/latest/download/project-manager-cli-macos-arm64.tar.gz -o pm-cli.tar.gz

# 解压并安装
tar -xzf pm-cli.tar.gz
chmod +x project-manager-cli-*
mv project-manager-cli-* ~/.local/bin/project-manager-cli
```

**Linux：**
```bash
# x86_64
curl -L https://github.com/try-to-fly/project-manager-cli/releases/latest/download/project-manager-cli-linux-x86_64.tar.gz -o pm-cli.tar.gz

# 解压并安装
tar -xzf pm-cli.tar.gz
chmod +x project-manager-cli-*
mv project-manager-cli-* ~/.local/bin/project-manager-cli
```

**Windows：**
从 [Releases](https://github.com/try-to-fly/project-manager-cli/releases) 页面下载 Windows 版本并解压到合适的位置。

#### 方式三：从源码构建

如果您想从源码构建，需要先安装 Rust：

```bash
# 安装 Rust（如果还没有安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 验证 Rust 安装
rustc --version
cargo --version
```

然后构建项目：

```bash
# 克隆项目
git clone https://github.com/try-to-fly/project-manager-cli.git
cd project-manager-cli

# 构建 release 版本
cargo build --release

# 安装到系统（可选）
cargo install --path .
```

### 验证安装

安装完成后，验证是否成功：

```bash
# 查看版本
project-manager-cli --version

# 查看帮助
project-manager-cli --help
```

如果提示找不到命令，请确保安装目录在您的 PATH 中：

```bash
# 添加到 PATH（bash）
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# 添加到 PATH（zsh）
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

## 📖 使用示例

### 基本使用

```bash
# 扫描当前目录
project-manager-cli scan

# 扫描指定目录
project-manager-cli scan ~/Documents ~/Projects

# 启动交互式 TUI 界面
project-manager-cli tui

# 显示项目统计信息
project-manager-cli stats ~/Documents

# 清理项目依赖
project-manager-cli clean /path/to/project

# 查看帮助信息
project-manager-cli --help
```

### 高级用法

```bash
# 指定最大扫描深度
project-manager-cli scan ~/Documents --depth 5

# 输出为 JSON 格式
project-manager-cli scan ~/Documents --format json

# 保存结果到文件
project-manager-cli scan ~/Documents --output results.json

# 使用自定义配置文件
project-manager-cli --config custom-config.toml scan ~/Documents
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