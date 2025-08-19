use std::time::Duration;
use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};
use anyhow::Result;
use tokio::sync::mpsc;

use crate::models::Project;

/// 应用程序事件枚举
#[derive(Clone, Debug)]
pub enum Event {
    /// 键盘输入事件
    Key(KeyEvent),
    
    /// 终端大小调整事件
    Resize(u16, u16),
    
    /// 扫描完成事件
    ScanComplete,
    
    /// 扫描进度更新
    ScanProgress(String),
    
    /// 发现新项目
    ProjectFound(Project),
    
    /// 项目大小更新事件
    ProjectSizeUpdated {
        project_index: usize,
        code_size: u64,
        total_size: u64,
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
}

impl EventHandler {
    /// 创建新的事件处理器
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            receiver,
            sender,
            handler: None,
        }
    }
    
    /// 启动事件监听
    pub fn start(&mut self) {
        let sender = self.sender.clone();
        
        self.handler = Some(tokio::spawn(async move {
            let mut tick_interval = tokio::time::interval(Duration::from_millis(250));
            
            loop {
                tokio::select! {
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
    
    /// 停止事件处理
    pub fn stop(&mut self) {
        if let Some(handle) = self.handler.take() {
            handle.abort();
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
    
    /// 检查是否是退出键 (Ctrl+C, Ctrl+D, ESC, q)
    pub fn is_quit_key(key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            KeyCode::Char('d') | KeyCode::Char('D') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            KeyCode::Esc => true,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_quit_keys() {
        assert!(keys::is_quit_key(&KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
        assert!(keys::is_quit_key(&KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
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
}