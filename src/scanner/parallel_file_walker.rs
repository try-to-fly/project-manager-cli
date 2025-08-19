use std::path::{Path, PathBuf};
use std::collections::HashSet;
use anyhow::Result;
use tokio::sync::mpsc;
// use tokio_stream::wrappers::ReceiverStream;  // 暂时未使用
// use futures::stream::StreamExt;  // 暂时未使用
use rayon::prelude::*;
use walkdir::WalkDir;

/// 并发文件系统扫描器 - 充分利用 Rust 的并发能力
pub struct ParallelFileWalker {
    /// 需要忽略的目录
    ignore_dirs: HashSet<String>,
    
    /// 需要忽略的文件扩展名
    ignore_extensions: HashSet<String>,
    
    /// 最大并发任务数
    max_concurrent_tasks: usize,
    
    /// 工作队列大小
    queue_size: usize,
}

/// 文件信息结构
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub is_code_file: bool,
    pub is_dependency_file: bool,
}

/// 目录信息结构
#[derive(Debug, Clone)]
pub struct DirectoryInfo {
    pub path: PathBuf,
    pub is_dependency: bool,
}

/// 扫描进度信息
#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub processed_files: usize,
    pub total_estimated: Option<usize>,
    pub current_path: PathBuf,
    pub bytes_processed: u64,
    pub stage: ScanStage,
}

/// 扫描阶段
#[derive(Debug, Clone, PartialEq)]
pub enum ScanStage {
    Discovery,    // 发现文件阶段
    Metadata,     // 获取元数据阶段
    Calculation,  // 大小计算阶段
    Completed,    // 完成阶段
}

impl ParallelFileWalker {
    /// 创建新的并发文件扫描器
    pub fn new() -> Self {
        Self {
            ignore_dirs: Self::default_ignore_dirs(),
            ignore_extensions: Self::default_ignore_extensions(),
            max_concurrent_tasks: num_cpus::get().max(4), // 至少4个并发任务
            queue_size: 1000,
        }
    }
    
    /// 使用自定义配置创建扫描器
    pub fn with_config(
        ignore_dirs: HashSet<String>,
        ignore_extensions: HashSet<String>,
        max_concurrent_tasks: usize,
    ) -> Self {
        Self {
            ignore_dirs,
            ignore_extensions,
            max_concurrent_tasks,
            queue_size: 1000,
        }
    }
    
    /// 并发扫描指定路径的所有文件
    pub async fn scan_parallel<F>(
        &self, 
        root_path: &Path, 
        progress_callback: F
    ) -> Result<Vec<FileInfo>>
    where
        F: Fn(ScanProgress) + Send + Sync + 'static,
    {
        let progress_callback = std::sync::Arc::new(progress_callback);
        
        // 阶段1：快速发现所有文件路径
        progress_callback(ScanProgress {
            processed_files: 0,
            total_estimated: None,
            current_path: root_path.to_path_buf(),
            bytes_processed: 0,
            stage: ScanStage::Discovery,
        });
        
        let file_paths = self.discover_files_fast(root_path).await?;
        let total_files = file_paths.len();
        
        // 阶段2：并发获取文件元数据
        progress_callback(ScanProgress {
            processed_files: 0,
            total_estimated: Some(total_files),
            current_path: root_path.to_path_buf(),
            bytes_processed: 0,
            stage: ScanStage::Metadata,
        });
        
        let file_infos = self.process_files_parallel(file_paths, progress_callback.clone()).await?;
        
        // 阶段3：完成
        progress_callback(ScanProgress {
            processed_files: total_files,
            total_estimated: Some(total_files),
            current_path: root_path.to_path_buf(),
            bytes_processed: file_infos.iter().map(|f| f.size).sum(),
            stage: ScanStage::Completed,
        });
        
        Ok(file_infos)
    }
    
