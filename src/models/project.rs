use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        self.dependencies.iter().map(|d| d.size).sum()
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