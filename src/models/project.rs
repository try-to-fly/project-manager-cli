use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 依赖计算状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyCalculationStatus {
    /// 未开始计算
    NotCalculated,
    /// 正在计算中
    Calculating,
    /// 计算完成
    Completed,
    /// 计算失败
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// 项目名称
    pub name: String,
    
    /// 项目路径
    pub path: PathBuf,
    
    /// 项目类型
    pub project_type: ProjectType,
    
    /// 代码大小（不包含依赖）
    pub code_size: u64,
    
    /// 总大小（包含依赖）
    pub total_size: u64,
    
    /// 被 gitignore 排除的文件大小（不含依赖目录，避免重复计算）
    pub gitignore_excluded_size: u64,
    
    /// 代码文件数量（不包含依赖）
    pub code_file_count: usize,
    
    /// 依赖文件数量
    pub dependency_file_count: usize,
    
    /// 总文件数量
    pub total_file_count: usize,
    
    /// 被 gitignore 排除的文件数量
    pub gitignore_excluded_file_count: usize,
    
    /// 最后修改时间
    pub last_modified: DateTime<Utc>,
    
    /// Git 信息（如果是 Git 项目）
    pub git_info: Option<GitInfo>,
    
    /// 依赖信息
    pub dependencies: Vec<DependencyInfo>,
    
    /// 是否被用户标记为忽略
    pub is_ignored: bool,
    
    /// 项目描述（从 package.json、Cargo.toml 等获取）
    pub description: Option<String>,
    
    /// 依赖计算状态
    pub dependency_calculation_status: DependencyCalculationStatus,
    
    /// 缓存的依赖总大小（从异步计算中获得）
    pub cached_dependency_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectType {
    /// Git 仓库（可能包含多种语言）
    Git,
    
    /// Node.js 项目
    NodeJs,
    
    /// Rust 项目
    Rust,
    
    /// Python 项目
    Python,
    
    /// Go 项目
    Go,
    
    /// Java/Maven 项目
    Java,
    
    /// C++ 项目
    Cpp,
    
    /// 混合项目（包含多种项目类型）
    Mixed(Vec<ProjectType>),
    
    /// 未知类型
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    /// Git 远程仓库 URL
    pub remote_url: Option<String>,
    
    /// 当前分支
    pub current_branch: Option<String>,
    
    /// 最后提交时间
    pub last_commit_time: Option<DateTime<Utc>>,
    
    /// 最后提交信息
    pub last_commit_message: Option<String>,
    
    /// 最后提交作者
    pub last_commit_author: Option<String>,
    
    /// 是否有未提交的更改
    pub has_uncommitted_changes: bool,
    
    /// 是否有未推送的提交
    pub has_unpushed_commits: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// 依赖类型
    pub dependency_type: DependencyType,
    
    /// 依赖目录路径
    pub path: PathBuf,
    
    /// 依赖大小
    pub size: u64,
    
    /// 包数量（对于 node_modules）
    pub package_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    /// Node.js node_modules
    NodeModules,
    
    /// Rust target 目录
    RustTarget,
    
    /// Python __pycache__
    PythonCache,
    
    /// Python venv/env
    PythonVenv,
    
    /// Go mod cache
    GoMod,
    
    /// Maven .m2
    Maven,
    
    /// 其他类型的依赖
    Other(String),
}

impl Project {
    /// 获取依赖总大小
    pub fn dependency_size(&self) -> u64 {
        // 优先使用缓存的依赖大小（从异步计算得到的准确值）
        // 如果没有缓存值，则使用 dependencies 向量计算
        self.cached_dependency_size
            .unwrap_or_else(|| self.dependencies.iter().map(|d| d.size).sum())
    }
    
    /// 获取项目代码大小（不包含依赖）
    pub fn size(&self) -> u64 {
        self.code_size
    }
    
    /// 获取代码文件数量（准确值）
    pub fn file_count(&self) -> usize {
        self.code_file_count
    }
    
    /// 获取总文件数量（包含依赖）
    pub fn total_files(&self) -> usize {
        self.total_file_count
    }
    
    /// 获取依赖文件数量
    pub fn dependency_files(&self) -> usize {
        self.dependency_file_count
    }
    
    /// 检查是否有未提交的更改
    pub fn has_uncommitted_changes(&self) -> bool {
        self.git_info.as_ref()
            .map(|info| info.has_uncommitted_changes)
            .unwrap_or(false)
    }
    
    /// 检查是否是 monorepo
    pub fn is_monorepo(&self) -> bool {
        match &self.project_type {
            ProjectType::Mixed(_) => true,
            _ => self.dependencies.len() > 1,
        }
    }
    
    /// 获取项目类型的显示名称
    pub fn type_display_name(&self) -> String {
        match &self.project_type {
            ProjectType::Git => "Git".to_string(),
            ProjectType::NodeJs => "Node.js".to_string(),
            ProjectType::Rust => "Rust".to_string(),
            ProjectType::Python => "Python".to_string(),
            ProjectType::Go => "Go".to_string(),
            ProjectType::Java => "Java".to_string(),
            ProjectType::Cpp => "C++".to_string(),
            ProjectType::Mixed(types) => {
                let type_names: Vec<String> = types.iter()
                    .map(|t| match t {
                        ProjectType::NodeJs => "Node.js",
                        ProjectType::Rust => "Rust",
                        ProjectType::Python => "Python",
                        ProjectType::Go => "Go",
                        ProjectType::Java => "Java",
                        ProjectType::Cpp => "C++",
                        _ => "Other",
                    })
                    .map(|s| s.to_string())
                    .collect();
                format!("Mixed ({})", type_names.join(", "))
            }
            ProjectType::Unknown => "Unknown".to_string(),
        }
    }
    
    /// 获取依赖计算状态的显示文本
    pub fn dependency_status_display(&self) -> &str {
        match &self.dependency_calculation_status {
            DependencyCalculationStatus::NotCalculated => "等待计算",
            DependencyCalculationStatus::Calculating => "计算中...",
            DependencyCalculationStatus::Completed => "",
            DependencyCalculationStatus::Failed(_) => "计算失败",
        }
    }
    
    /// 获取远程仓库的简短名称
    pub fn remote_name(&self) -> Option<String> {
        self.git_info.as_ref()
            .and_then(|info| info.remote_url.as_ref())
            .and_then(|url| {
                // 提取 GitHub/GitLab 等的仓库名
                if let Some(captures) = regex::Regex::new(r"[:/]([^/]+)/([^/]+?)(?:\.git)?/?$")
                    .ok()
                    .and_then(|re| re.captures(url))
                {
                    Some(format!("{}/{}", &captures[1], &captures[2]))
                } else {
                    Some(url.clone())
                }
            })
    }
}

impl ProjectType {
    /// 获取项目类型的字符串表示
    pub fn as_str(&self) -> &str {
        match self {
            ProjectType::Git => "git",
            ProjectType::NodeJs => "nodejs",
            ProjectType::Rust => "rust",
            ProjectType::Python => "python",
            ProjectType::Go => "go",
            ProjectType::Java => "java",
            ProjectType::Cpp => "cpp",
            ProjectType::Mixed(_) => "mixed",
            ProjectType::Unknown => "unknown",
        }
    }
    
    /// 获取项目类型的优先级（用于排序）
    pub fn priority(&self) -> u8 {
        match self {
            ProjectType::Git => 1,
            ProjectType::Mixed(_) => 2,
            ProjectType::NodeJs => 3,
            ProjectType::Rust => 4,
            ProjectType::Python => 5,
            ProjectType::Go => 6,
            ProjectType::Java => 7,
            ProjectType::Cpp => 8,
            ProjectType::Unknown => 9,
        }
    }
}

impl DependencyType {
    /// 获取依赖类型的显示名称
    pub fn display_name(&self) -> &str {
        match self {
            DependencyType::NodeModules => "node_modules",
            DependencyType::RustTarget => "target",
            DependencyType::PythonCache => "__pycache__",
            DependencyType::PythonVenv => "venv",
            DependencyType::GoMod => "go.mod",
            DependencyType::Maven => ".m2",
            DependencyType::Other(name) => name,
        }
    }
}