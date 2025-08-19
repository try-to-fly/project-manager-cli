use project_manager_cli::scanner::{SizeCalculator, CacheConfig};
use std::time::{Duration, Instant};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let project_path = std::env::current_dir()?;
    
    println!("🚀 项目大小计算优化演示");
    println!("项目路径: {}", project_path.display());
    println!();
    
    // 演示1：不使用缓存的计算
    println!("📊 演示 1: 不使用缓存");
    let mut calculator_no_cache = SizeCalculator::new();
    
    let start = Instant::now();
    let size_info1 = calculator_no_cache.calculate_project_size(&project_path).await?;
    let duration1 = start.elapsed();
    
    println!("计算用时: {:?}", duration1);
    println!("代码大小: {} bytes ({:.2} MB)", size_info1.code_size, size_info1.code_size as f64 / (1024.0 * 1024.0));
    println!("依赖大小: {} bytes ({:.2} MB)", size_info1.dependency_size, size_info1.dependency_size as f64 / (1024.0 * 1024.0));
    println!("gitignore 排除大小: {} bytes ({:.2} MB)", size_info1.gitignore_excluded_size, size_info1.gitignore_excluded_size as f64 / (1024.0 * 1024.0));
    println!("总大小: {} bytes ({:.2} MB)", size_info1.total_size, size_info1.total_size as f64 / (1024.0 * 1024.0));
    println!("文件数量: {} 个", size_info1.total_file_count);
    println!();
    
    // 演示2：使用缓存的计算
    println!("💾 演示 2: 使用缓存");
    
    let cache_config = CacheConfig {
        enabled: true,
        expiry_duration: Duration::from_secs(60 * 60), // 1小时
        max_entries: 100,
    };
    
    let mut calculator_with_cache = SizeCalculator::new_with_cache(cache_config).await?;
    
    // 第一次计算（会缓存结果）
    println!("第一次计算 (缓存MISS):");
    let start = Instant::now();
    let size_info2 = calculator_with_cache.calculate_project_size(&project_path).await?;
    let duration2 = start.elapsed();
    
    println!("计算用时: {:?}", duration2);
    println!("代码大小: {} bytes ({:.2} MB)", size_info2.code_size, size_info2.code_size as f64 / (1024.0 * 1024.0));
    println!("依赖大小: {} bytes ({:.2} MB)", size_info2.dependency_size, size_info2.dependency_size as f64 / (1024.0 * 1024.0));
    println!("gitignore 排除大小: {} bytes ({:.2} MB)", size_info2.gitignore_excluded_size, size_info2.gitignore_excluded_size as f64 / (1024.0 * 1024.0));
    println!("总大小: {} bytes ({:.2} MB)", size_info2.total_size, size_info2.total_size as f64 / (1024.0 * 1024.0));
    println!();
    
    // 第二次计算（从缓存获取）
    println!("第二次计算 (缓存HIT):");
    let start = Instant::now();
    let size_info3 = calculator_with_cache.calculate_project_size(&project_path).await?;
    let duration3 = start.elapsed();
    
    println!("计算用时: {:?}", duration3);
    println!("代码大小: {} bytes ({:.2} MB)", size_info3.code_size, size_info3.code_size as f64 / (1024.0 * 1024.0));
    println!("依赖大小: {} bytes ({:.2} MB)", size_info3.dependency_size, size_info3.dependency_size as f64 / (1024.0 * 1024.0));
    println!("gitignore 排除大小: {} bytes ({:.2} MB)", size_info3.gitignore_excluded_size, size_info3.gitignore_excluded_size as f64 / (1024.0 * 1024.0));
    println!("总大小: {} bytes ({:.2} MB)", size_info3.total_size, size_info3.total_size as f64 / (1024.0 * 1024.0));
    println!();
    
    // 验证结果一致性
    assert_eq!(size_info2.code_size, size_info3.code_size);
    assert_eq!(size_info2.total_size, size_info3.total_size);
    println!("✅ 缓存结果验证通过");
    
    // 性能提升计算
    let speedup = duration2.as_millis() as f64 / duration3.as_millis() as f64;
    println!("⚡ 性能提升: {:.1}x 倍", speedup);
    
    // 缓存统计
    if let Some(stats) = calculator_with_cache.get_cache_stats() {
        println!();
        println!("📈 缓存统计:");
        println!("  - 缓存条目数: {}", stats.total_entries);
        println!("  - 过期条目数: {}", stats.expired_entries);
        println!("  - Git 仓库数: {}", stats.git_repositories);
        println!("  - 缓存的总代码大小: {:.2} MB", stats.total_code_size as f64 / (1024.0 * 1024.0));
        println!("  - 缓存的总依赖大小: {:.2} MB", stats.total_dependency_size as f64 / (1024.0 * 1024.0));
        println!("  - 缓存的 gitignore 排除大小: {:.2} MB", stats.total_gitignore_size as f64 / (1024.0 * 1024.0));
        println!("  - 缓存文件大小: {} bytes", stats.cache_file_size);
    }
    
    println!();
    println!("🎉 演示完成！");
    
    Ok(())
}