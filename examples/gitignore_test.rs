use project_manager_cli::scanner::git_ignore_analyzer::GitIgnoreAnalyzer;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let project_path = std::env::current_dir()?;
    
    println!("🔍 Git 忽略文件大小分析");
    println!("项目路径: {}", project_path.display());
    println!();
    
    let analyzer = GitIgnoreAnalyzer::new(&project_path)?;
    
    if !analyzer.is_git_repository() {
        println!("❌ 这不是一个 Git 项目");
        return Ok(());
    }
    
    println!("✅ 这是一个 Git 项目");
    println!();
    
    // 获取基本统计
    let basic_stats = analyzer.get_ignore_stats();
    println!("📊 基本忽略统计:");
    println!("  - 被忽略的路径数量: {}", basic_stats.total_ignored_paths);
    println!();
    
    // 获取详细统计（包含文件大小）
    println!("📈 详细忽略统计（含文件大小）:");
    let detailed_stats = analyzer.get_detailed_ignore_stats().await?;
    println!("  - 被忽略的路径数量: {}", detailed_stats.total_ignored_paths);
    println!("  - 被忽略的文件数量: {}", detailed_stats.ignored_files_count);
    println!("  - 被忽略的文件总大小: {} bytes ({:.2} MB)", 
             detailed_stats.ignored_files_size,
             detailed_stats.ignored_files_size as f64 / (1024.0 * 1024.0));
    println!();
    
    // 排除依赖目录后的统计（避免与依赖大小重复计算）
    println!("📉 排除依赖目录后的忽略统计:");
    let exclude_deps = ["node_modules", "target", "build", "dist"];
    let (size_without_deps, count_without_deps) = analyzer
        .calculate_ignored_files_size_exclude_dependencies(&exclude_deps).await?;
    
    println!("  - 被忽略的文件数量（排除依赖）: {}", count_without_deps);  
    println!("  - 被忽略的文件大小（排除依赖）: {} bytes ({:.2} MB)", 
             size_without_deps,
             size_without_deps as f64 / (1024.0 * 1024.0));
    
    if detailed_stats.ignored_files_size > size_without_deps {
        println!("  - 依赖目录占用大小: {} bytes ({:.2} MB)",
                 detailed_stats.ignored_files_size - size_without_deps,
                 (detailed_stats.ignored_files_size - size_without_deps) as f64 / (1024.0 * 1024.0));
    }
    
    println!();
    println!("🎉 分析完成！");
    
    Ok(())
}