mod cli;
mod config;
mod models;
mod scanner;
mod tui;
mod operations;
mod utils;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Commands, ConfigAction};
use config::Config;
use tui::app::App;
use std::process::Command;

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
        Some(Commands::Scan { paths, depth, format: _, output: _ }) => {
            println!("扫描功能待实现");
            println!("扫描路径: {:?}", paths);
            if let Some(d) = depth {
                println!("最大深度: {}", d);
            }
        }
        Some(Commands::Tui { paths }) => {
            let scan_paths = if paths.is_empty() {
                vec![std::env::current_dir()?.display().to_string()]
            } else {
                paths
            };
            
            let mut app = App::new(config, scan_paths);
            app.run().await?;
        }
        Some(Commands::Clean { project_path, clean_type: _, force }) => {
            println!("清理功能待实现");
            println!("项目路径: {}", project_path);
            println!("强制执行: {}", force);
        }
        Some(Commands::Delete { project_path, force }) => {
            println!("删除功能待实现");
            println!("项目路径: {}", project_path);
            println!("强制执行: {}", force);
        }
        Some(Commands::Config { action }) => {
            handle_config_command(action, &config).await?;
        }
        Some(Commands::Stats { paths, detailed }) => {
            println!("统计功能待实现");
            println!("分析路径: {:?}", paths);
            println!("详细统计: {}", detailed);
        }
        None => {
            // 默认启动 TUI 模式
            let scan_paths = if cli.paths.is_empty() {
                vec![std::env::current_dir()?.display().to_string()]
            } else {
                cli.paths
            };
            
            let mut app = App::new(config, scan_paths);
            app.run().await?;
        }
    }
    
    Ok(())
}

/// 处理配置相关命令
async fn handle_config_command(action: ConfigAction, config: &Config) -> Result<()> {
    match action {
        ConfigAction::Show => {
            show_config(config)?;
        }
        ConfigAction::Edit => {
            edit_config().await?;
        }
        ConfigAction::Reset => {
            reset_config().await?;
        }
        ConfigAction::Ignore { path } => {
            add_ignore_path(path).await?;
        }
        ConfigAction::Unignore { path } => {
            remove_ignore_path(path).await?;
        }
    }
    Ok(())
}

/// 显示当前配置
fn show_config(config: &Config) -> Result<()> {
    println!("📋 项目管理器配置信息");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 扫描路径
    println!("\n🔍 扫描路径:");
    for (i, path) in config.scan_paths.iter().enumerate() {
        println!("  {}. {}", i + 1, path);
    }
    
    // 忽略配置
    println!("\n🚫 忽略配置:");
    
    // 忽略的路径
    println!("  路径 ({} 个):", config.ignore.paths.len());
    if config.ignore.paths.is_empty() {
        println!("    (无)");
    } else {
        for path in &config.ignore.paths {
            println!("    • {}", path);
        }
    }
    
    // 忽略的目录名
    println!("  目录名 ({} 个):", config.ignore.directories.len());
    let mut dirs: Vec<_> = config.ignore.directories.iter().collect();
    dirs.sort();
    for dir in dirs.chunks(5) {
        print!("    ");
        for (i, d) in dir.iter().enumerate() {
            if i > 0 { print!(", "); }
            print!("{}", d);
        }
        println!();
    }
    
    // 忽略的扩展名
    if !config.ignore.extensions.is_empty() {
        println!("  文件扩展名 ({} 个):", config.ignore.extensions.len());
        let mut exts: Vec<_> = config.ignore.extensions.iter().collect();
        exts.sort();
        print!("    ");
        for (i, ext) in exts.iter().enumerate() {
            if i > 0 { print!(", "); }
            print!(".{}", ext);
        }
        println!();
    }
    
    // 扫描配置
    println!("\n⚙️  扫描配置:");
    println!("  最大深度: {}", config.scan.max_depth.map_or("无限制".to_string(), |d| d.to_string()));
    println!("  跟随符号链接: {}", if config.scan.follow_symlinks { "是" } else { "否" });
    println!("  并发扫描数: {}", config.scan.concurrent_scans);
    println!("  扫描隐藏目录: {}", if config.scan.scan_hidden { "是" } else { "否" });
    
    // 缓存配置
    println!("\n💾 缓存配置:");
    println!("  启用缓存: {}", if config.cache.enabled { "是" } else { "否" });
    println!("  过期时间: {} 小时", config.cache.expiry_duration.as_secs() / 3600);
    println!("  最大条目数: {}", config.cache.max_entries);
    
    println!("\n📁 配置文件位置:");
    if let Ok(config_path) = Config::default_config_path() {
        println!("  {}", config_path.display());
    }
    
    Ok(())
}

