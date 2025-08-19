use std::path::{Path, PathBuf};
use std::collections::HashSet;
use anyhow::Result;
use tokio::fs;
use std::fs::Metadata;

/// 大小计算器 - 负责计算项目的代码大小和依赖大小
pub struct SizeCalculator {
    /// 需要忽略的目录（通常是依赖目录）
    ignore_dirs: HashSet<String>,
    
    /// 需要忽略的文件扩展名
    ignore_extensions: HashSet<String>,
}

/// 项目大小统计结果
#[derive(Debug, Clone, Default)]
pub struct ProjectSizeInfo {
    /// 代码文件总大小（不包含依赖）
    pub code_size: u64,
    
    /// 依赖总大小
    pub dependency_size: u64,
    
    /// 项目总大小（包含所有文件）
    pub total_size: u64,
    
    /// 代码文件数量
    pub code_file_count: usize,
    
    /// 依赖文件数量
    pub dependency_file_count: usize,
    
    /// 总文件数量
    pub total_file_count: usize,
    
    /// 最后修改时间
    pub last_modified: Option<std::time::SystemTime>,
}

/// 目录大小统计
#[derive(Debug, Clone)]
pub struct DirectorySizeInfo {
    /// 目录路径
    pub path: PathBuf,
    
    /// 目录大小
    pub size: u64,
    
    /// 文件数量
    pub file_count: usize,
    
    /// 是否是依赖目录
    pub is_dependency: bool,
}

impl SizeCalculator {
    /// 创建新的大小计算器
    pub fn new() -> Self {
        Self {
            ignore_dirs: Self::default_ignore_dirs(),
            ignore_extensions: Self::default_ignore_extensions(),
        }
    }
    
    /// 使用自定义忽略规则创建计算器
    pub fn with_custom_ignore(
        ignore_dirs: HashSet<String>,
        ignore_extensions: HashSet<String>,
    ) -> Self {
        Self {
            ignore_dirs,
            ignore_extensions,
        }
    }
    
    /// 计算项目的完整大小信息
    pub async fn calculate_project_size(&self, project_path: &Path) -> Result<ProjectSizeInfo> {
        let mut size_info = ProjectSizeInfo::default();
        
        self.calculate_directory_recursive(project_path, &mut size_info).await?;
        
        Ok(size_info)
    }
    
    /// 只计算代码大小（排除依赖）
    pub async fn calculate_code_size(&self, project_path: &Path) -> Result<u64> {
        let size_info = self.calculate_project_size(project_path).await?;
        Ok(size_info.code_size)
    }
    
