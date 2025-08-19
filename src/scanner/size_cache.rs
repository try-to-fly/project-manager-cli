use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tokio::fs;

/// 项目大小缓存管理器
pub struct SizeCache {
    /// 缓存文件路径
    cache_file: PathBuf,
    
    /// 内存中的缓存数据
    cache_data: CacheData,
    
    /// 缓存配置
    config: CacheConfig,
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 缓存过期时间（默认 24 小时）
    pub expiry_duration: Duration,
    
    /// 最大缓存条目数（默认 1000）
    pub max_entries: usize,
    
    /// 是否启用缓存
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            expiry_duration: Duration::from_secs(24 * 60 * 60), // 24 小时
            max_entries: 1000,
            enabled: true,
        }
    }
}

/// 缓存数据结构
#[derive(Debug, Serialize, Deserialize, Default)]
struct CacheData {
    /// 缓存条目
    entries: HashMap<String, CacheEntry>,
    
    /// 缓存元数据
    metadata: CacheMetadata,
}

/// 缓存条目
#[derive(Debug, Serialize, Deserialize, Clone)]
struct CacheEntry {
    /// 项目路径
    project_path: String,
    
    /// 项目大小信息
    size_info: CachedSizeInfo,
    
    /// 缓存创建时间
    created_at: SystemTime,
    
    /// 项目最后修改时间
    last_modified: SystemTime,
    
    /// 是否是 Git 项目
    is_git_repo: bool,
}

/// 可序列化的项目大小信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedSizeInfo {
    /// 代码文件总大小（不包含依赖）
    pub code_size: u64,
    
    /// 依赖总大小
    pub dependency_size: u64,
    
    /// 项目总大小（包含所有文件）
    pub total_size: u64,
    
    /// 被 gitignore 排除的文件大小（不含依赖目录，避免重复计算）
    pub gitignore_excluded_size: u64,
    
    /// 代码文件数量
    pub code_file_count: usize,
    
    /// 依赖文件数量
    pub dependency_file_count: usize,
    
    /// 总文件数量
    pub total_file_count: usize,
    
    /// 被 gitignore 排除的文件数量（不含依赖目录）
    pub gitignore_excluded_file_count: usize,
    
    /// 最后修改时间（序列化为时间戳）
    #[serde(with = "systemtime_serde")]
    pub last_modified: Option<SystemTime>,
}

/// SystemTime 序列化模块
mod systemtime_serde {
    use super::*;
    use serde::{Deserializer, Serializer};
    
    pub fn serialize<S>(time: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match time {
            Some(t) => {
                let duration_since_epoch = t.duration_since(SystemTime::UNIX_EPOCH)
                    .map_err(serde::ser::Error::custom)?;
                serializer.serialize_u64(duration_since_epoch.as_secs())
            }
            None => serializer.serialize_none(),
        }
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs: Option<u64> = Option::deserialize(deserializer)?;
        Ok(secs.map(|s| SystemTime::UNIX_EPOCH + Duration::from_secs(s)))
    }
}

/// 缓存元数据
#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    /// 缓存格式版本
    version: u32,
    
    /// 缓存创建时间
    created_at: SystemTime,
    
    /// 缓存最后更新时间
    updated_at: SystemTime,
}

impl Default for CacheMetadata {
    fn default() -> Self {
        let now = SystemTime::now();
        Self {
            version: 1,
            created_at: now,
            updated_at: now,
        }
    }
}

impl SizeCache {
    /// 创建新的缓存管理器
    pub async fn new(config: CacheConfig) -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .or_else(|| dirs::home_dir().map(|p| p.join(".cache")))
            .unwrap_or_else(|| PathBuf::from("."));
        
        let cache_dir = cache_dir.join("project-manager-cli");
        
