mod cli;
mod config;
mod models;
mod scanner;
mod tui;
mod operations;
mod utils;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Commands};
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    // 加载配置
    let config = if let Some(config_path) = cli.config {
        Config::load_from_file(&config_path)?
    } else {
        Config::load_or_create_default()?
    };
    
    // 根据命令执行相应操作
    match cli.command {
        Commands::Scan { paths, depth, format, output } => {
            println!("扫描功能待实现");
            println!("扫描路径: {:?}", paths);
            if let Some(d) = depth {
                println!("最大深度: {}", d);
            }
        }
        Commands::Tui { paths } => {
            println!("TUI 功能待实现");
            println!("初始路径: {:?}", paths);
        }
        Commands::Clean { project_path, clean_type, force } => {
            println!("清理功能待实现");
            println!("项目路径: {}", project_path);
            println!("强制执行: {}", force);
        }
        Commands::Delete { project_path, force } => {
            println!("删除功能待实现");
            println!("项目路径: {}", project_path);
            println!("强制执行: {}", force);
        }
        Commands::Config { action } => {
            println!("配置管理功能待实现");
        }
        Commands::Stats { paths, detailed } => {
            println!("统计功能待实现");
            println!("分析路径: {:?}", paths);
            println!("详细统计: {}", detailed);
        }
    }
    
    Ok(())
}