    /// 快速发现所有文件路径（使用 walkdir + rayon 并行化）
    async fn discover_files_fast(&self, root_path: &Path) -> Result<Vec<PathBuf>> {
        let root_path = root_path.to_path_buf();
        let ignore_dirs = self.ignore_dirs.clone();
        
        // 使用 tokio::task::spawn_blocking 在线程池中运行 CPU 密集型任务
        let file_paths = tokio::task::spawn_blocking(move || {
            WalkDir::new(&root_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .collect::<Vec<_>>()
                .into_par_iter() // 并行化
                .filter_map(|entry| {
                    let path = entry.path();
                    
                    // 跳过目录和应该忽略的文件
                    if path.is_dir() {
                        return None;
                    }
                    
                    // 检查是否在忽略的目录中
                    if path.ancestors().any(|ancestor| {
                        if let Some(name) = ancestor.file_name() {
                            if let Some(name_str) = name.to_str() {
                                return ignore_dirs.contains(name_str);
                            }
                        }
                        false
                    }) {
                        return None;
                    }
                    
                    Some(path.to_path_buf())
                })
                .collect()
        }).await?;
        
        Ok(file_paths)
    }
    
    /// 并发处理文件元数据获取
    async fn process_files_parallel<F>(
        &self,
        file_paths: Vec<PathBuf>,
        progress_callback: std::sync::Arc<F>
    ) -> Result<Vec<FileInfo>>
    where
        F: Fn(ScanProgress) + Send + Sync + 'static,
    {
        let (tx, mut rx) = mpsc::channel(self.queue_size);
        let total_files = file_paths.len();
        
        // 创建工作任务
        let chunk_size = (file_paths.len() / self.max_concurrent_tasks).max(1);
        let tasks: Vec<_> = file_paths
            .chunks(chunk_size)
            .map(|chunk| {
                let chunk = chunk.to_vec();
                let tx = tx.clone();
                let ignore_extensions = self.ignore_extensions.clone();
                let progress_callback = progress_callback.clone();
                
                tokio::spawn(async move {
                    for (i, path) in chunk.iter().enumerate() {
                        match tokio::fs::metadata(path).await {
                            Ok(metadata) if metadata.is_file() => {
                                let file_info = FileInfo {
                                    path: path.clone(),
                                    size: metadata.len(),
                                    is_code_file: Self::is_code_file(path, &ignore_extensions),
                                    is_dependency_file: Self::is_dependency_file(path),
                                };
                                
                                if tx.send(file_info).await.is_err() {
                                    break; // 接收器已关闭
                                }
                            }
                            _ => continue, // 跳过无法访问或非文件的项目
                        }
                        
                        // 每处理10个文件报告一次进度
                        if i % 10 == 0 {
                            progress_callback(ScanProgress {
                                processed_files: i + 1,
                                total_estimated: Some(total_files),
                                current_path: path.clone(),
                                bytes_processed: 0, // 将在最后统计
                                stage: ScanStage::Metadata,
                            });
                        }
                    }
                })
            })
            .collect();
        
        // 关闭发送端，这样接收端知道何时结束
        drop(tx);
        
        // 收集所有结果
        let mut results = Vec::new();
        while let Some(file_info) = rx.recv().await {
            results.push(file_info);
        }
        
        // 等待所有任务完成
        for task in tasks {
            let _ = task.await;
        }
        
        Ok(results)
    }
    
    /// 判断是否为代码文件
    fn is_code_file(path: &Path, ignore_extensions: &HashSet<String>) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                let ext_lower = ext_str.to_lowercase();
                
                // 常见代码文件扩展名
                let code_extensions = ["rs", "js", "ts", "py", "java", "cpp", "c", "h", 
                                     "go", "php", "rb", "swift", "kt", "scala", "cs", 
                                     "vue", "jsx", "tsx", "html", "css", "scss", "less"];
                
                return code_extensions.contains(&ext_lower.as_str()) 
                    && !ignore_extensions.contains(&ext_lower);
            }
        }
        false
    }
    
    /// 判断是否为依赖文件
    fn is_dependency_file(path: &Path) -> bool {
        // 检查文件路径是否包含依赖目录
        path.ancestors().any(|ancestor| {
            if let Some(name) = ancestor.file_name() {
                if let Some(name_str) = name.to_str() {
                    matches!(name_str, 
                        "node_modules" | "target" | "build" | "dist" | "out" | 
                        "bin" | "obj" | "__pycache__" | "venv" | "env" | 
                        ".venv" | ".env" | "site-packages" | "vendor" | "bower_components"
                    )
                } else {
                    false
                }
            } else {
                false
            }
        })
    }
    
    /// 默认忽略的目录
    fn default_ignore_dirs() -> HashSet<String> {
        [
            ".git", ".svn", ".hg", ".bzr",
            "node_modules", "target", "build", "dist", "out",
            "bin", "obj", "__pycache__", "venv", "env",
            ".venv", ".env", "site-packages", "vendor",
            "bower_components", ".idea", ".vscode", ".vs",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
    
    /// 默认忽略的文件扩展名
    fn default_ignore_extensions() -> HashSet<String> {
        [
            "exe", "dll", "so", "dylib", "a", "lib", "o", "obj",
            "zip", "tar", "gz", "7z", "rar", "bz2",
            "jpg", "jpeg", "png", "gif", "bmp", "svg", "ico",
            "mp3", "mp4", "avi", "mov", "wmv", "mkv",
            "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
}

/// 大小计算结果
#[derive(Debug, Clone)]
pub struct SizeCalculationResult {
    pub code_size: u64,
    pub dependency_size: u64,
    pub total_size: u64,
    pub code_file_count: usize,
    pub dependency_file_count: usize,
    pub total_file_count: usize,
}

impl SizeCalculationResult {
    /// 从文件信息列表计算大小统计
    pub fn from_file_infos(file_infos: &[FileInfo]) -> Self {
        let (code_files, _dependency_files): (Vec<_>, Vec<_>) = 
            file_infos.iter().partition(|f| f.is_code_file && !f.is_dependency_file);
        
        let dependency_only_files: Vec<_> = 
            file_infos.iter().filter(|f| f.is_dependency_file).collect();
        
        let code_size = code_files.iter().map(|f| f.size).sum();
        let dependency_size = dependency_only_files.iter().map(|f| f.size).sum();
        
        Self {
            code_size,
            dependency_size,
            total_size: file_infos.iter().map(|f| f.size).sum(),
            code_file_count: code_files.len(),
            dependency_file_count: dependency_only_files.len(),
            total_file_count: file_infos.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;
    
    #[tokio::test]
    async fn test_parallel_scanning() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();
        
        // 创建测试文件结构
        fs::create_dir(root_path.join("src")).await.unwrap();
        fs::write(root_path.join("src/main.rs"), "fn main() {}").await.unwrap();
        fs::write(root_path.join("src/lib.rs"), "pub mod test;").await.unwrap();
        
        fs::create_dir(root_path.join("target")).await.unwrap();
        fs::write(root_path.join("target/output.exe"), "binary data").await.unwrap();
        
        let scanner = ParallelFileWalker::new();
        let progress_reports = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        
        let progress_callback = {
            let reports = progress_reports.clone();
            move |progress: ScanProgress| {
                reports.lock().unwrap().push(progress);
            }
        };
        
        let results = scanner.scan_parallel(root_path, progress_callback).await.unwrap();
        
        // 验证结果
        assert!(!results.is_empty());
        assert!(results.iter().any(|f| f.path.to_string_lossy().contains("main.rs")));
        assert!(results.iter().any(|f| f.is_code_file));
        
        // 验证进度报告
        let reports = progress_reports.lock().unwrap();
        assert!(!reports.is_empty());
        assert!(reports.iter().any(|r| r.stage == ScanStage::Discovery));
        assert!(reports.iter().any(|r| r.stage == ScanStage::Completed));
    }
}