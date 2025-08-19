use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
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
    
    /// 缓存配置
    pub cache: CacheConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 是否启用缓存
    pub enabled: bool,
    
    /// 缓存过期时间（小时）
    #[serde(with = "duration_hours")]
    pub expiry_duration: Duration,
    
    /// 最大缓存条目数
    pub max_entries: usize,
    
    /// 自动清理过期缓存的间隔（小时）
    #[serde(with = "duration_hours")]
    pub cleanup_interval: Duration,
}

/// Duration 序列化为小时数
mod duration_hours {
    use super::*;
    use serde::{Deserializer, Serializer, Deserialize};
    
    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hours = duration.as_secs() / 3600;
        serializer.serialize_u64(hours)
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hours = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(hours * 3600))
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            expiry_duration: Duration::from_secs(24 * 3600), // 24 小时
            max_entries: 1000,
            cleanup_interval: Duration::from_secs(6 * 3600), // 6 小时
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan_paths: crate::config::defaults::DefaultConfig::default_scan_paths(),
            ignore: ProjectIgnoreConfig::default(),
            scan: ScanConfig::default(),
            display: DisplayConfig::default(),
            cache: CacheConfig::default(),
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
            // 尝试加载现有配置，如果失败则尝试兼容性加载
            match Self::load_from_file(&config_path) {
                Ok(config) => Ok(config),
                Err(_) => {
                    // 尝试向后兼容的配置加载
                    Self::load_with_backward_compatibility(&config_path)
                }
            }
        } else {
            let config = Self::default();
            config.save_to_file(&config_path)?;
            Ok(config)
        }
    }
    
    /// 向后兼容的配置加载
    fn load_with_backward_compatibility(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        
        // 尝试解析为部分配置（允许缺少字段）
        match toml::from_str::<PartialConfig>(&content) {
            Ok(partial_config) => {
                // 将部分配置转换为完整配置
                let mut config = Self::default();
                
                if let Some(scan_paths) = partial_config.scan_paths {
                    config.scan_paths = scan_paths;
                }
                if let Some(ignore) = partial_config.ignore {
                    config.ignore = ignore;
                }
                if let Some(scan) = partial_config.scan {
                    config.scan = scan;
                }
                if let Some(display) = partial_config.display {
                    config.display = display;
                }
                if let Some(cache) = partial_config.cache {
                    config.cache = cache;
                }
                
                // 保存更新后的配置文件
                config.save_to_file(path)?;
                Ok(config)
            }
            Err(e) => {
                eprintln!("警告: 配置文件格式错误，使用默认配置。错误: {}", e);
                let config = Self::default();
                config.save_to_file(path)?;
                Ok(config)
            }
        }
    }
    
    /// 将配置转换为 SizeCache 可以使用的 CacheConfig
    pub fn to_size_cache_config(&self) -> crate::scanner::size_cache::CacheConfig {
        crate::scanner::size_cache::CacheConfig {
            enabled: self.cache.enabled,
            expiry_duration: self.cache.expiry_duration,
            max_entries: self.cache.max_entries,
        }
    }
}

impl CacheConfig {
    /// 转换为 SizeCache 的配置
    pub fn to_size_cache_config(&self) -> crate::scanner::size_cache::CacheConfig {
        crate::scanner::size_cache::CacheConfig {
            enabled: self.enabled,
            expiry_duration: self.expiry_duration,
            max_entries: self.max_entries,
        }
    }
}

/// 部分配置结构体，用于向后兼容
#[derive(Debug, Serialize, Deserialize)]
struct PartialConfig {
    /// 扫描路径
    pub scan_paths: Option<Vec<String>>,
    
    /// 忽略配置
    pub ignore: Option<ProjectIgnoreConfig>,
    
    /// 扫描配置
    pub scan: Option<ScanConfig>,
    
    /// 显示配置
    pub display: Option<DisplayConfig>,
    
    /// 缓存配置
    pub cache: Option<CacheConfig>,
}