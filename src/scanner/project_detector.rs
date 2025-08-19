use std::path::{Path, PathBuf};
use crate::models::{ProjectType, DependencyInfo, DependencyType};
use anyhow::Result;

/// 项目类型检测器
pub struct ProjectDetector;

/// 检测到的项目信息
#[derive(Debug, Clone)]
pub struct DetectedProject {
    /// 项目路径
    pub path: PathBuf,
    
    /// 项目类型
    pub project_type: ProjectType,
    
    /// 项目名称
    pub name: String,
    
    /// 项目描述（如果能从配置文件中获取）
    pub description: Option<String>,
    
    /// 检测到的依赖信息
    pub dependencies: Vec<DependencyInfo>,
    
    /// 是否是 Git 仓库
    pub is_git_repo: bool,
}

impl ProjectDetector {
    pub fn new() -> Self {
        Self
    }
    
    /// 检测指定路径是否是一个项目，并返回项目信息
    pub async fn detect_project(&self, path: &Path) -> Result<Option<DetectedProject>> {
        if !path.is_dir() {
            return Ok(None);
        }
        
        let mut detected_types = Vec::new();
        let mut dependencies = Vec::new();
        let mut description = None;
        
        // 检测 Git 仓库
        let is_git_repo = path.join(".git").exists();
        if is_git_repo {
            detected_types.push(ProjectType::Git);
        }
        
        // 检测 Node.js 项目
        if let Some((node_desc, node_deps)) = self.detect_nodejs(path).await? {
            detected_types.push(ProjectType::NodeJs);
            if description.is_none() {
                description = node_desc;
            }
            dependencies.extend(node_deps);
        }
        
        // 检测 Rust 项目
        if let Some((rust_desc, rust_deps)) = self.detect_rust(path).await? {
            detected_types.push(ProjectType::Rust);
            if description.is_none() {
                description = rust_desc;
            }
            dependencies.extend(rust_deps);
        }
        
        // 检测 Python 项目
        if let Some((python_desc, python_deps)) = self.detect_python(path).await? {
            detected_types.push(ProjectType::Python);
            if description.is_none() {
                description = python_desc;
            }
            dependencies.extend(python_deps);
        }
        
        // 检测 Go 项目
        if let Some((go_desc, go_deps)) = self.detect_go(path).await? {
            detected_types.push(ProjectType::Go);
            if description.is_none() {
                description = go_desc;
            }
            dependencies.extend(go_deps);
        }
        
        // 检测 Java 项目
        if let Some((java_desc, java_deps)) = self.detect_java(path).await? {
            detected_types.push(ProjectType::Java);
            if description.is_none() {
                description = java_desc;
            }
            dependencies.extend(java_deps);
        }
        
        // 检测 C++ 项目
        if let Some((cpp_desc, cpp_deps)) = self.detect_cpp(path).await? {
            detected_types.push(ProjectType::Cpp);
            if description.is_none() {
                description = cpp_desc;
            }
            dependencies.extend(cpp_deps);
        }
        
        // 如果没有检测到任何项目类型，但是是 Git 仓库，仍然返回项目信息
        if detected_types.is_empty() && !is_git_repo {
            return Ok(None);
        }
        
        // 确定最终的项目类型
        let project_type = match detected_types.len() {
            0 => ProjectType::Unknown,
            1 => detected_types[0].clone(),
            _ => {
                // 过滤掉 Git 类型，因为它不是真正的项目类型
                let non_git_types: Vec<_> = detected_types
                    .into_iter()
                    .filter(|t| *t != ProjectType::Git)
                    .collect();
                
                match non_git_types.len() {
                    0 => ProjectType::Git,  // 只有 Git，没有其他项目类型
                    1 => non_git_types[0].clone(),
                    _ => ProjectType::Mixed(non_git_types),
                }
            }
        };
        
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        Ok(Some(DetectedProject {
            path: path.to_path_buf(),
            project_type,
            name,
            description,
            dependencies,
            is_git_repo,
        }))
    }
    
