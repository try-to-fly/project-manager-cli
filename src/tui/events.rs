#![allow(dead_code)]

use std::time::Duration;
use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use anyhow::Result;
use tokio::sync::mpsc;

use crate::models::{Project, GitInfo};
use crate::scanner::{ScanStage};

/// 应用程序事件枚举
#[derive(Clone, Debug)]
pub enum Event {
    /// 键盘输入事件
    Key(KeyEvent),
    
    /// 鼠标事件
    Mouse(MouseEvent),
    
    /// 终端大小调整事件
    Resize(u16, u16),
    
    /// 扫描完成事件
    ScanComplete,
    
    /// 扫描进度更新
    ScanProgress(String),
    
    /// 发现新项目
    ProjectFound(Project),
    
    /// 项目大小更新事件（已弃用，使用ProjectDetailsUpdated替代）
    ProjectSizeUpdated {
        project_index: usize,
        code_size: u64,
        total_size: u64,
        gitignore_excluded_size: u64,
        code_file_count: usize,
        dependency_file_count: usize,
        total_file_count: usize,
        gitignore_excluded_file_count: usize,
    },
    
    /// 项目详情更新事件
    ProjectDetailsUpdated {
        project_name: String,
        code_size: u64,
        dependency_size: u64,
        total_size: u64,
        gitignore_excluded_size: u64,
        code_file_count: usize,
        dependency_file_count: usize,
        total_file_count: usize,
        gitignore_excluded_file_count: usize,
        git_info: Option<GitInfo>,
    },
    
    /// 项目开始计算事件
    ProjectCalculationStarted {
        project_name: String,
    },
    
    /// 大小计算进度更新
    SizeCalculationProgress {
        project_name: String,
        processed_files: usize,
        total_files: Option<usize>,
        current_path: String,
        bytes_processed: u64,
        stage: ScanStage,
    },
    
    /// 应用程序退出
    Quit,
    
    /// 刷新项目列表
    Refresh,
    
    /// 定时刷新
    Tick,
}

/// 事件处理器 - 负责捕获和分发终端事件
pub struct EventHandler {
    /// 事件接收器
    receiver: mpsc::UnboundedReceiver<Event>,
    
    /// 事件发送器
    pub sender: mpsc::UnboundedSender<Event>,
    
    /// 事件处理任务句柄
    handler: Option<tokio::task::JoinHandle<()>>,
    
