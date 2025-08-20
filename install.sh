#!/bin/bash

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 配置
REPO="try-to-fly/project-manager-cli"
BINARY_NAME="project-manager-cli"

# 打印带颜色的消息
print_error() {
    echo -e "${RED}错误: $1${NC}" >&2
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}➜ $1${NC}"
}

# 检测操作系统和架构
detect_platform() {
    local os=""
    local arch=""
    
    # 检测操作系统
    case "$(uname -s)" in
        Linux*)     os="linux";;
        Darwin*)    os="darwin";;
        CYGWIN*|MINGW*|MSYS*) os="windows";;
        *)          
            print_error "不支持的操作系统: $(uname -s)"
            exit 1
            ;;
    esac
    
    # 检测架构
    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64";;
        aarch64|arm64)  arch="aarch64";;
        armv7l|armhf)   arch="armv7";;
        i386|i686)      arch="i686";;
        *)              
            print_error "不支持的架构: $(uname -m)"
            exit 1
            ;;
    esac
    
    # 特殊处理 macOS 架构名称
    if [ "$os" = "darwin" ]; then
        if [ "$arch" = "x86_64" ]; then
            arch="x86_64"
        elif [ "$arch" = "aarch64" ]; then
            arch="aarch64"
        fi
    fi
    
    echo "${os}-${arch}"
}

# 获取最新版本号
get_latest_version() {
    # 使用 GitHub API 获取最新发布版本
    local latest_release=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest")
    
    if [ -z "$latest_release" ] || [ "$latest_release" = "null" ]; then
        print_error "无法获取最新版本信息"
        exit 1
    fi
    
    local version=$(echo "$latest_release" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    
    if [ -z "$version" ]; then
        print_error "无法解析版本号"
        exit 1
    fi
    
    echo "$version"
}

# 构建下载 URL
build_download_url() {
    local version=$1
    local platform=$2
    
    # 映射平台到实际的文件名
    local filename=""
    case "$platform" in
        linux-x86_64)
            filename="${BINARY_NAME}-linux-x86_64.tar.gz"
            ;;
        linux-aarch64)
            filename="${BINARY_NAME}-linux-aarch64.tar.gz"
            ;;
        linux-armv7)
            filename="${BINARY_NAME}-linux-armv7.tar.gz"
            ;;
        linux-i686)
            filename="${BINARY_NAME}-linux-i686.tar.gz"
            ;;
        darwin-x86_64)
            filename="${BINARY_NAME}-macos-intel.tar.gz"
            ;;
        darwin-aarch64)
            filename="${BINARY_NAME}-macos-arm64.tar.gz"
            ;;
        windows-x86_64)
            filename="${BINARY_NAME}-windows-x86_64.zip"
            ;;
        windows-i686)
            filename="${BINARY_NAME}-windows-i686.zip"
            ;;
        *)
            print_error "不支持的平台: $platform"
            exit 1
            ;;
    esac
    
    echo "https://github.com/${REPO}/releases/download/${version}/${filename}"
}

# 确定安装目录
get_install_dir() {
    # 优先使用 ~/.local/bin
    if [ -d "$HOME/.local/bin" ]; then
        echo "$HOME/.local/bin"
    # 其次使用 ~/bin
    elif [ -d "$HOME/bin" ]; then
        echo "$HOME/bin"
    # 如果都不存在，创建 ~/.local/bin
    else
        mkdir -p "$HOME/.local/bin"
        echo "$HOME/.local/bin"
    fi
}

# 下载并安装
download_and_install() {
    local url=$1
    local install_dir=$2
    local platform=$3
    
    print_info "下载中: $url"
    
    # 创建临时目录
    local temp_dir=$(mktemp -d)
    trap "rm -rf $temp_dir" EXIT
    
    # 判断是 zip 还是 tar.gz
    local is_zip=false
    if [[ "$url" == *.zip ]]; then
        is_zip=true
    fi
    
    # 下载文件
    if [ "$is_zip" = true ]; then
        local archive_file="$temp_dir/download.zip"
    else
        local archive_file="$temp_dir/download.tar.gz"
    fi
    
    if ! curl -L -o "$archive_file" "$url"; then
        print_error "下载失败"
        exit 1
    fi
    
    print_success "下载完成"
    
    # 解压文件
    print_info "解压中..."
    cd "$temp_dir"
    
    if [ "$is_zip" = true ]; then
        if ! command -v unzip &> /dev/null; then
            print_error "需要 unzip 工具来解压文件"
            exit 1
        fi
        unzip -q "$archive_file"
    else
        tar -xzf "$archive_file"
    fi
    
    # 查找可执行文件
    local binary_file=""
    if [[ "$platform" == windows-* ]]; then
        # Windows 平台查找 .exe 文件
        binary_file=$(find . -name "*.exe" -type f 2>/dev/null | head -1)
    else
        # Unix 平台查找可执行文件（可能有不同的命名）
        binary_file=$(find . -type f -perm +111 ! -name "*.tar.gz" ! -name "*.zip" 2>/dev/null | head -1)
        
        # 如果没找到，尝试按名称查找
        if [ -z "$binary_file" ]; then
            binary_file=$(find . -name "${BINARY_NAME}*" -type f ! -name "*.tar.gz" ! -name "*.zip" 2>/dev/null | head -1)
        fi
    fi
    
    if [ -z "$binary_file" ]; then
        print_error "在压缩包中找不到可执行文件"
        exit 1
    fi
    
    # 安装到目标目录
    print_info "安装到: $install_dir"
    
    # 确保有执行权限
    chmod +x "$binary_file"
    
    # 移动文件到安装目录
    if [[ "$platform" == windows-* ]]; then
        mv "$binary_file" "$install_dir/${BINARY_NAME}.exe"
        local installed_file="$install_dir/${BINARY_NAME}.exe"
    else
        mv "$binary_file" "$install_dir/${BINARY_NAME}"
        local installed_file="$install_dir/${BINARY_NAME}"
    fi
    
    print_success "安装成功: $installed_file"
    
    # 检查 PATH
    check_path "$install_dir"
}

# 检查 PATH 环境变量
check_path() {
    local install_dir=$1
    
    if [[ ":$PATH:" != *":$install_dir:"* ]]; then
        print_info ""
        print_info "注意: $install_dir 不在你的 PATH 中"
        print_info "请将以下内容添加到你的 shell 配置文件中 (~/.bashrc, ~/.zshrc 等):"
        print_info ""
        echo "    export PATH=\"$install_dir:\$PATH\""
        print_info ""
        print_info "然后运行: source ~/.bashrc (或对应的配置文件)"
        print_info ""
        print_info "或者你可以直接运行:"
        echo "    $install_dir/${BINARY_NAME}"
    else
        print_info ""
        print_info "你现在可以运行: ${BINARY_NAME}"
    fi
}

# 主函数
main() {
    echo "======================================"
    echo "  Project Manager CLI 安装脚本"
    echo "======================================"
    echo ""
    
    # 检测平台
    local platform=$(detect_platform)
    print_success "检测到平台: $platform"
    
    # 获取最新版本
    print_info "获取最新版本信息..."
    local version=$(get_latest_version)
    print_success "最新版本: $version"
    
    # 构建下载 URL
    local download_url=$(build_download_url "$version" "$platform")
    
    # 确定安装目录
    local install_dir=$(get_install_dir)
    print_info "安装目录: $install_dir"
    
    # 下载并安装
    download_and_install "$download_url" "$install_dir" "$platform"
    
    echo ""
    print_success "安装完成！"
}

# 运行主函数
main "$@"