    /// 计算指定目录的大小
    pub fn calculate_directory_size<'a>(&'a self, dir_path: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<DirectorySizeInfo>> + Send + 'a>> {
        Box::pin(async move {
            let mut total_size = 0u64;
            let mut file_count = 0usize;
            
            let is_dependency = self.is_dependency_directory(dir_path);
            
            let mut entries = fs::read_dir(dir_path).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let metadata = entry.metadata().await?;
                
                if metadata.is_file() {
                    total_size += metadata.len();
                    file_count += 1;
                } else if metadata.is_dir() {
                    // 递归计算子目录大小
                    let sub_info = self.calculate_directory_size(&entry.path()).await?;
                    total_size += sub_info.size;
                    file_count += sub_info.file_count;
                }
            }
            
            Ok(DirectorySizeInfo {
                path: dir_path.to_path_buf(),
                size: total_size,
                file_count,
                is_dependency,
            })
        })
    }
    
    /// 递归计算目录大小
    fn calculate_directory_recursive<'a>(
        &'a self,
        dir_path: &'a Path,
        size_info: &'a mut ProjectSizeInfo,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut entries = fs::read_dir(dir_path).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let metadata = entry.metadata().await?;
                
                if metadata.is_file() {
                    self.process_file(&path, &metadata, size_info).await?;
                } else if metadata.is_dir() {
                    if self.should_process_directory(&path) {
                        if self.is_dependency_directory(&path) {
                            // 这是依赖目录，只计算总大小
                            let dep_info = self.calculate_directory_size(&path).await?;
                            size_info.dependency_size += dep_info.size;
                            size_info.dependency_file_count += dep_info.file_count;
                            size_info.total_size += dep_info.size;
                            size_info.total_file_count += dep_info.file_count;
                        } else {
                            // 普通代码目录，递归处理
                            self.calculate_directory_recursive(&path, size_info).await?;
                        }
                    }
                }
            }
            
            Ok(())
        })
    }
    
    /// 处理单个文件
    async fn process_file(
        &self,
        file_path: &Path,
        metadata: &Metadata,
        size_info: &mut ProjectSizeInfo,
    ) -> Result<()> {
        let file_size = metadata.len();
        let is_ignored = self.should_ignore_file(file_path);
        
        // 更新最后修改时间
        if let Ok(modified) = metadata.modified() {
            if size_info.last_modified.is_none() || 
               size_info.last_modified.unwrap() < modified {
                size_info.last_modified = Some(modified);
            }
        }
        
        // 更新总计
        size_info.total_size += file_size;
        size_info.total_file_count += 1;
        
        // 如果不是被忽略的文件，计入代码大小
        if !is_ignored {
            size_info.code_size += file_size;
            size_info.code_file_count += 1;
        }
        
        Ok(())
    }
    
    /// 检查是否应该处理该目录
    fn should_process_directory(&self, dir_path: &Path) -> bool {
        let dir_name = match dir_path.file_name() {
            Some(name) => name.to_string_lossy(),
            None => return false,
        };
        
        // 不处理隐藏目录（以 . 开头），除了 .git
        if dir_name.starts_with('.') && dir_name != ".git" {
            return false;
        }
        
        // 处理所有其他目录
        true
    }
    
    /// 检查是否是依赖目录
    fn is_dependency_directory(&self, dir_path: &Path) -> bool {
        let dir_name = match dir_path.file_name() {
            Some(name) => name.to_string_lossy(),
            None => return false,
        };
        
        self.ignore_dirs.contains(dir_name.as_ref())
    }
    
    /// 检查是否应该忽略该文件
    fn should_ignore_file(&self, file_path: &Path) -> bool {
        // 检查文件扩展名
        if let Some(extension) = file_path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            if self.ignore_extensions.contains(&ext) {
                return true;
            }
        }
        
        // 检查文件名
        if let Some(file_name) = file_path.file_name() {
            let name = file_name.to_string_lossy();
            
            // 忽略隐藏文件
            if name.starts_with('.') {
                return true;
            }
            
            // 忽略常见的临时文件
            if name.ends_with('~') || name.ends_with(".tmp") || name.ends_with(".temp") {
                return true;
            }
        }
        
        false
    }
    
    /// 获取默认忽略的目录
    fn default_ignore_dirs() -> HashSet<String> {
        let mut dirs = HashSet::new();
        
        // 各语言的依赖目录
        dirs.insert("node_modules".to_string());
        dirs.insert("target".to_string());
        dirs.insert("build".to_string());
        dirs.insert("dist".to_string());
        dirs.insert("out".to_string());
        dirs.insert("bin".to_string());
        dirs.insert("obj".to_string());
        
        // Python 相关
        dirs.insert("__pycache__".to_string());
        dirs.insert("venv".to_string());
        dirs.insert("env".to_string());
        dirs.insert(".venv".to_string());
        dirs.insert(".env".to_string());
        dirs.insert("site-packages".to_string());
        
        // 版本控制
        dirs.insert(".git".to_string());
        dirs.insert(".svn".to_string());
        dirs.insert(".hg".to_string());
        
        // IDE 和编辑器
        dirs.insert(".vscode".to_string());
        dirs.insert(".idea".to_string());
        dirs.insert(".vs".to_string());
        
        // 其他
        dirs.insert("vendor".to_string());
        dirs.insert("bower_components".to_string());
        
        dirs
    }
    
    /// 获取默认忽略的文件扩展名
    fn default_ignore_extensions() -> HashSet<String> {
        let mut exts = HashSet::new();
        
        // 编译产物
        exts.insert("o".to_string());
        exts.insert("obj".to_string());
        exts.insert("exe".to_string());
        exts.insert("dll".to_string());
        exts.insert("so".to_string());
        exts.insert("dylib".to_string());
        exts.insert("a".to_string());
        exts.insert("lib".to_string());
        exts.insert("class".to_string());
        exts.insert("pyc".to_string());
        exts.insert("pyo".to_string());
        
        // 日志和临时文件
        exts.insert("log".to_string());
        exts.insert("tmp".to_string());
        exts.insert("temp".to_string());
        exts.insert("cache".to_string());
        exts.insert("lock".to_string());
        
        // 备份文件
        exts.insert("bak".to_string());
        exts.insert("backup".to_string());
        exts.insert("swp".to_string());
        
        exts
    }
    
    /// 获取项目的主要依赖目录大小
    pub async fn get_dependency_directories(&self, project_path: &Path) -> Result<Vec<DirectorySizeInfo>> {
        let mut dependency_dirs = Vec::new();
        
        let mut entries = fs::read_dir(project_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() && self.is_dependency_directory(&path) {
                let dir_info = self.calculate_directory_size(&path).await?;
                dependency_dirs.push(dir_info);
            }
        }
        
        // 按大小排序
        dependency_dirs.sort_by(|a, b| b.size.cmp(&a.size));
        
        Ok(dependency_dirs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_calculate_empty_directory() {
        let calculator = SizeCalculator::new();
        let temp_dir = tempdir().unwrap();
        
        let size_info = calculator.calculate_project_size(temp_dir.path()).await.unwrap();
        
        assert_eq!(size_info.code_size, 0);
        assert_eq!(size_info.dependency_size, 0);
        assert_eq!(size_info.total_size, 0);
        assert_eq!(size_info.code_file_count, 0);
    }

    #[tokio::test]
    async fn test_calculate_simple_project() {
        let calculator = SizeCalculator::new();
        let temp_dir = tempdir().unwrap();
        
        // 创建一些测试文件
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        
        let main_file = src_dir.join("main.rs");
        fs::write(&main_file, "fn main() { println!(\"Hello, world!\"); }").unwrap();
        
        let readme_file = temp_dir.path().join("README.md");
        fs::write(&readme_file, "# Test Project\n\nThis is a test.").unwrap();
        
        let size_info = calculator.calculate_project_size(temp_dir.path()).await.unwrap();
        
        assert!(size_info.code_size > 0);
        assert_eq!(size_info.dependency_size, 0);
        assert_eq!(size_info.total_size, size_info.code_size);
        assert_eq!(size_info.code_file_count, 2);
    }

    #[tokio::test]
    async fn test_ignore_dependency_directories() {
        let calculator = SizeCalculator::new();
        let temp_dir = tempdir().unwrap();
        
        // 创建代码文件
        let src_file = temp_dir.path().join("index.js");
        fs::write(&src_file, "console.log('Hello, world!');").unwrap();
        
        // 创建 node_modules 目录
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir(&node_modules).unwrap();
        
        let package_dir = node_modules.join("some-package");
        fs::create_dir(&package_dir).unwrap();
        
        let package_file = package_dir.join("index.js");
        fs::write(&package_file, "module.exports = {};".repeat(100)).unwrap();
        
        let size_info = calculator.calculate_project_size(temp_dir.path()).await.unwrap();
        
        // 代码大小应该只包含 index.js
        assert!(size_info.code_size > 0);
        assert!(size_info.code_size < 100); // 主文件很小
        
        // 依赖大小应该包含 node_modules
        assert!(size_info.dependency_size > 1000); // 依赖文件较大
        
        // 总大小应该是两者之和
        assert_eq!(size_info.total_size, size_info.code_size + size_info.dependency_size);
    }

    #[test]
    fn test_is_dependency_directory() {
        let calculator = SizeCalculator::new();
        
        assert!(calculator.is_dependency_directory(Path::new("/project/node_modules")));
        assert!(calculator.is_dependency_directory(Path::new("/project/target")));
        assert!(!calculator.is_dependency_directory(Path::new("/project/src")));
        assert!(!calculator.is_dependency_directory(Path::new("/project/tests")));
    }

    #[test]
    fn test_should_ignore_file() {
        let calculator = SizeCalculator::new();
        
        // 应该忽略的文件
        assert!(calculator.should_ignore_file(Path::new("file.log")));
        assert!(calculator.should_ignore_file(Path::new("file.tmp")));
        assert!(calculator.should_ignore_file(Path::new(".hidden")));
        assert!(calculator.should_ignore_file(Path::new("file~")));
        
        // 不应该忽略的文件
        assert!(!calculator.should_ignore_file(Path::new("main.rs")));
        assert!(!calculator.should_ignore_file(Path::new("README.md")));
        assert!(!calculator.should_ignore_file(Path::new("package.json")));
    }
}