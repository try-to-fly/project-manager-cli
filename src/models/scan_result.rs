#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::models::Project;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// 扫描的项目列表
    pub projects: Vec<Project>,
    
    /// 扫描统计信息
    pub stats: ScanStats,
    
    /// 扫描开始时间
    pub scan_start_time: DateTime<Utc>,
    
    /// 扫描结束时间
    pub scan_end_time: Option<DateTime<Utc>>,
    
    /// 扫描的根路径
    pub scanned_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    /// 总的项目数量
    pub total_projects: usize,
    
    /// 按类型分组的项目数量
    pub projects_by_type: std::collections::HashMap<String, usize>,
    
    /// 总的代码大小
    pub total_code_size: u64,
    
    /// 总的依赖大小
    pub total_dependency_size: u64,
    
    /// 扫描的目录数量
    pub scanned_directories: usize,
    
    /// 跳过的目录数量
    pub skipped_directories: usize,
    
    /// 扫描耗时
    pub scan_duration: Option<Duration>,
    
    /// 最大的项目（按总大小）
    pub largest_project: Option<String>,
    
    /// 最大的依赖文件夹
    pub largest_dependency: Option<String>,
    
    /// Git 项目数量
    pub git_projects_count: usize,
    
    /// 有未提交更改的项目数量
    pub uncommitted_changes_count: usize,
}

impl ScanResult {
    /// 创建新的扫描结果
    pub fn new(scanned_paths: Vec<String>) -> Self {
        Self {
            projects: Vec::new(),
            stats: ScanStats::default(),
            scan_start_time: Utc::now(),
            scan_end_time: None,
            scanned_paths,
        }
    }
    
    /// 添加项目到扫描结果
    pub fn add_project(&mut self, project: Project) {
        self.projects.push(project);
        self.update_stats();
    }
    
    /// 完成扫描
    pub fn finish_scan(&mut self) {
        self.scan_end_time = Some(Utc::now());
        self.update_stats();
        
        if let Some(end_time) = self.scan_end_time {
            self.stats.scan_duration = Some(
                end_time.signed_duration_since(self.scan_start_time)
                    .to_std()
                    .unwrap_or_default()
            );
        }
    }
    
    /// 更新统计信息
    fn update_stats(&mut self) {
        let projects = &self.projects;
        
        self.stats.total_projects = projects.len();
        self.stats.total_code_size = projects.iter().map(|p| p.code_size).sum();
        self.stats.total_dependency_size = projects.iter().map(|p| p.dependency_size()).sum();
        
        // 按类型统计项目数量
        self.stats.projects_by_type.clear();
        for project in projects {
            let type_name = project.type_display_name();
            *self.stats.projects_by_type.entry(type_name).or_insert(0) += 1;
        }
        
        // Git 相关统计
        self.stats.git_projects_count = projects.iter()
            .filter(|p| p.git_info.is_some())
            .count();
        
        self.stats.uncommitted_changes_count = projects.iter()
            .filter(|p| p.git_info.as_ref()
                .map(|info| info.has_uncommitted_changes)
                .unwrap_or(false))
            .count();
        
        // 找出最大的项目
        self.stats.largest_project = projects.iter()
            .max_by_key(|p| p.total_size)
            .map(|p| p.name.clone());
        
        // 找出最大的依赖
        self.stats.largest_dependency = projects.iter()
            .flat_map(|p| &p.dependencies)
            .max_by_key(|d| d.size)
            .map(|d| format!("{} ({})", 
                d.path.display(), 
                d.dependency_type.display_name()));
    }
    
    /// 获取扫描耗时的友好显示
    pub fn scan_duration_display(&self) -> String {
        match self.stats.scan_duration {
            Some(duration) => {
                let seconds = duration.as_secs();
                if seconds < 60 {
                    format!("{}s", seconds)
                } else {
                    format!("{}m {}s", seconds / 60, seconds % 60)
                }
            }
            None => "进行中...".to_string(),
        }
    }
    
    /// 按不同条件过滤项目
    pub fn filter_projects<F>(&self, predicate: F) -> Vec<&Project>
    where
        F: Fn(&Project) -> bool,
    {
        self.projects.iter().filter(|p| predicate(p)).collect()
    }
    
    /// 获取所有 Git 项目
    pub fn git_projects(&self) -> Vec<&Project> {
        self.filter_projects(|p| p.git_info.is_some())
    }
    
    /// 获取有未提交更改的项目
    pub fn projects_with_uncommitted_changes(&self) -> Vec<&Project> {
        self.filter_projects(|p| {
            p.git_info.as_ref()
                .map(|info| info.has_uncommitted_changes)
                .unwrap_or(false)
        })
    }
    
    /// 获取大型项目（大于指定大小）
    pub fn large_projects(&self, min_size: u64) -> Vec<&Project> {
        self.filter_projects(|p| p.total_size > min_size)
    }
}

impl Default for ScanStats {
    fn default() -> Self {
        Self {
            total_projects: 0,
            projects_by_type: std::collections::HashMap::new(),
            total_code_size: 0,
            total_dependency_size: 0,
            scanned_directories: 0,
            skipped_directories: 0,
            scan_duration: None,
            largest_project: None,
            largest_dependency: None,
            git_projects_count: 0,
            uncommitted_changes_count: 0,
        }
    }
}