    /// 暂停信号发送器
    pause_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl EventHandler {
    /// 创建新的事件处理器
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            receiver,
            sender,
            handler: None,
            pause_sender: None,
        }
    }
    
    /// 启动事件监听
    pub fn start(&mut self) {
        let sender = self.sender.clone();
        let (pause_tx, pause_rx) = tokio::sync::oneshot::channel();
        self.pause_sender = Some(pause_tx);
        
        self.handler = Some(tokio::spawn(async move {
            let mut tick_interval = tokio::time::interval(Duration::from_millis(250));
            let mut pause_rx = pause_rx;
            
            loop {
                tokio::select! {
                    // 监听暂停信号
                    _ = &mut pause_rx => {
                        // 收到暂停信号，退出事件循环
                        break;
                    }
                    
                    // 处理定时器事件
                    _ = tick_interval.tick() => {
                        if sender.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    
                    // 处理终端事件
                    result = tokio::task::spawn_blocking(|| {
                        event::poll(Duration::from_millis(16))
                    }) => {
                        if let Ok(Ok(true)) = result {
                            if let Ok(event) = event::read() {
                                let app_event = match event {
                                    event::Event::Key(key) => Event::Key(key),
                                    event::Event::Mouse(mouse) => Event::Mouse(mouse),
                                    event::Event::Resize(w, h) => Event::Resize(w, h),
                                    _ => continue,
                                };
                                
                                if sender.send(app_event).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }));
    }
    
    /// 接收下一个事件
    pub async fn next(&mut self) -> Result<Event> {
        self.receiver.recv().await
            .ok_or_else(|| anyhow::anyhow!("事件通道已关闭"))
    }
    
    /// 发送自定义事件
    pub fn send(&self, event: Event) -> Result<()> {
        self.sender.send(event)
            .map_err(|_| anyhow::anyhow!("无法发送事件"))
    }
    
    /// 暂停事件处理（用于启动外部编辑器）
    pub fn pause(&mut self) {
        if let Some(pause_sender) = self.pause_sender.take() {
            let _ = pause_sender.send(()); // 发送暂停信号
        }
        // 不立即abort，让任务通过oneshot信号优雅退出
        // handle将在任务自然完成后自动清理
    }
    
    /// 恢复事件处理（从外部编辑器返回后）
    pub fn resume(&mut self) {
        // 重新启动事件处理
        self.start();
    }
    
    /// 停止事件处理
    pub fn stop(&mut self) {
        // 首先尝试通过信号优雅退出
        if let Some(pause_sender) = self.pause_sender.take() {
            let _ = pause_sender.send(());
        }
        
        // 如果有运行中的任务且无法通过信号停止，才使用abort
        if let Some(handle) = self.handler.take() {
            if !handle.is_finished() {
                handle.abort();
            }
        }
    }
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 键盘快捷键辅助函数
pub mod keys {
    use super::*;
    
    /// 检查是否是退出键 (Ctrl+C, Ctrl+D, q)
    pub fn is_quit_key(key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            KeyCode::Char('d') | KeyCode::Char('D') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            KeyCode::Char('q') | KeyCode::Char('Q') => true,
            _ => false,
        }
    }
    
    /// 检查是否是刷新键 (F5, Ctrl+R, r)
    pub fn is_refresh_key(key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::F(5) => true,
            KeyCode::Char('r') | KeyCode::Char('R') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            KeyCode::Char('r') | KeyCode::Char('R') => true,
            _ => false,
        }
    }
    
    /// 检查是否是向上导航键
    pub fn is_up_key(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K'))
    }
    
    /// 检查是否是向下导航键
    pub fn is_down_key(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J'))
    }
    
    /// 检查是否是确认键 (Enter, Space)
    pub fn is_enter_key(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Enter | KeyCode::Char(' '))
    }
    
    /// 检查是否是删除键 (Delete, d)
    pub fn is_delete_key(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Delete | KeyCode::Char('d') | KeyCode::Char('D'))
    }
    
    /// 检查是否是清理键 (c)
    pub fn is_clean_key(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
            && !key.modifiers.contains(KeyModifiers::CONTROL)
    }
    
    /// 检查是否是忽略键 (i)
    pub fn is_ignore_key(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Char('i') | KeyCode::Char('I'))
    }
    
    /// 检查是否是帮助键 (h, ?, F1)
    pub fn is_help_key(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') | KeyCode::F(1))
    }
    
    /// 检查是否是标签切换键 (Tab)
    pub fn is_tab_key(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Tab)
    }
    
    /// 检查是否是nvim编辑键 (e)
    pub fn is_nvim_key(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Char('e') | KeyCode::Char('E'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_quit_keys() {
        assert!(keys::is_quit_key(&KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
        assert!(!keys::is_quit_key(&KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(keys::is_quit_key(&KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)));
        assert!(!keys::is_quit_key(&KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)));
    }

    #[test]
    fn test_navigation_keys() {
        assert!(keys::is_up_key(&KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
        assert!(keys::is_up_key(&KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)));
        assert!(keys::is_down_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)));
        assert!(keys::is_down_key(&KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)));
    }

    #[test]
    fn test_nvim_key() {
        assert!(keys::is_nvim_key(&KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)));
        assert!(keys::is_nvim_key(&KeyEvent::new(KeyCode::Char('E'), KeyModifiers::NONE)));
        assert!(!keys::is_nvim_key(&KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)));
    }
}