use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "project-manager-cli")]
#[command(about = "一个用于扫描和管理代码项目的 CLI 工具")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    /// 初始扫描路径 (默认启动 TUI 模式时使用)
    #[arg(default_value = ".")]
    pub paths: Vec<String>,
    
    /// 配置文件路径
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    
    /// 详细输出
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 扫描指定目录中的项目
    Scan {
        /// 要扫描的目录路径
        #[arg(default_value = ".")]
        paths: Vec<String>,
        
        /// 最大扫描深度
        #[arg(short, long)]
        depth: Option<usize>,
        
        /// 输出格式
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Table)]
        format: OutputFormat,
        
        /// 保存结果到文件
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// 启动交互式 TUI 界面
    Tui {
        /// 初始扫描路径
        #[arg(default_value = ".")]
        paths: Vec<String>,
    },
    
    /// 清理项目依赖
    Clean {
        /// 项目路径
        project_path: String,
        
        /// 清理类型
        #[arg(short, long, value_enum, default_value_t = CleanType::Dependencies)]
        clean_type: CleanType,
        
        /// 强制删除，不询问确认
        #[arg(short, long)]
        force: bool,
    },
    
    /// 删除项目到回收站
    Delete {
        /// 项目路径
        project_path: String,
        
        /// 强制删除，不询问确认
        #[arg(short, long)]
        force: bool,
    },
    
    /// 管理配置
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    
    /// 项目统计信息
    Stats {
        /// 要分析的目录路径
        #[arg(default_value = ".")]
        paths: Vec<String>,
        
        /// 显示详细统计
        #[arg(short, long)]
        detailed: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// 显示当前配置
    Show,
    
    /// 编辑配置文件
    Edit,
    
    /// 重置为默认配置
    Reset,
    
    /// 添加忽略路径
    Ignore {
        /// 要忽略的路径
        path: String,
    },
    
    /// 移除忽略路径
    Unignore {
        /// 要取消忽略的路径
        path: String,
    },
}

#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    /// 表格格式
    Table,
    /// JSON 格式
    Json,
    /// CSV 格式
    Csv,
}

#[derive(clap::ValueEnum, Clone)]
pub enum CleanType {
    /// 清理依赖文件夹 (node_modules, target 等)
    Dependencies,
    /// 清理缓存文件
    Cache,
    /// 清理所有临时文件
    All,
}