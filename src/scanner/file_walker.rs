use std::path::{Path, PathBuf};
use anyhow::Result;
use tokio::sync::mpsc;
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};

use crate::config::Config;
use crate::scanner::{ProjectDetector, DetectedProject};

/// 文件遍历器 - 负责扫描目录并发现项目
pub struct FileWalker {
    config: Config,
    project_detector: ProjectDetector,
    max_depth: Option<usize>,
    follow_symlinks: bool,
}

/// 扫描进度信息
#[derive(Debug, Clone)]
pub struct ScanProgress {
    /// 已扫描的目录数量
    pub scanned_dirs: usize,
    
    /// 跳过的目录数量
    pub skipped_dirs: usize,
    
    /// 发现的项目数量
    pub found_projects: usize,
    
    /// 当前扫描的路径
    pub current_path: Option<PathBuf>,
    
    /// 总的待扫描目录数（估算）
    pub total_dirs_estimate: Option<usize>,
}

impl Default for ScanProgress {
    fn default() -> Self {
        Self {
            scanned_dirs: 0,
            skipped_dirs: 0,
            found_projects: 0,
            current_path: None,
            total_dirs_estimate: None,
        }
    }
}

impl FileWalker {
    /// 创建新的文件遍历器
    pub fn new(config: Config) -> Self {
        Self {
            max_depth: config.scan.max_depth,
            follow_symlinks: config.scan.follow_symlinks,
            config,
            project_detector: ProjectDetector::new(),
        }
    }
    
    /// 扫描指定路径，返回发现的项目列表
    pub async fn scan_paths(&self, paths: &[String]) -> Result<Vec<DetectedProject>> {
        let (tx, mut rx) = mpsc::channel(1000);
        let mut projects = Vec::new();
        
        // 创建进度条（在测试时禁用）
        let progress = if cfg!(test) {
            ProgressBar::hidden()
        } else {
            self.create_progress_bar()
        };
        
        // 启动扫描任务
        let scan_task = {
            let paths = paths.to_vec();
            let tx = tx.clone();
            let walker = self.clone();
            tokio::spawn(async move {
                walker.scan_paths_internal(paths, tx).await
            })
        };
        
        // 接收扫描结果
        let mut scan_progress = ScanProgress::default();
        while let Some(result) = rx.recv().await {
            match result {
                ScanResult::Project(project) => {
                    scan_progress.found_projects += 1;
                    progress.set_message(format!(
                        "发现 {} 个项目 | 已扫描 {} 个目录", 
                        scan_progress.found_projects,
                        scan_progress.scanned_dirs
                    ));
                    projects.push(project);
                }
                ScanResult::Progress(new_progress) => {
                    scan_progress = new_progress;
                    progress.inc(1);
                    if let Some(current_path) = &scan_progress.current_path {
                        progress.set_message(format!(
                            "扫描: {} | 项目: {} | 目录: {}", 
                            current_path.display(),
                            scan_progress.found_projects,
                            scan_progress.scanned_dirs
                        ));
                    }
                }
                ScanResult::Error(err) => {
                    tracing::warn!("扫描时出错: {}", err);
                }
            }
        }
        
        // 等待扫描完成
        scan_task.await??;
        progress.finish_with_message(format!(
            "扫描完成！发现 {} 个项目，扫描了 {} 个目录", 
            projects.len(),
            scan_progress.scanned_dirs
        ));
        
        Ok(projects)
    }
    
