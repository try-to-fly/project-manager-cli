use std::path::Path;
use tempfile::tempdir;
use std::fs;
use git2::Repository;

use project_manager_cli::scanner::{SizeCalculator, GitIgnoreAnalyzer, SizeCache, CacheConfig};

#[tokio::test]
async fn test_integrated_git_project_size_calculation_with_cache() {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();
    
    // 创建一个 Git 项目
    Repository::init(project_path).unwrap();
    
    // 创建 .gitignore 文件
    let gitignore_content = "target/\n*.log\nnode_modules/";
    fs::write(project_path.join(".gitignore"), gitignore_content).unwrap();
    
    // 创建一些测试文件
    let src_dir = project_path.join("src");
    fs::create_dir(&src_dir).unwrap();
    let main_file = src_dir.join("main.rs");
    fs::write(&main_file, "fn main() { println!(\"Hello, world!\"); }").unwrap();
    
    // 创建 target 目录（应该被 .gitignore 忽略）
    let target_dir = project_path.join("target");
    fs::create_dir(&target_dir).unwrap();
    let target_file = target_dir.join("debug.exe");
    fs::write(&target_file, "binary content".repeat(1000)).unwrap();
    
    // 创建 README.md
    let readme_file = project_path.join("README.md");
    fs::write(&readme_file, "# Test Project\n\nA test project with caching.").unwrap();
    
    // 测试 Git 忽略分析器
    let git_analyzer = GitIgnoreAnalyzer::new(project_path).unwrap();
    assert!(git_analyzer.is_git_repository());
    
    // 测试带缓存的大小计算器
    let cache_config = CacheConfig {
        enabled: true,
        expiry_duration: std::time::Duration::from_secs(60),
        max_entries: 100,
    };
    
    let mut calculator = SizeCalculator::new_with_cache(cache_config).await.unwrap();
    
    // 第一次计算（应该缓存结果）
    let size_info1 = calculator.calculate_project_size(project_path).await.unwrap();
    
    // 验证大小计算结果
    assert!(size_info1.code_size > 0, "代码大小应该大于 0");
    assert!(size_info1.total_size >= size_info1.code_size, "总大小应该大于等于代码大小");
    
    // target 目录应该被忽略，所以代码大小应该小于总文件系统大小
    let all_files_size = get_directory_size(project_path);
    assert!(size_info1.total_size < all_files_size, "计算的大小应该小于实际文件系统大小（因为忽略了 target）");
    
    // 第二次计算（应该从缓存获取）
    let size_info2 = calculator.calculate_project_size(project_path).await.unwrap();
    
    // 验证缓存结果一致
    assert_eq!(size_info1.code_size, size_info2.code_size);
    assert_eq!(size_info1.total_size, size_info2.total_size);
    assert_eq!(size_info1.code_file_count, size_info2.code_file_count);
    
    // 验证缓存状态
    if let Some(status) = calculator.get_cache_status(project_path) {
        use project_manager_cli::scanner::CacheStatus;
        assert_eq!(status, CacheStatus::Valid);
    }
    
    // 验证缓存统计
    if let Some(stats) = calculator.get_cache_stats() {
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.expired_entries, 0);
    }
}

#[tokio::test]
async fn test_non_git_project_size_calculation() {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path();
    
    // 创建一个非 Git 项目
    let src_dir = project_path.join("src");
    fs::create_dir(&src_dir).unwrap();
    let main_file = src_dir.join("index.js");
    fs::write(&main_file, "console.log('Hello, world!');").unwrap();
    
    // 创建 node_modules 目录（应该被默认忽略规则忽略）
    let node_modules = project_path.join("node_modules");
    fs::create_dir(&node_modules).unwrap();
    let package_file = node_modules.join("some-package").join("index.js");
    fs::create_dir(package_file.parent().unwrap()).unwrap();
    fs::write(&package_file, "module.exports = {};".repeat(100)).unwrap();
    
    // 测试 Git 忽略分析器
    let git_analyzer = GitIgnoreAnalyzer::new(project_path).unwrap();
    assert!(!git_analyzer.is_git_repository());
    
    // 测试大小计算器
    let mut calculator = SizeCalculator::new();
    let size_info = calculator.calculate_project_size(project_path).await.unwrap();
    
    // 验证结果
    assert!(size_info.code_size > 0, "代码大小应该大于 0");
    assert!(size_info.dependency_size > 0, "依赖大小应该大于 0（node_modules）");
    assert_eq!(size_info.total_size, size_info.code_size + size_info.dependency_size);
    assert!(size_info.code_file_count > 0);
    assert!(size_info.dependency_file_count > 0);
}

// 辅助函数：递归计算目录大小
fn get_directory_size(dir: &Path) -> u64 {
    let mut total_size = 0;
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path) {
                    total_size += metadata.len();
                }
            } else if path.is_dir() {
                total_size += get_directory_size(&path);
            }
        }
    }
    
    total_size
}