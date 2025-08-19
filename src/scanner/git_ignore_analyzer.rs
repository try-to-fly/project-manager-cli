use std::path::{Path, PathBuf};
use std::collections::HashSet;
use anyhow::Result;
use git2::Repository;
use ignore::{WalkBuilder, DirEntry};

/// Git 忽略规则分析器 - 负责解析和应用 .gitignore 规则
pub struct GitIgnoreAnalyzer {
    /// 项目根路径
    root_path: PathBuf,
    
    /// 是否是 Git 项目
    is_git_repo: bool,
    
    /// 忽略的路径集合（用于快速查找）
    ignored_paths: HashSet<PathBuf>,
}

impl GitIgnoreAnalyzer {
    /// 创建新的 Git 忽略分析器
    pub fn new(project_path: &Path) -> Result<Self> {
        let mut analyzer = Self {
            root_path: project_path.to_path_buf(),
            is_git_repo: false,
            ignored_paths: HashSet::new(),
        };
        
        // 检查是否是 Git 项目
        analyzer.is_git_repo = Repository::discover(project_path).is_ok();
        
        if analyzer.is_git_repo {
            analyzer.build_ignore_set()?;
        }
        
        Ok(analyzer)
    }
    
    /// 检查是否是 Git 项目
    pub fn is_git_repository(&self) -> bool {
        self.is_git_repo
    }
    
    /// 检查路径是否应该被忽略
    pub fn should_ignore(&self, path: &Path) -> bool {
        if !self.is_git_repo {
            return false;
        }
        
        // 将相对路径转换为绝对路径进行比较
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root_path.join(path)
        };
        
        // 检查路径本身或其任何父路径是否在忽略列表中
        let mut current_path = absolute_path.as_path();
        loop {
            if self.ignored_paths.contains(current_path) {
                return true;
            }
            
            match current_path.parent() {
                Some(parent) if parent != self.root_path => {
                    current_path = parent;
                }
                _ => break,
            }
        }
        
        false
    }
    
    /// 获取可以遍历的文件和目录列表
    pub fn get_walkable_entries(&self) -> Result<Vec<PathBuf>> {
        if !self.is_git_repo {
            // 非 Git 项目，返回所有条目
            return self.get_all_entries();
        }
        
        let mut entries = Vec::new();
        
        let walker = WalkBuilder::new(&self.root_path)
            .git_ignore(true)
            .git_exclude(true)
            .git_global(true)
            .hidden(false) // 包含隐藏文件，但排除 .git 目录
            .build();
        
        for result in walker {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    
                    // 手动排除 .git 目录
                    if path.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name == ".git")
                        .unwrap_or(false) {
                        continue;
                    }
                    
                    entries.push(path.to_path_buf());
                }
                Err(_) => continue,
            }
        }
        
        Ok(entries)
    }
    
    /// 构建忽略路径集合
    fn build_ignore_set(&mut self) -> Result<()> {
        let walker = WalkBuilder::new(&self.root_path)
            .git_ignore(true)
            .git_exclude(true)
            .git_global(true)
            .hidden(false)
            .build();
        
        // 首先获取所有被忽略的路径
        let mut all_paths = HashSet::new();
        let walker_all = WalkBuilder::new(&self.root_path)
            .git_ignore(false)
            .git_exclude(false)
            .git_global(false)
            .hidden(false)
            .build();
        
        for result in walker_all {
            if let Ok(entry) = result {
                all_paths.insert(entry.path().to_path_buf());
            }
        }
        
        // 然后获取不被忽略的路径
        let mut not_ignored = HashSet::new();
        for result in walker {
            if let Ok(entry) = result {
                let path = entry.path();
                
                // 手动排除 .git 目录
                if path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name == ".git")
                    .unwrap_or(false) {
                    continue;
                }
                
                not_ignored.insert(path.to_path_buf());
            }
        }
        
        // 计算被忽略的路径（差集）
        for path in all_paths {
            if !not_ignored.contains(&path) {
                self.ignored_paths.insert(path);
            }
        }
        
        // 手动添加 .git 目录
        self.ignored_paths.insert(self.root_path.join(".git"));
        
        Ok(())
    }
    
    /// 获取所有条目（非 Git 项目使用）
    fn get_all_entries(&self) -> Result<Vec<PathBuf>> {
        use std::fs;
        
        fn collect_entries(dir: &Path, entries: &mut Vec<PathBuf>) -> Result<()> {
            let read_dir = fs::read_dir(dir)?;
            
            for entry in read_dir {
                let entry = entry?;
                let path = entry.path();
                
                entries.push(path.clone());
                
                if path.is_dir() {
                    collect_entries(&path, entries)?;
                }
            }
            
            Ok(())
        }
        
        let mut entries = Vec::new();
        collect_entries(&self.root_path, &mut entries)?;
        
        Ok(entries)
    }
    
    /// 获取被忽略的路径统计
    pub fn get_ignore_stats(&self) -> IgnoreStats {
        IgnoreStats {
            total_ignored_paths: self.ignored_paths.len(),
            is_git_repo: self.is_git_repo,
            ignored_files_size: 0, // 需要调用 calculate_ignored_files_size 来获取准确值
            ignored_files_count: 0, // 需要调用 calculate_ignored_files_size 来获取准确值
        }
    }
    
    /// 计算被忽略的文件总大小和数量
    pub async fn calculate_ignored_files_size(&self) -> Result<(u64, usize)> {
        if !self.is_git_repo {
            return Ok((0, 0));
        }
        
        self.calculate_ignored_files_size_exclude_dependencies(&[]).await
    }
    
    /// 计算被忽略的文件总大小和数量，排除指定的依赖目录（避免重复计算）
    pub async fn calculate_ignored_files_size_exclude_dependencies(&self, exclude_dirs: &[&str]) -> Result<(u64, usize)> {
        if !self.is_git_repo {
            return Ok((0, 0));
        }
        
        use tokio::fs;
        
        let mut total_size = 0u64;
        let mut file_count = 0usize;
        
        for ignored_path in &self.ignored_paths {
            if ignored_path.exists() {
                // 检查是否是需要排除的依赖目录
                if ignored_path.is_dir() {
                    if let Some(dir_name) = ignored_path.file_name().and_then(|n| n.to_str()) {
                        if exclude_dirs.contains(&dir_name) {
                            continue; // 跳过依赖目录，避免重复计算
                        }
                    }
                }
                
                if ignored_path.is_file() {
                    match fs::metadata(ignored_path).await {
                        Ok(metadata) => {
                            total_size += metadata.len();
                            file_count += 1;
                        }
                        Err(_) => continue,
                    }
                } else if ignored_path.is_dir() {
                    let (dir_size, dir_file_count) = self.calculate_directory_size(ignored_path).await?;
                    total_size += dir_size;
                    file_count += dir_file_count;
                }
            }
        }
        
        Ok((total_size, file_count))
    }
    
    /// 递归计算目录大小
    fn calculate_directory_size<'a>(&'a self, dir_path: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(u64, usize)>> + Send + 'a>> {
        Box::pin(async move {
            use tokio::fs;
            
            let mut total_size = 0u64;
            let mut file_count = 0usize;
            
            let mut entries = match fs::read_dir(dir_path).await {
                Ok(entries) => entries,
                Err(_) => return Ok((0, 0)), // 无法访问的目录
            };
            
            loop {
                let entry = match entries.next_entry().await {
                    Ok(Some(entry)) => entry,
                    Ok(None) => break,
                    Err(_) => continue,
                };
                let path = entry.path();
                
                match entry.metadata().await {
                    Ok(metadata) => {
                        if metadata.is_file() {
                            total_size += metadata.len();
                            file_count += 1;
                        } else if metadata.is_dir() {
                            let (sub_size, sub_count) = self.calculate_directory_size(&path).await?;
                            total_size += sub_size;
                            file_count += sub_count;
                        }
                    }
                    Err(_) => continue,
                }
            }
            
            Ok((total_size, file_count))
        })
    }
    
    /// 获取详细的忽略统计信息（包含文件大小）
    pub async fn get_detailed_ignore_stats(&self) -> Result<IgnoreStats> {
        let (ignored_files_size, ignored_files_count) = self.calculate_ignored_files_size().await?;
        
        Ok(IgnoreStats {
            total_ignored_paths: self.ignored_paths.len(),
            is_git_repo: self.is_git_repo,
            ignored_files_size,
            ignored_files_count,
        })
    }
}