/// 编辑配置文件
async fn edit_config() -> Result<()> {
    let config_path = Config::default_config_path()?;
    
    println!("📝 打开配置文件进行编辑...");
    println!("文件路径: {}", config_path.display());
    
    // 尝试不同的编辑器
    let editors = ["nvim", "vim", "nano", "code", "subl"];
    let mut editor_found = false;
    
    for editor in &editors {
        if Command::new("which").arg(editor).output().is_ok() {
            let status = Command::new(editor)
                .arg(&config_path)
                .status()?;
            
            if status.success() {
                println!("✅ 配置文件编辑完成");
            } else {
                println!("⚠️  编辑器异常退出");
            }
            editor_found = true;
            break;
        }
    }
    
    if !editor_found {
        println!("❌ 未找到可用的编辑器");
        println!("请手动编辑配置文件: {}", config_path.display());
    }
    
    Ok(())
}

/// 重置配置为默认值
async fn reset_config() -> Result<()> {
    println!("⚠️  即将重置配置为默认值");
    print!("确认要继续吗？ (y/N): ");
    
    use std::io::{self, Write};
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
        let config_path = Config::default_config_path()?;
        let default_config = Config::default();
        default_config.save_to_file(&config_path)?;
        
        println!("✅ 配置已重置为默认值");
        println!("配置文件: {}", config_path.display());
    } else {
        println!("❌ 操作已取消");
    }
    
    Ok(())
}

/// 添加忽略路径
async fn add_ignore_path(path: String) -> Result<()> {
    let config_path = Config::default_config_path()?;
    let mut config = Config::load_or_create_default()?;
    
    // 规范化路径
    let normalized_path = normalize_path(&path)?;
    
    // 检查路径是否已存在
    if config.ignore.paths.contains(&normalized_path) {
        println!("⚠️  路径已在忽略列表中: {}", normalized_path);
        return Ok(());
    }
    
    // 添加到忽略列表
    config.ignore.paths.insert(normalized_path.clone());
    
    // 保存配置
    config.save_to_file(&config_path)?;
    
    println!("✅ 已添加到忽略列表: {}", normalized_path);
    println!("💾 配置已保存");
    
    Ok(())
}

/// 移除忽略路径
async fn remove_ignore_path(path: String) -> Result<()> {
    let config_path = Config::default_config_path()?;
    let mut config = Config::load_or_create_default()?;
    
    // 规范化路径
    let normalized_path = normalize_path(&path)?;
    
    // 尝试移除路径
    if config.ignore.paths.remove(&normalized_path) {
        // 保存配置
        config.save_to_file(&config_path)?;
        println!("✅ 已从忽略列表移除: {}", normalized_path);
        println!("💾 配置已保存");
    } else {
        // 尝试模糊匹配
        let matches: Vec<_> = config.ignore.paths.iter()
            .filter(|p| p.contains(&normalized_path) || normalized_path.contains(p.as_str()))
            .cloned()
            .collect();
        
        if matches.is_empty() {
            println!("❌ 路径不在忽略列表中: {}", normalized_path);
        } else if matches.len() == 1 {
            let matched_path = matches[0].clone();
            config.ignore.paths.remove(&matched_path);
            config.save_to_file(&config_path)?;
            println!("✅ 已从忽略列表移除: {}", matched_path);
            println!("💾 配置已保存");
        } else {
            println!("❓ 找到多个匹配项:");
            for (i, matched) in matches.iter().enumerate() {
                println!("  {}. {}", i + 1, matched);
            }
            println!("请指定更精确的路径");
        }
    }
    
    Ok(())
}

/// 规范化路径
fn normalize_path(path: &str) -> Result<String> {
    use std::path::Path;
    
    let path = if path.starts_with('~') {
        // 展开 ~ 为用户主目录
        if let Some(home) = dirs::home_dir() {
            let without_tilde = &path[1..];
            if without_tilde.is_empty() || without_tilde.starts_with('/') {
                home.join(&without_tilde[1..]).to_string_lossy().to_string()
            } else {
                return Err(anyhow::anyhow!("无效的路径格式: {}", path));
            }
        } else {
            return Err(anyhow::anyhow!("无法获取用户主目录"));
        }
    } else {
        path.to_string()
    };
    
    // 转换为绝对路径
    let path = Path::new(&path);
    let canonical = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    
    Ok(canonical.to_string_lossy().to_string())
}
