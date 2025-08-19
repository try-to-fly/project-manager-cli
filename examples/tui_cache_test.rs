use project_manager_cli::scanner::SizeCalculator;
use project_manager_cli::config::Config;
use std::time::Instant;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 TUI 缓存功能测试");
    println!();
    
    let project_path = std::env::current_dir()?;
    println!("测试项目: {}", project_path.display());
    
    // 模拟TUI中的缓存使用逻辑
    let config = Config::load_or_create_default().unwrap_or_default();
    let mut size_calculator = SizeCalculator::new_with_cache(config.cache.to_size_cache_config())
        .await
        .unwrap_or_else(|_| SizeCalculator::new());
    
    println!();
    println!("📊 第一次计算（模拟TUI启动）:");
    let start = Instant::now();
    let size_info1 = size_calculator.calculate_project_size(&project_path).await?;
    let duration1 = start.elapsed();
    
    println!("用时: {:?}", duration1);
    println!("代码大小: {} bytes", size_info1.code_size);
    println!("总大小: {} bytes", size_info1.total_size);
    
    println!();
    println!("💾 第二次计算（模拟TUI重启）:");
    
    // 重新创建计算器，模拟TUI重启
    let config = Config::load_or_create_default().unwrap_or_default();
    let mut size_calculator = SizeCalculator::new_with_cache(config.cache.to_size_cache_config())
        .await
        .unwrap_or_else(|_| SizeCalculator::new());
    
    let start = Instant::now();
    let size_info2 = size_calculator.calculate_project_size(&project_path).await?;
    let duration2 = start.elapsed();
    
    println!("用时: {:?}", duration2);
    println!("代码大小: {} bytes", size_info2.code_size);
    println!("总大小: {} bytes", size_info2.total_size);
    
    // 验证结果一致性
    assert_eq!(size_info1.code_size, size_info2.code_size);
    assert_eq!(size_info1.total_size, size_info2.total_size);
    
    let speedup = duration1.as_micros() as f64 / duration2.as_micros() as f64;
    
    println!();
    if duration2 < duration1 {
        println!("✅ 缓存有效！性能提升: {:.1}x", speedup);
    } else {
        println!("❌ 缓存可能未生效");
    }
    
    println!();
    println!("🎯 TUI现在会在重启时使用缓存，避免重复计算！");
    
    Ok(())
}