        // 确保缓存目录存在
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).await?;
        }
        
        let cache_file = cache_dir.join("size_cache.json");
        
        let mut cache = Self {
            cache_file,
            cache_data: CacheData::default(),
            config,
        };
        
        // 加载现有缓存
        cache.load_cache().await?;
        
        Ok(cache)
    }
    
    /// 从缓存获取项目大小信息
    pub async fn get(&self, project_path: &Path) -> Option<CachedSizeInfo> {
        if !self.config.enabled {
            return None;
        }
        
        let key = self.generate_cache_key(project_path);
        
        if let Some(entry) = self.cache_data.entries.get(&key) {
            // 检查缓存是否过期
            if !self.is_cache_expired(&entry) {
                // 检查项目是否有更新
                if let Ok(last_modified) = self.get_project_last_modified(project_path).await {
                    if last_modified <= entry.last_modified {
                        return Some(entry.size_info.clone());
                    }
                }
            }
        }
        
        None
    }
    
    /// 将项目大小信息存入缓存
    pub async fn put(&mut self, project_path: &Path, size_info: CachedSizeInfo, is_git_repo: bool) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        let key = self.generate_cache_key(project_path);
        let last_modified = self.get_project_last_modified(project_path).await?;
        
        let entry = CacheEntry {
            project_path: project_path.to_string_lossy().to_string(),
            size_info,
            created_at: SystemTime::now(),
            last_modified,
            is_git_repo,
        };
        
        self.cache_data.entries.insert(key, entry);
        
        // 检查是否超过最大条目数
        if self.cache_data.entries.len() > self.config.max_entries {
            self.cleanup_old_entries().await;
        }
        
        // 更新元数据
        self.cache_data.metadata.updated_at = SystemTime::now();
        
        // 保存缓存
        self.save_cache().await?;
        
        Ok(())
    }
    
    /// 清除过期的缓存条目
    pub async fn cleanup_expired(&mut self) -> Result<usize> {
        let initial_count = self.cache_data.entries.len();
        let now = SystemTime::now();
        
        // 收集需要删除的键
        let expired_keys: Vec<String> = self.cache_data.entries
            .iter()
            .filter_map(|(key, entry)| {
                if let Ok(elapsed) = entry.created_at.elapsed() {
                    if elapsed > self.config.expiry_duration {
                        Some(key.clone())
                    } else {
                        None
                    }
                } else {
                    Some(key.clone()) // 时间异常，认为已过期
                }
            })
            .collect();
        
        // 删除过期的条目
        for key in &expired_keys {
            self.cache_data.entries.remove(key);
        }
        
        let removed_count = initial_count - self.cache_data.entries.len();
        
        if removed_count > 0 {
            self.cache_data.metadata.updated_at = now;
            self.save_cache().await?;
        }
        
        Ok(removed_count)
    }
    
    /// 清除所有缓存
    pub async fn clear_all(&mut self) -> Result<()> {
        self.cache_data.entries.clear();
        self.cache_data.metadata.updated_at = SystemTime::now();
        self.save_cache().await?;
        Ok(())
    }
    
    /// 获取缓存统计信息
    pub fn get_stats(&self) -> CacheStats {
        let now = SystemTime::now();
        let mut expired_count = 0;
        let mut git_repos = 0;
        let mut total_cached_size = 0u64;
        let mut total_code_size = 0u64;
        let mut total_dependency_size = 0u64;
        let mut total_gitignore_size = 0u64;
        
        for entry in self.cache_data.entries.values() {
            if self.is_cache_expired(entry) {
                expired_count += 1;
            }
            
            if entry.is_git_repo {
                git_repos += 1;
            }
            
            total_cached_size += entry.size_info.total_size;
            total_code_size += entry.size_info.code_size;
            total_dependency_size += entry.size_info.dependency_size;
            total_gitignore_size += entry.size_info.gitignore_excluded_size;
        }
        
        CacheStats {
            total_entries: self.cache_data.entries.len(),
            expired_entries: expired_count,
            git_repositories: git_repos,
            total_cached_size,
            total_code_size,
            total_dependency_size,
            total_gitignore_size,
            cache_file_size: self.get_cache_file_size(),
            last_updated: self.cache_data.metadata.updated_at,
        }
    }
    
    /// 检查特定路径的缓存状态
    pub fn check_cache_status(&self, project_path: &Path) -> CacheStatus {
        let key = self.generate_cache_key(project_path);
        
        match self.cache_data.entries.get(&key) {
            Some(entry) => {
                if self.is_cache_expired(entry) {
                    CacheStatus::Expired
                } else {
                    CacheStatus::Valid
                }
            }
            None => CacheStatus::NotCached,
        }
    }
    
    // 私有方法
    
    /// 生成缓存键
    fn generate_cache_key(&self, project_path: &Path) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        project_path.to_string_lossy().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
    
    /// 检查缓存是否过期
    fn is_cache_expired(&self, entry: &CacheEntry) -> bool {
        if let Ok(elapsed) = entry.created_at.elapsed() {
            elapsed > self.config.expiry_duration
        } else {
            true // 时间异常，认为已过期
        }
    }
    
    /// 获取项目最后修改时间（考虑 .gitignore 文件的变化）
    async fn get_project_last_modified(&self, project_path: &Path) -> Result<SystemTime> {
        let mut last_modified = fs::metadata(project_path).await?.modified()?;
        
        // 检查 .gitignore 文件的修改时间
        let gitignore_path = project_path.join(".gitignore");
        if gitignore_path.exists() {
            if let Ok(gitignore_metadata) = fs::metadata(&gitignore_path).await {
                if let Ok(gitignore_modified) = gitignore_metadata.modified() {
                    if gitignore_modified > last_modified {
                        last_modified = gitignore_modified;
                    }
                }
            }
        }
        
        // 也检查一些重要文件的修改时间
        let important_files = [
            "Cargo.toml", "package.json", "requirements.txt", "go.mod", "pom.xml"
        ];
        
        for file_name in &important_files {
            let file_path = project_path.join(file_name);
            if file_path.exists() {
                if let Ok(file_metadata) = fs::metadata(&file_path).await {
                    if let Ok(file_modified) = file_metadata.modified() {
                        if file_modified > last_modified {
                            last_modified = file_modified;
                        }
                    }
                }
            }
        }
        
        Ok(last_modified)
    }
    
    /// 清理旧的缓存条目
    async fn cleanup_old_entries(&mut self) {
        // 按创建时间排序，删除最旧的条目
        let mut entries: Vec<_> = self.cache_data.entries.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        entries.sort_by(|a, b| a.1.created_at.cmp(&b.1.created_at));
        
        let remove_count = self.cache_data.entries.len() - self.config.max_entries;
        for (key, _) in entries.iter().take(remove_count) {
            self.cache_data.entries.remove(key);
        }
    }
    
    /// 加载缓存文件
    async fn load_cache(&mut self) -> Result<()> {
        if !self.cache_file.exists() {
            return Ok(());
        }
        
        let content = fs::read_to_string(&self.cache_file).await?;
        
        match serde_json::from_str::<CacheData>(&content) {
            Ok(data) => {
                self.cache_data = data;
            }
            Err(_) => {
                // 缓存文件格式错误，重新开始
                self.cache_data = CacheData::default();
            }
        }
        
        Ok(())
    }
    
    /// 保存缓存文件
    async fn save_cache(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.cache_data)?;
        fs::write(&self.cache_file, content).await?;
        Ok(())
    }
    
    /// 获取缓存文件大小
    fn get_cache_file_size(&self) -> u64 {
        std::fs::metadata(&self.cache_file)
            .map(|m| m.len())
            .unwrap_or(0)
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 总缓存条目数
    pub total_entries: usize,
    
    /// 过期条目数
    pub expired_entries: usize,
    
    /// Git 仓库数量
    pub git_repositories: usize,
    
    /// 缓存的总项目大小
    pub total_cached_size: u64,
    
    /// 缓存的总代码大小
    pub total_code_size: u64,
    
    /// 缓存的总依赖大小
    pub total_dependency_size: u64,
    
    /// 缓存的总 gitignore 排除大小
    pub total_gitignore_size: u64,
    
    /// 缓存文件大小
    pub cache_file_size: u64,
    
    /// 最后更新时间
    pub last_updated: SystemTime,
}

