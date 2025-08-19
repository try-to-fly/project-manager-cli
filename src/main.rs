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
use tui::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志，设置日志级别为 INFO
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
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
            let scan_paths = if paths.is_empty() {
                vec![std::env::current_dir()?.display().to_string()]
            } else {
                paths
            };
            
            let mut app = App::new(config, scan_paths);
            app.run().await?;
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