/// 忽略统计信息
#[derive(Debug, Clone)]
pub struct IgnoreStats {
    /// 被忽略的路径总数
    pub total_ignored_paths: usize,
    
    /// 是否是 Git 仓库
    pub is_git_repo: bool,
    
    /// 被忽略的文件总大小（字节）
    pub ignored_files_size: u64,
    
    /// 被忽略的文件数量
    pub ignored_files_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use git2::Repository;

    #[test]
    fn test_non_git_project() {
        let temp_dir = tempdir().unwrap();
        
        let analyzer = GitIgnoreAnalyzer::new(temp_dir.path()).unwrap();
        
        assert!(!analyzer.is_git_repository());
        assert!(!analyzer.should_ignore(temp_dir.path().join("test.txt").as_path()));
    }

    #[test]
    fn test_git_project_basic() {
        let temp_dir = tempdir().unwrap();
        
        // 创建 Git 仓库
        Repository::init(temp_dir.path()).unwrap();
        
        // 创建 .gitignore 文件
        let gitignore_content = "*.log\nnode_modules/\ntarget/";
        fs::write(temp_dir.path().join(".gitignore"), gitignore_content).unwrap();
        
        let analyzer = GitIgnoreAnalyzer::new(temp_dir.path()).unwrap();
        
        assert!(analyzer.is_git_repository());
        
        // .git 目录应该被忽略
        assert!(analyzer.should_ignore(&temp_dir.path().join(".git")));
    }

    #[test]
    fn test_should_ignore_git_directory() {
        let temp_dir = tempdir().unwrap();
        Repository::init(temp_dir.path()).unwrap();
        
        let analyzer = GitIgnoreAnalyzer::new(temp_dir.path()).unwrap();
        
        // .git 目录及其子目录应该被忽略
        assert!(analyzer.should_ignore(&temp_dir.path().join(".git")));
        assert!(analyzer.should_ignore(&temp_dir.path().join(".git").join("objects")));
    }

    #[test]
    fn test_get_ignore_stats() {
        let temp_dir = tempdir().unwrap();
        Repository::init(temp_dir.path()).unwrap();
        
        let analyzer = GitIgnoreAnalyzer::new(temp_dir.path()).unwrap();
        let stats = analyzer.get_ignore_stats();
        
        assert!(stats.is_git_repo);
        assert!(stats.total_ignored_paths > 0); // 至少有 .git 目录
    }
}