/// 缓存状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheStatus {
    /// 有效缓存
    Valid,
    
    /// 缓存已过期
    Expired,
    
    /// 未缓存
    NotCached,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path();
        
        let config = CacheConfig {
            expiry_duration: Duration::from_secs(60),
            max_entries: 10,
            enabled: true,
        };
        
        let mut cache = SizeCache::new(config).await.unwrap();
        
        // 测试未缓存状态
        assert!(cache.get(project_path).await.is_none());
        assert_eq!(cache.check_cache_status(project_path), CacheStatus::NotCached);
        
        // 添加缓存
        let size_info = CachedSizeInfo {
            code_size: 1000,
            dependency_size: 2000,
            total_size: 3000,
            gitignore_excluded_size: 0,
            code_file_count: 10,
            dependency_file_count: 5,
            total_file_count: 15,
            gitignore_excluded_file_count: 0,
            last_modified: Some(SystemTime::now()),
        };
        
        cache.put(project_path, size_info.clone(), false).await.unwrap();
        
        // 测试缓存命中
        let cached = cache.get(project_path).await.unwrap();
        assert_eq!(cached.code_size, size_info.code_size);
        assert_eq!(cached.total_size, size_info.total_size);
        assert_eq!(cache.check_cache_status(project_path), CacheStatus::Valid);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path();
        
        let config = CacheConfig {
            expiry_duration: Duration::from_millis(100), // 100ms 过期
            max_entries: 10,
            enabled: true,
        };
        
        let mut cache = SizeCache::new(config).await.unwrap();
        
        let size_info = CachedSizeInfo {
            code_size: 1000,
            dependency_size: 0,
            total_size: 1000,
            gitignore_excluded_size: 0,
            code_file_count: 1,
            dependency_file_count: 0,
            total_file_count: 1,
            gitignore_excluded_file_count: 0,
            last_modified: Some(SystemTime::now()),
        };
        
        cache.put(project_path, size_info, false).await.unwrap();
        
        // 立即检查应该有效
        assert!(cache.get(project_path).await.is_some());
        
        // 等待过期
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // 现在应该过期了
        assert!(cache.get(project_path).await.is_none());
        assert_eq!(cache.check_cache_status(project_path), CacheStatus::Expired);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let temp_dir = tempdir().unwrap();
        
        // 创建一个使用临时目录的缓存配置
        let config = CacheConfig {
            expiry_duration: Duration::from_secs(60),
            max_entries: 10,
            enabled: true,
        };
        
        let cache_dir = temp_dir.path().join("cache");
        fs::create_dir_all(&cache_dir).await.unwrap();
        let cache_file = cache_dir.join("test_cache.json");
        
        let mut cache = SizeCache {
            cache_file,
            cache_data: CacheData::default(),
            config,
        };
        
        let stats = cache.get_stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.expired_entries, 0);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path();
        
        let config = CacheConfig::default();
        let mut cache = SizeCache::new(config).await.unwrap();
        
        let size_info = CachedSizeInfo {
            code_size: 1000,
            dependency_size: 0,
            total_size: 1000,
            gitignore_excluded_size: 0,
            code_file_count: 1,
            dependency_file_count: 0,
            total_file_count: 1,
            gitignore_excluded_file_count: 0,
            last_modified: Some(SystemTime::now()),
        };
        
        cache.put(project_path, size_info, false).await.unwrap();
        assert!(cache.get(project_path).await.is_some());
        
        cache.clear_all().await.unwrap();
        assert!(cache.get(project_path).await.is_none());
        assert_eq!(cache.get_stats().total_entries, 0);
    }
}