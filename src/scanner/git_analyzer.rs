#![allow(dead_code)]

use std::path::Path;
use git2::{Repository, RepositoryState, Status};
use chrono::{DateTime, Utc};
use anyhow::Result;

use crate::models::GitInfo;

/// Git 仓库分析器 - 负责提取 Git 仓库的详细信息
pub struct GitAnalyzer;

impl GitAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    /// 分析指定路径的 Git 仓库，返回仓库信息
    pub fn analyze_repository(&self, repo_path: &Path) -> Result<Option<GitInfo>> {
        // 尝试打开 Git 仓库
        let repo = match Repository::discover(repo_path) {
            Ok(repo) => repo,
            Err(_) => return Ok(None), // 不是 Git 仓库
        };
        
        let mut git_info = GitInfo {
            remote_url: None,
            current_branch: None,
            last_commit_time: None,
            last_commit_message: None,
            last_commit_author: None,
            has_uncommitted_changes: false,
            has_unpushed_commits: false,
        };
        
        // 获取远程仓库 URL
        git_info.remote_url = self.get_remote_url(&repo)?;
        
        // 获取当前分支
        git_info.current_branch = self.get_current_branch(&repo)?;
        
        // 获取最后提交信息
        if let Some((time, message, author)) = self.get_last_commit_info(&repo)? {
            git_info.last_commit_time = Some(time);
            git_info.last_commit_message = Some(message);
            git_info.last_commit_author = Some(author);
        }
        
        // 检查是否有未提交的更改
        git_info.has_uncommitted_changes = self.has_uncommitted_changes(&repo)?;
        
        // 检查是否有未推送的提交
        git_info.has_unpushed_commits = self.has_unpushed_commits(&repo)?;
        
        Ok(Some(git_info))
    }
    
    /// 获取远程仓库 URL（通常是 origin）
    fn get_remote_url(&self, repo: &Repository) -> Result<Option<String>> {
        let remotes = repo.remotes()?;
        
        // 优先查找 origin 远程仓库
        for remote_name in remotes.iter() {
            if let Some(name) = remote_name {
                if name == "origin" {
                    if let Ok(remote) = repo.find_remote(name) {
                        if let Some(url) = remote.url() {
                            return Ok(Some(url.to_string()));
                        }
                    }
                }
            }
        }
        
        // 如果没有 origin，返回第一个远程仓库
        if let Some(first_remote) = remotes.iter().next() {
            if let Some(name) = first_remote {
                if let Ok(remote) = repo.find_remote(name) {
                    if let Some(url) = remote.url() {
                        return Ok(Some(url.to_string()));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// 获取当前分支名
    fn get_current_branch(&self, repo: &Repository) -> Result<Option<String>> {
        let head = match repo.head() {
            Ok(head) => head,
            Err(_) => return Ok(None), // 可能是一个新的空仓库
        };
        
        if let Some(branch_name) = head.shorthand() {
            Ok(Some(branch_name.to_string()))
        } else {
            Ok(None)
        }
    }
    
    /// 获取最后一次提交的信息
    fn get_last_commit_info(&self, repo: &Repository) -> Result<Option<(DateTime<Utc>, String, String)>> {
        let head = match repo.head() {
            Ok(head) => head,
            Err(_) => return Ok(None),
        };
        
        let commit = match head.peel_to_commit() {
            Ok(commit) => commit,
            Err(_) => return Ok(None),
        };
        
        // 获取提交时间
        let time = commit.time();
        let timestamp = time.seconds();
        let datetime = DateTime::from_timestamp(timestamp, 0)
            .unwrap_or_else(|| Utc::now());
        
        // 获取提交信息
        let message = commit.message()
            .unwrap_or("(无提交信息)")
            .lines()
            .next()
            .unwrap_or("(无提交信息)")
            .to_string();
        
        // 获取作者信息
        let author = commit.author();
        let author_name = format!(
            "{} <{}>",
            author.name().unwrap_or("(未知)"),
            author.email().unwrap_or("(未知)")
        );
        
        Ok(Some((datetime, message, author_name)))
    }
    
    /// 检查是否有未提交的更改
    fn has_uncommitted_changes(&self, repo: &Repository) -> Result<bool> {
        let statuses = repo.statuses(None)?;
        
        for status in statuses.iter() {
            let flags = status.status();
            
            // 检查是否有任何未提交的更改
            if flags.contains(Status::INDEX_NEW) ||
               flags.contains(Status::INDEX_MODIFIED) ||
               flags.contains(Status::INDEX_DELETED) ||
               flags.contains(Status::INDEX_RENAMED) ||
               flags.contains(Status::INDEX_TYPECHANGE) ||
               flags.contains(Status::WT_NEW) ||
               flags.contains(Status::WT_MODIFIED) ||
               flags.contains(Status::WT_DELETED) ||
               flags.contains(Status::WT_RENAMED) ||
               flags.contains(Status::WT_TYPECHANGE) {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// 检查是否有未推送的提交
    fn has_unpushed_commits(&self, repo: &Repository) -> Result<bool> {
        // 获取当前分支的HEAD
        let head = match repo.head() {
            Ok(head) => head,
            Err(_) => return Ok(false),
        };
        
        let local_commit = match head.peel_to_commit() {
            Ok(commit) => commit,
            Err(_) => return Ok(false),
        };
        
        // 获取当前分支名
        let branch_name = match head.shorthand() {
            Some(name) => name,
            None => return Ok(false),
        };
        
        // 尝试获取对应的远程分支
        let remote_branch_name = format!("origin/{}", branch_name);
        let remote_ref = match repo.find_reference(&format!("refs/remotes/{}", remote_branch_name)) {
            Ok(reference) => reference,
            Err(_) => {
                // 如果没有对应的远程分支，认为有未推送的提交
                return Ok(true);
            }
        };
        
        let remote_commit = match remote_ref.peel_to_commit() {
            Ok(commit) => commit,
            Err(_) => return Ok(true),
        };
        
        // 比较本地和远程的提交ID
        Ok(local_commit.id() != remote_commit.id())
    }
    
    /// 检查仓库状态
    pub fn get_repository_state(&self, repo_path: &Path) -> Result<Option<String>> {
        let repo = match Repository::discover(repo_path) {
            Ok(repo) => repo,
            Err(_) => return Ok(None),
        };
        
        let state = repo.state();
        let state_str = match state {
            RepositoryState::Clean => "clean",
            RepositoryState::Merge => "merging",
            RepositoryState::Revert => "reverting",
            RepositoryState::RevertSequence => "revert-sequence",
            RepositoryState::CherryPick => "cherry-picking",
            RepositoryState::CherryPickSequence => "cherry-pick-sequence",
            RepositoryState::Bisect => "bisecting",
            RepositoryState::Rebase => "rebasing",
            RepositoryState::RebaseInteractive => "rebasing-interactive",
            RepositoryState::RebaseMerge => "rebasing-merge",
            RepositoryState::ApplyMailbox => "applying-mailbox",
            RepositoryState::ApplyMailboxOrRebase => "applying-mailbox-or-rebase",
        };
        
        Ok(Some(state_str.to_string()))
    }
    
    /// 获取仓库的统计信息
    pub fn get_repository_stats(&self, repo_path: &Path) -> Result<Option<RepositoryStats>> {
        let repo = match Repository::discover(repo_path) {
            Ok(repo) => repo,
            Err(_) => return Ok(None),
        };
        
        let mut stats = RepositoryStats {
            total_commits: 0,
            total_branches: 0,
            total_tags: 0,
            total_remotes: 0,
        };
        
        // 统计提交数量（最近1000个）
        if let Ok(head) = repo.head() {
            if let Ok(commit) = head.peel_to_commit() {
                let mut revwalk = repo.revwalk()?;
                revwalk.push(commit.id())?;
                stats.total_commits = revwalk.count();
            }
        }
        
        // 统计分支数量
        let branches = repo.branches(Some(git2::BranchType::Local))?;
        stats.total_branches = branches.count();
        
        // 统计标签数量
        let tag_names = repo.tag_names(None)?;
        stats.total_tags = tag_names.len();
        
        // 统计远程仓库数量
        let remotes = repo.remotes()?;
        stats.total_remotes = remotes.len();
        
        Ok(Some(stats))
    }
}

/// Git 仓库统计信息
#[derive(Debug, Clone)]
pub struct RepositoryStats {
    /// 总提交数
    pub total_commits: usize,
    
    /// 总分支数
    pub total_branches: usize,
    
    /// 总标签数
    pub total_tags: usize,
    
    /// 总远程仓库数
    pub total_remotes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use git2::{Repository, Signature, Oid};

    #[test]
    fn test_analyze_non_git_directory() {
        let analyzer = GitAnalyzer::new();
        let temp_dir = tempdir().unwrap();
        
        let result = analyzer.analyze_repository(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_git_repository() {
        let analyzer = GitAnalyzer::new();
        let temp_dir = tempdir().unwrap();
        
        // 创建一个 Git 仓库
        let repo = Repository::init(temp_dir.path()).unwrap();
        
        // 创建一个文件并提交
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();
        
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        ).unwrap();
        
        let result = analyzer.analyze_repository(temp_dir.path()).unwrap();
        assert!(result.is_some());
        
        let git_info = result.unwrap();
        // 现代 Git 版本默认分支可能是 "main" 或 "master"
        assert!(git_info.current_branch == Some("main".to_string()) || 
                git_info.current_branch == Some("master".to_string()));
        assert!(git_info.last_commit_message.is_some());
        assert!(git_info.last_commit_author.is_some());
    }
}