    /// 检测 Node.js 项目
    async fn detect_nodejs(&self, path: &Path) -> Result<Option<(Option<String>, Vec<DependencyInfo>)>> {
        let package_json = path.join("package.json");
        let node_modules = path.join("node_modules");
        
        if !package_json.exists() && !node_modules.exists() {
            return Ok(None);
        }
        
        let mut dependencies = Vec::new();
        let mut description = None;
        
        // 读取 package.json 获取描述信息
        if package_json.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&package_json).await {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    description = json.get("description")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string());
                }
            }
        }
        
        // 检测 node_modules
        if node_modules.exists() {
            if let Ok(size) = self.calculate_directory_size(&node_modules).await {
                let package_count = self.count_packages(&node_modules).await.unwrap_or(0);
                dependencies.push(DependencyInfo {
                    dependency_type: DependencyType::NodeModules,
                    path: node_modules,
                    size,
                    package_count: Some(package_count),
                });
            }
        }
        
        Ok(Some((description, dependencies)))
    }
    
    /// 检测 Rust 项目
    async fn detect_rust(&self, path: &Path) -> Result<Option<(Option<String>, Vec<DependencyInfo>)>> {
        let cargo_toml = path.join("Cargo.toml");
        let target_dir = path.join("target");
        
        if !cargo_toml.exists() {
            return Ok(None);
        }
        
        let mut dependencies = Vec::new();
        let mut description = None;
        
        // 读取 Cargo.toml 获取描述信息
        if let Ok(content) = tokio::fs::read_to_string(&cargo_toml).await {
            if let Ok(cargo_config) = toml::from_str::<toml::Value>(&content) {
                description = cargo_config.get("package")
                    .and_then(|p| p.get("description"))
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string());
            }
        }
        
        // 检测 target 目录
        if target_dir.exists() {
            if let Ok(size) = self.calculate_directory_size(&target_dir).await {
                dependencies.push(DependencyInfo {
                    dependency_type: DependencyType::RustTarget,
                    path: target_dir,
                    size,
                    package_count: None,
                });
            }
        }
        
        Ok(Some((description, dependencies)))
    }
    
    /// 检测 Python 项目
    async fn detect_python(&self, path: &Path) -> Result<Option<(Option<String>, Vec<DependencyInfo>)>> {
        let requirements_txt = path.join("requirements.txt");
        let pyproject_toml = path.join("pyproject.toml");
        let setup_py = path.join("setup.py");
        let pycache = path.join("__pycache__");
        
        // 检查是否有 Python 项目的标志文件
        if !requirements_txt.exists() && !pyproject_toml.exists() && !setup_py.exists() && !pycache.exists() {
            // 检查是否有 .py 文件
            if !self.has_python_files(path).await? {
                return Ok(None);
            }
        }
        
        let mut dependencies = Vec::new();
        let mut description = None;
        
        // 读取项目描述
        if pyproject_toml.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&pyproject_toml).await {
                if let Ok(config) = toml::from_str::<toml::Value>(&content) {
                    description = config.get("project")
                        .and_then(|p| p.get("description"))
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string());
                }
            }
        }
        
        // 检测虚拟环境
        for venv_name in &["venv", "env", ".venv", ".env"] {
            let venv_path = path.join(venv_name);
            if venv_path.exists() {
                if let Ok(size) = self.calculate_directory_size(&venv_path).await {
                    dependencies.push(DependencyInfo {
                        dependency_type: DependencyType::PythonVenv,
                        path: venv_path,
                        size,
                        package_count: None,
                    });
                }
            }
        }
        
        // 检测 __pycache__
        if pycache.exists() {
            if let Ok(size) = self.calculate_directory_size(&pycache).await {
                dependencies.push(DependencyInfo {
                    dependency_type: DependencyType::PythonCache,
                    path: pycache,
                    size,
                    package_count: None,
                });
            }
        }
        
        Ok(Some((description, dependencies)))
    }
    
    /// 检测 Go 项目
    async fn detect_go(&self, path: &Path) -> Result<Option<(Option<String>, Vec<DependencyInfo>)>> {
        let go_mod = path.join("go.mod");
        let go_sum = path.join("go.sum");
        
        if !go_mod.exists() && !go_sum.exists() && !self.has_go_files(path).await? {
            return Ok(None);
        }
        
        let dependencies = Vec::new(); // Go 模块通常不在本地存储依赖
        Ok(Some((None, dependencies)))
    }
    
    /// 检测 Java 项目
    async fn detect_java(&self, path: &Path) -> Result<Option<(Option<String>, Vec<DependencyInfo>)>> {
        let pom_xml = path.join("pom.xml");
        let build_gradle = path.join("build.gradle");
        let build_gradle_kts = path.join("build.gradle.kts");
        
        if !pom_xml.exists() && !build_gradle.exists() && !build_gradle_kts.exists() {
            if !self.has_java_files(path).await? {
                return Ok(None);
            }
        }
        
        let mut dependencies = Vec::new();
        
        // 检测 Maven target 目录
        let target_dir = path.join("target");
        if target_dir.exists() {
            if let Ok(size) = self.calculate_directory_size(&target_dir).await {
                dependencies.push(DependencyInfo {
                    dependency_type: DependencyType::Other("target".to_string()),
                    path: target_dir,
                    size,
                    package_count: None,
                });
            }
        }
        
        // 检测 Gradle build 目录
        let build_dir = path.join("build");
        if build_dir.exists() {
            if let Ok(size) = self.calculate_directory_size(&build_dir).await {
                dependencies.push(DependencyInfo {
                    dependency_type: DependencyType::Other("build".to_string()),
                    path: build_dir,
                    size,
                    package_count: None,
                });
            }
        }
        
        Ok(Some((None, dependencies)))
    }
    
    /// 检测 C++ 项目
    async fn detect_cpp(&self, path: &Path) -> Result<Option<(Option<String>, Vec<DependencyInfo>)>> {
        let cmake_lists = path.join("CMakeLists.txt");
        let makefile = path.join("Makefile");
        
        if !cmake_lists.exists() && !makefile.exists() {
            if !self.has_cpp_files(path).await? {
                return Ok(None);
            }
        }
        
        let mut dependencies = Vec::new();
        
        // 检测 build 目录
        let build_dir = path.join("build");
        if build_dir.exists() {
            if let Ok(size) = self.calculate_directory_size(&build_dir).await {
                dependencies.push(DependencyInfo {
                    dependency_type: DependencyType::Other("build".to_string()),
                    path: build_dir,
                    size,
                    package_count: None,
                });
            }
        }
        
        Ok(Some((None, dependencies)))
    }
    
    /// 计算目录大小（跳过大型依赖目录）
    fn calculate_directory_size<'a>(&'a self, path: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64>> + Send + 'a>> {
        Box::pin(async move {
            let mut total_size = 0;
            let mut entries = tokio::fs::read_dir(path).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let metadata = entry.metadata().await?;
                if metadata.is_file() {
                    total_size += metadata.len();
                } else if metadata.is_dir() {
                    // 获取目录名
                    let dir_name = entry.file_name();
                    let dir_name_str = dir_name.to_string_lossy();
                    
                    // 跳过大型依赖目录，避免递归计算导致性能问题
                    if matches!(dir_name_str.as_ref(),
                        "node_modules" | ".git" | "target" | ".svn" | 
                        "__pycache__" | ".pytest_cache" |
                        "venv" | ".venv" | "env" | ".env" |
                        ".idea" | ".vscode" | ".vs" | 
                        "dist" | "build" | "out" |
                        ".gradle" | ".mvn" | 
                        "vendor" | "bower_components" |
                        ".sass-cache" | ".cache" |
                        "coverage" | ".nyc_output" |
                        ".next" | ".nuxt" | ".parcel-cache"
                    ) {
                        // 对于这些目录，只估算一个大概的大小，不递归计算
                        // 或者可以选择完全跳过
                        continue;
                    }
                    
                    // 递归计算子目录大小
                    total_size += self.calculate_directory_size(&entry.path()).await.unwrap_or(0);
                }
            }
            
            Ok(total_size)
        })
    }
    
    /// 计算 node_modules 中的包数量
    async fn count_packages(&self, node_modules_path: &Path) -> Result<usize> {
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(node_modules_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.metadata().await?.is_dir() {
                let name = entry.file_name();
                if let Some(name_str) = name.to_str() {
                    if name_str.starts_with('@') {
                        // 这是一个 scoped package，需要统计里面的子包
                        let mut scoped_entries = tokio::fs::read_dir(entry.path()).await?;
                        while let Some(scoped_entry) = scoped_entries.next_entry().await? {
                            if scoped_entry.metadata().await?.is_dir() {
                                count += 1;
                            }
                        }
                    } else {
                        count += 1;
                    }
                }
            }
        }
        
        Ok(count)
    }
    
    /// 检查目录中是否有 Python 文件
    async fn has_python_files(&self, path: &Path) -> Result<bool> {
        let mut entries = tokio::fs::read_dir(path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.metadata().await?.is_file() {
                if let Some(extension) = entry.path().extension() {
                    if extension == "py" {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// 检查目录中是否有 Go 文件
    async fn has_go_files(&self, path: &Path) -> Result<bool> {
        let mut entries = tokio::fs::read_dir(path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.metadata().await?.is_file() {
                if let Some(extension) = entry.path().extension() {
                    if extension == "go" {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// 检查目录中是否有 Java 文件
    async fn has_java_files(&self, path: &Path) -> Result<bool> {
        let mut entries = tokio::fs::read_dir(path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.metadata().await?.is_file() {
                if let Some(extension) = entry.path().extension() {
                    if extension == "java" || extension == "kt" || extension == "scala" {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// 检查目录中是否有 C++ 文件
    async fn has_cpp_files(&self, path: &Path) -> Result<bool> {
        let mut entries = tokio::fs::read_dir(path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.metadata().await?.is_file() {
                if let Some(extension) = entry.path().extension() {
                    if matches!(extension.to_str(), Some("cpp") | Some("cxx") | Some("cc") | Some("c") | Some("hpp") | Some("h")) {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
}