    /// 创建进度条
    fn create_progress_bar(&self) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    }
    
    /// 内部扫描实现
    async fn scan_paths_internal(
        &self, 
        paths: Vec<String>, 
        tx: mpsc::Sender<ScanResult>
    ) -> Result<()> {
        let mut scan_progress = ScanProgress::default();
        
        for path_str in paths {
            let path = PathBuf::from(&path_str);
            if !path.exists() {
                tracing::warn!("路径不存在: {}", path.display());
                continue;
            }
            
            if path.is_dir() {
                self.scan_directory(&path, &mut scan_progress, &tx).await?;
            } else {
                tracing::warn!("不是目录: {}", path.display());
            }
        }
        
        Ok(())
    }
    
    /// 扫描单个目录
    async fn scan_directory(
        &self,
        root_path: &Path,
        progress: &mut ScanProgress,
        tx: &mpsc::Sender<ScanResult>,
    ) -> Result<()> {
        let mut walker = WalkDir::new(root_path);
        
        // 配置遍历选项
        if let Some(max_depth) = self.max_depth {
            walker = walker.max_depth(max_depth);
        }
        
        if !self.follow_symlinks {
            walker = walker.follow_links(false);
        }
        
        // 遍历目录
        for entry in walker.into_iter() {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    
                    // 更新当前扫描路径
                    progress.current_path = Some(path.to_path_buf());
                    
                    // 发送进度更新
                    if let Err(_) = tx.send(ScanResult::Progress(progress.clone())).await {
                        break; // 接收端已关闭
                    }
                    
                    if entry.file_type().is_dir() {
                        progress.scanned_dirs += 1;
                        
                        // 检查是否应该忽略此目录
                        if self.should_ignore_directory(path) {
                            progress.skipped_dirs += 1;
                            continue;
                        }
                        
                        // 检测是否是项目
                        if let Ok(Some(detected_project)) = self.project_detector.detect_project(path).await {
                            // 发送发现的项目
                            if let Err(_) = tx.send(ScanResult::Project(detected_project)).await {
                                break; // 接收端已关闭
                            }
                        }
                    }
                }
                Err(err) => {
                    // 发送错误
                    let _ = tx.send(ScanResult::Error(
                        anyhow::anyhow!("遍历目录时出错: {}", err)
                    )).await;
                }
            }
        }
        
        Ok(())
    }
    
    /// 检查是否应该忽略指定目录
    fn should_ignore_directory(&self, path: &Path) -> bool {
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return false,
        };
        
        // 检查是否在忽略目录列表中
        if self.config.ignore.directories.contains(file_name) {
            return true;
        }
        
        // 检查是否在忽略路径列表中
        let path_str = path.to_string_lossy();
        if self.config.ignore.paths.iter().any(|ignored_path| {
            path_str.contains(ignored_path)
        }) {
            return true;
        }
        
        // 检查是否在手动忽略的项目列表中
        if self.config.ignore.projects.contains(&path_str.to_string()) {
            return true;
        }
        
        // 不扫描隐藏目录（除非配置允许）
        if !self.config.scan.scan_hidden && file_name.starts_with('.') {
            // 但是要扫描 .git 目录的父目录
            if file_name != ".git" {
                return true;
            }
        }
        
        false
    }
    
    /// 获取扫描配置的克隆
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}

// 实现 Clone trait，以便在异步任务中使用
impl Clone for FileWalker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            project_detector: ProjectDetector::new(), // 重新创建检测器
            max_depth: self.max_depth,
            follow_symlinks: self.follow_symlinks,
        }
    }
}

/// 扫描结果枚举
#[derive(Debug)]
enum ScanResult {
    /// 发现的项目
    Project(DetectedProject),
    
    /// 进度更新
    Progress(ScanProgress),
    
    /// 扫描错误
    Error(anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_project_detector_basic() {
        let detector = ProjectDetector::new();
        
        let temp_dir = tempdir().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        
        // 创建一个简单的 Cargo.toml
        fs::write(&cargo_toml_path, r#"
[package]
name = "test-project"
version = "0.1.0"
description = "A test project"
"#).unwrap();
        
        let detected = detector.detect_project(temp_dir.path()).await.unwrap();
        
        assert!(detected.is_some());
        let project = detected.unwrap();
        assert_eq!(project.name, temp_dir.path().file_name().unwrap().to_str().unwrap());
        assert_eq!(project.description, Some("A test project".to_string()));
    }

    #[test]
    fn test_should_ignore_directory() {
        let mut config = Config::default();
        config.ignore.directories.insert("node_modules".to_string());
        config.scan.scan_hidden = true; // 允许扫描隐藏目录以便测试
        
        let walker = FileWalker::new(config);
        
        let temp_dir = tempdir().unwrap();
        let node_modules = temp_dir.path().join("node_modules");
        let normal_dir = temp_dir.path().join("src");
        
        assert!(walker.should_ignore_directory(&node_modules));
        assert!(!walker.should_ignore_directory(&normal_dir));
    }
}