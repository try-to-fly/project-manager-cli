use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 扫描的根目录列表
    pub scan_paths: Vec<String>,
    
    /// 忽略配置
    pub ignore: ProjectIgnoreConfig,
    
    /// 扫描配置
    pub scan: ScanConfig,
    
    /// 显示配置
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIgnoreConfig {
    /// 忽略的目录名
    pub directories: HashSet<String>,
    
    /// 忽略的文件扩展名
    pub extensions: HashSet<String>,
    
    /// 忽略的完整路径
    pub paths: HashSet<String>,
    
    /// 手动标记为忽略的项目路径
    pub projects: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// 最大扫描深度
    pub max_depth: Option<usize>,
    
    /// 是否跟随符号链接
    pub follow_symlinks: bool,
    
    /// 并发扫描线程数
    pub concurrent_scans: usize,
    
    /// 是否扫描隐藏目录
    pub scan_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// 默认排序字段
    pub default_sort: SortField,
    
    /// 大小显示单位
    pub size_unit: SizeUnit,
    
    /// 时间格式
    pub time_format: String,
    
    /// 是否显示隐藏项目
    pub show_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortField {
    Name,
    Size,
    LastModified,
    ProjectType,
    DependencySize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SizeUnit {
    Auto,
    Bytes,
    KB,
    MB,
    GB,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan_paths: crate::config::defaults::DefaultConfig::default_scan_paths(),
            ignore: ProjectIgnoreConfig::default(),
            scan: ScanConfig::default(),
            display: DisplayConfig::default(),
        }
    }
}

impl Default for ProjectIgnoreConfig {
    fn default() -> Self {
        Self {
            directories: crate::config::defaults::DefaultConfig::default_ignore_dirs(),
            extensions: crate::config::defaults::DefaultConfig::default_ignore_extensions(),
            paths: HashSet::new(),
            projects: HashSet::new(),
        }
    }
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_depth: Some(10),
            follow_symlinks: false,
            concurrent_scans: 4,
            scan_hidden: false,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            default_sort: SortField::LastModified,
            size_unit: SizeUnit::Auto,
            time_format: "%Y-%m-%d %H:%M:%S".to_string(),
            show_hidden: false,
        }
    }
}

impl Config {
    /// 从文件加载配置
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// 保存配置到文件
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// 获取默认配置文件路径
    pub fn default_config_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("无法找到配置目录"))?;
        path.push("project-manager-cli");
        path.push("config.toml");
        Ok(path)
    }
    
    /// 加载配置，如果文件不存在则创建默认配置
    pub fn load_or_create_default() -> Result<Self> {
        let config_path = Self::default_config_path()?;
        
        if config_path.exists() {
            Self::load_from_file(&config_path)
        } else {
            let config = Self::default();
            config.save_to_file(&config_path)?;
            Ok(config)
        }
    }
}