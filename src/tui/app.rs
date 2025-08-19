use std::io;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, Paragraph},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    Frame,
};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{EnableMouseCapture, DisableMouseCapture},
    execute,
};
use anyhow::Result;

use crate::config::Config;
use crate::models::{Project, DependencyCalculationStatus};
use crate::scanner::FileWalker;

/// 统一的进度信息结构
#[derive(Clone, Debug)]
pub struct ProgressInfo {
    /// 进度类型
    pub progress_type: ProgressType,
    /// 当前进度
    pub current: usize,
    /// 总数（如果已知）
    pub total: Option<usize>,
    /// 当前处理的项目/文件名
    pub current_item: String,
    /// 额外的状态信息
    #[allow(dead_code)]
    pub extra_info: String,
}

/// 进度类型枚举
#[derive(Clone, Debug, PartialEq)]
pub enum ProgressType {
    /// 空闲状态
    Idle,
    /// 扫描项目
    Scanning,
    /// 发现文件
    Discovering,
    /// 计算大小
    Calculating,
}

impl Default for ProgressInfo {
    fn default() -> Self {
        Self {
            progress_type: ProgressType::Idle,
            current: 0,
            total: None,
            current_item: String::new(),
            extra_info: String::new(),
        }
    }
}

impl ProgressInfo {
    /// 格式化为显示字符串
    pub fn format_display(&self) -> String {
        match self.progress_type {
            ProgressType::Idle => String::new(),
            ProgressType::Scanning => {
                format!("扫描中: {} 个项目", self.current)
            }
            ProgressType::Discovering => {
                if let Some(total) = self.total {
                    format!("发现文件: {}/{}", self.current, total)
                } else {
                    format!("发现文件: {}", self.current)
                }
            }
            ProgressType::Calculating => {
                let item_display = if self.current_item.len() > 15 {
                    format!("{}...", &self.current_item[..12])
                } else {
                    self.current_item.clone()
                };
                
                if let Some(total) = self.total {
                    let percentage = if total > 0 { (self.current * 100) / total } else { 0 };
                    format!("计算 {}: {}/{} ({}%)", item_display, self.current, total, percentage)
                } else {
                    format!("计算 {}: {} 文件", item_display, self.current)
                }
            }
        }
    }
}
use crate::tui::events::{Event, EventHandler, keys};
use crate::tui::screens::MainScreen;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use std::collections::HashMap;

/// 应用程序状态
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// 启动中
    Starting,
    
    /// 扫描项目中
    Scanning,
    
    /// 显示项目列表
    ProjectList,
    
    /// 显示项目详情
    ProjectDetail,
    
    /// 显示帮助信息
    Help,
    
    /// 确认对话框
    ConfirmDialog,
    
    /// 外部编辑器状态
    ExternalEditor,
    
    /// 错误状态
    #[allow(dead_code)]
    Error(String),
    
    /// 退出中
    Quitting,
}

/// 主应用程序
pub struct App {
    /// 应用配置
    #[allow(dead_code)]
    config: Config,
    
    /// 当前状态
    state: AppState,
    
    /// 项目列表
    projects: Vec<Project>,
    
    /// 当前选中的项目索引
    selected_project: usize,
    
    /// 扫描的路径
    scan_paths: Vec<String>,
    
    /// 状态消息
    status_message: String,
    
    /// 是否显示详细信息
    #[allow(dead_code)]
    show_details: bool,
    
    /// 当前视图标签
    current_tab: TabView,
    
    /// 扫描进度信息
    scan_progress: String,
    
    /// 统一的进度信息状态
    progress_info: ProgressInfo,
    
    /// 事件处理器
    event_handler: EventHandler,
    
    /// 主屏幕
    main_screen: MainScreen,
    
    /// 项目计算任务管理
    calculation_tasks: HashMap<String, JoinHandle<()>>,
    
    /// 最大并发任务数限制
    #[allow(dead_code)]
    max_concurrent_tasks: usize,
    
    /// 取消令牌，用于优雅退出任务
    cancellation_token: CancellationToken,
}

/// 视图标签
#[derive(Debug, Clone, PartialEq)]
pub enum TabView {
    /// 项目列表
    Projects,
    
    /// 统计信息
    Statistics,
    
    /// Git 状态
    GitStatus,
}

impl App {
    /// 创建新的应用程序
    pub fn new(config: Config, scan_paths: Vec<String>) -> Self {
        Self {
            config,
            state: AppState::Starting,
            projects: Vec::new(),
            selected_project: 0,
            scan_paths,
            status_message: "正在启动...".to_string(),
            show_details: false,
            current_tab: TabView::Projects,
            scan_progress: String::new(),
            progress_info: ProgressInfo::default(),
            event_handler: EventHandler::new(),
            main_screen: MainScreen::new(),
            calculation_tasks: HashMap::new(),
            max_concurrent_tasks: 10, // 限制最多10个并发计算任务
            cancellation_token: CancellationToken::new(),
        }
    }
    
    /// 运行应用程序
    pub async fn run(&mut self) -> Result<()> {
        // 设置终端
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        // 启动事件处理
        self.event_handler.start();
        
        // 开始扫描
        self.start_scan().await?;
        
        // 主事件循环
        let result = self.main_loop(&mut terminal).await;
        
        // 清理所有运行中的任务
        self.cleanup_all_tasks().await;
        
        // 恢复终端
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        terminal.show_cursor()?;
        
        result
    }
    
    /// 主事件循环
    async fn main_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let mut needs_redraw = true; // 首次绘制
        let mut last_redraw = std::time::Instant::now();
        let min_redraw_interval = std::time::Duration::from_millis(16); // 约60fps
        
        loop {
            // 只在需要时重绘，且限制重绘频率
            if needs_redraw && last_redraw.elapsed() >= min_redraw_interval {
                terminal.draw(|f| self.draw(f))?;
                needs_redraw = false;
                last_redraw = std::time::Instant::now();
                
                // 定期清理已完成的任务
                self.cleanup_finished_tasks();
            }
            
            // 处理事件
            match self.event_handler.next().await? {
                Event::Key(key) => {
                    if keys::is_quit_key(&key) {
                        self.state = AppState::Quitting;
                        break;
                    }
                    
                    let force_redraw = self.handle_key_event(key).await?;
                    needs_redraw = true;
                    if force_redraw {
                        // 清除屏幕并强制立即重绘
                        terminal.clear()?;
                        terminal.draw(|f| self.draw(f))?;
                        last_redraw = std::time::Instant::now();
                        needs_redraw = false;
                    }
                }
                Event::Mouse(mouse) => {
                    self.handle_mouse_event(mouse).await?;
                    needs_redraw = true;
                }
                Event::Resize(_w, _h) => {
                    // 终端大小调整需要重绘
                    needs_redraw = true;
                }
                Event::ScanComplete => {
                    self.state = AppState::ProjectList;
                    self.status_message = format!("扫描完成！发现 {} 个项目", self.projects.len());
                    // 清理扫描进度状态
                    self.progress_info = ProgressInfo::default();
                    self.scan_progress.clear();
                    needs_redraw = true;
                }
                Event::ScanProgress(progress) => {
                    // 更新扫描进度信息
                    if progress.contains("扫描完成") {
                        self.progress_info.progress_type = ProgressType::Idle;
                    } else {
                        self.progress_info.progress_type = ProgressType::Scanning;
                        if let Some(num_start) = progress.find("发现 ") {
                            if let Some(num_end) = progress[num_start + 3..].find(" 个") {
                                let num_str = &progress[num_start + 3..num_start + 3 + num_end];
                                if let Ok(count) = num_str.parse::<usize>() {
                                    self.progress_info.current = count;
                                }
                            }
                        }
                    }
                    self.scan_progress = progress;
                    needs_redraw = true;
                }
                Event::ProjectFound(project) => {
                    self.projects.push(project);
                    needs_redraw = true;
                }
                Event::ProjectSizeUpdated { 
                    project_index, 
                    code_size, 
                    total_size, 
                    gitignore_excluded_size,
                    code_file_count,
                    dependency_file_count,
                    total_file_count,
                    gitignore_excluded_file_count,
                } => {
                    // 更新指定项目的大小信息
                    if let Some(project) = self.projects.get_mut(project_index) {
                        project.code_size = code_size;
                        project.total_size = total_size;
                        project.gitignore_excluded_size = gitignore_excluded_size;
                        project.code_file_count = code_file_count;
                        project.dependency_file_count = dependency_file_count;
                        project.total_file_count = total_file_count;
                        project.gitignore_excluded_file_count = gitignore_excluded_file_count;
                    }
                    needs_redraw = true;
                }
                Event::ProjectDetailsUpdated {
                    project_name,
                    code_size,
                    dependency_size,
                    total_size,
                    gitignore_excluded_size,
                    code_file_count,
                    dependency_file_count,
                    total_file_count,
                    gitignore_excluded_file_count,
                    git_info,
                } => {
                    // 找到对应的项目并更新其详细信息
                    if let Some(project) = self.projects.iter_mut().find(|p| p.name == project_name) {
                        project.code_size = code_size;
                        project.total_size = total_size;
                        project.gitignore_excluded_size = gitignore_excluded_size;
                        project.code_file_count = code_file_count;
                        project.dependency_file_count = dependency_file_count;
                        project.total_file_count = total_file_count;
                        project.gitignore_excluded_file_count = gitignore_excluded_file_count;
                        project.git_info = git_info;
                        project.cached_dependency_size = Some(dependency_size); // 更新缓存的依赖大小
                        project.dependency_calculation_status = DependencyCalculationStatus::Completed;
                    }
                    needs_redraw = true;
                }
                Event::ProjectCalculationStarted { project_name } => {
                    // 找到对应的项目并标记为计算中状态
                    if let Some(project) = self.projects.iter_mut().find(|p| p.name == project_name) {
                        project.dependency_calculation_status = DependencyCalculationStatus::Calculating;
                    }
                    needs_redraw = true;
                }
                Event::Refresh => {
                    self.start_scan().await?;
                    needs_redraw = true;
                }
                Event::Quit => {
                    break;
                }
                Event::SizeCalculationProgress { 
                    project_name,
                    processed_files,
                    total_files,
                    current_path: _,
                    bytes_processed: _,
                    stage,
                } => {
                    // 更新统一进度信息
                    self.progress_info.progress_type = match stage {
                        crate::scanner::ScanStage::Discovery => ProgressType::Discovering,
                        crate::scanner::ScanStage::Metadata | crate::scanner::ScanStage::Calculation => ProgressType::Calculating,
                        crate::scanner::ScanStage::Completed => ProgressType::Idle,
                    };
                    self.progress_info.current = processed_files;
                    self.progress_info.total = total_files;
                    self.progress_info.current_item = project_name.clone();
                    
                    // 保持旧的扫描进度信息作为后备（兼容性）
                    if stage == crate::scanner::ScanStage::Completed {
                        self.scan_progress.clear();
                    } else {
                        self.scan_progress = format!(
                            "计算 {} 进度: {}/{} 文件 - {}",
                            project_name,
                            processed_files,
                            total_files.map(|t| t.to_string()).unwrap_or("?".to_string()),
                            match stage {
                                crate::scanner::ScanStage::Discovery => "发现文件",
                                crate::scanner::ScanStage::Metadata => "计算大小",
                                crate::scanner::ScanStage::Calculation => "分析结果",
                                crate::scanner::ScanStage::Completed => "完成",
                            }
                        );
                    }
                    needs_redraw = true;
                }
                Event::Tick => {
                    // 定时更新，不需要每次都重绘
                    // 只有在有变化时才需要重绘
                }
            }
            
            if self.state == AppState::Quitting {
                break;
            }
        }
        
        Ok(())
    }
    
    /// 处理鼠标事件
    async fn handle_mouse_event(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
        use crossterm::event::{MouseEventKind, MouseButton};
        
        match self.state {
            AppState::ProjectList => {
                self.handle_project_list_mouse(mouse).await?;
            }
            AppState::ProjectDetail => {
                // 在项目详情页面，点击任意地方返回列表
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    self.state = AppState::ProjectList;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// 处理键盘事件（返回true表示需要强制重绘）
    async fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        let mut needs_redraw = false;
        
        match self.state {
            AppState::ProjectList => {
                needs_redraw = self.handle_project_list_keys(key).await?;
            }
            AppState::ProjectDetail => {
                self.handle_project_detail_keys(key).await?;
            }
            AppState::Help => {
                if keys::is_enter_key(&key) || keys::is_help_key(&key) {
                    self.state = AppState::ProjectList;
                }
            }
            AppState::ConfirmDialog => {
                self.handle_confirm_dialog_keys(key).await?;
            }
            AppState::ExternalEditor => {
                // 在外部编辑器状态下，不处理任何键盘事件
                // 事件处理将在spawn_nvim方法中完成后自动恢复
            }
            _ => {}
        }
        
        Ok(needs_redraw)
    }
    
    /// 处理项目列表键盘事件（返回true表示需要强制重绘）
    async fn handle_project_list_keys(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        if keys::is_up_key(&key) && self.selected_project > 0 {
            self.selected_project -= 1;
        } else if keys::is_down_key(&key) && self.selected_project < self.projects.len().saturating_sub(1) {
            self.selected_project += 1;
        } else if keys::is_enter_key(&key) {
            self.state = AppState::ProjectDetail;
        } else if keys::is_refresh_key(&key) {
            self.start_scan().await?;
        } else if keys::is_help_key(&key) {
            self.state = AppState::Help;
        } else if keys::is_tab_key(&key) {
            self.switch_tab();
        } else if keys::is_delete_key(&key) {
            if !self.projects.is_empty() {
                self.state = AppState::ConfirmDialog;
                self.status_message = "确认删除选中的项目？ (y/N)".to_string();
            }
        } else if keys::is_clean_key(&key) {
            if !self.projects.is_empty() {
                self.clean_current_project().await?;
            }
        } else if keys::is_ignore_key(&key) {
            if !self.projects.is_empty() {
                self.toggle_ignore_project().await?;
            }
        } else if keys::is_nvim_key(&key) {
            if !self.projects.is_empty() {
                if let Some(project) = self.projects.get(self.selected_project) {
                    let project_path = project.path.clone();
                    return self.spawn_nvim(&project_path).await;
                }
            }
        }
        
        Ok(false)
    }
    
    /// 处理项目列表鼠标事件
    async fn handle_project_list_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
        use crossterm::event::{MouseEventKind, MouseButton};
        
        match mouse.kind {
            // 左键点击
            MouseEventKind::Down(MouseButton::Left) => {
                // 检查是否点击在标签栏区域 (前3行)
                if mouse.row <= 2 {
                    self.handle_tab_click(mouse.column);
                }
                // 检查是否点击在项目列表区域 (第4行开始，考虑表头)
                else if mouse.row >= 5 && !self.projects.is_empty() {
                    // 获取当前表格的滚动偏移量
                    let scroll_offset = self.main_screen.get_table_offset();
                    
                    // 计算点击的项目索引（减去标签栏和表头的行数，加上滚动偏移）
                    let clicked_row_in_view = mouse.row as usize - 5; // 3行标签栏 + 2行表头边框
                    let clicked_project_index = clicked_row_in_view + scroll_offset;
                    
                    // 确保索引在有效范围内
                    if clicked_project_index < self.projects.len() {
                        self.selected_project = clicked_project_index;
                    }
                }
            }
            // 双击功能需要自己实现，crossterm 没有直接的 DoubleClick 事件
            // 暂时移除双击功能，可以通过键盘 Enter 进入详情
            // 滚轮滚动
            MouseEventKind::ScrollUp => {
                if self.selected_project > 0 {
                    self.selected_project -= 1;
                }
            }
            MouseEventKind::ScrollDown => {
                if self.selected_project < self.projects.len().saturating_sub(1) {
                    self.selected_project += 1;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// 处理标签栏点击
    fn handle_tab_click(&mut self, column: u16) {
        // 简单的标签点击检测，基于列位置
        let tab_width = 20; // 假设每个标签大约20个字符宽
        let tab_index = (column / tab_width) as usize;
        
        match tab_index {
            0 => self.current_tab = TabView::Projects,
            1 => self.current_tab = TabView::Statistics,
            2 => self.current_tab = TabView::GitStatus,
            _ => {} // 超出范围的点击忽略
        }
    }
    
    /// 处理项目详情键盘事件
    async fn handle_project_detail_keys(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        if keys::is_enter_key(&key) || matches!(key.code, crossterm::event::KeyCode::Backspace) {
            self.state = AppState::ProjectList;
        }
        
        Ok(())
    }
    
    /// 处理确认对话框键盘事件
    async fn handle_confirm_dialog_keys(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => {
                self.delete_current_project().await?;
                self.state = AppState::ProjectList;
            }
            _ => {
                self.state = AppState::ProjectList;
                self.status_message = "操作已取消".to_string();
            }
        }
        
        Ok(())
    }
    
    /// 绘制界面
    fn draw(&mut self, f: &mut Frame) {
        let full_area = f.area();
        
        // 为底部单行状态栏留出空间
        let main_area = Rect {
            x: full_area.x,
            y: full_area.y,
            width: full_area.width,
            height: full_area.height.saturating_sub(1), // 减去一行状态栏的高度
        };
        
        match self.state {
            AppState::Starting => {
                self.draw_loading_screen(f, main_area);
            }
            AppState::Scanning => {
                self.draw_scanning_screen(f, main_area);
            }
            AppState::ProjectList => {
                self.main_screen.draw_project_list(f, main_area, &self.projects, self.selected_project, &self.current_tab);
            }
            AppState::ProjectDetail => {
                if let Some(project) = self.projects.get(self.selected_project) {
                    self.main_screen.draw_project_detail(f, main_area, project);
                }
            }
            AppState::Help => {
                self.draw_help_screen(f, main_area);
            }
            AppState::ConfirmDialog => {
                self.main_screen.draw_project_list(f, main_area, &self.projects, self.selected_project, &self.current_tab);
                self.draw_confirm_dialog(f, main_area);
            }
            AppState::ExternalEditor => {
                // 在外部编辑器状态下，显示空屏幕或者保持最后的界面
                // 由于实际上此时终端被nvim接管，这个状态可能不会被渲染
                self.draw_loading_screen(f, main_area);
            }
            AppState::Error(ref error) => {
                self.draw_error_screen(f, main_area, error);
            }
            _ => {}
        }
        
        // 绘制状态栏（使用完整区域）
        self.draw_status_bar(f, full_area);
    }
    
    /// 绘制加载屏幕
    fn draw_loading_screen(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("项目管理器")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        
        let paragraph = Paragraph::new("正在启动应用程序...")
            .block(block)
            .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, area);
    }
    
    /// 绘制扫描屏幕
    fn draw_scanning_screen(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("扫描项目")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow));
        
        let text = vec![
            Line::from("正在扫描项目..."),
            Line::from(""),
            Line::from(self.scan_progress.clone()),
        ];
        
        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, area);
    }
    
    /// 绘制帮助屏幕
    fn draw_help_screen(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("帮助信息")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));
        
        let help_text = vec![
            Line::from(vec![
                Span::styled("快捷键:", Style::default().add_modifier(Modifier::BOLD))
            ]),
            Line::from(""),
            Line::from("  q, Ctrl+C       - 退出应用程序"),
            Line::from("  r, F5           - 刷新项目列表"),
            Line::from("  h, ?, F1        - 显示帮助信息"),
            Line::from("  Tab             - 切换视图标签"),
            Line::from(""),
            Line::from("  ↑/↓, k/j        - 导航项目列表"),
            Line::from("  Enter, Space    - 查看项目详情"),
            Line::from("  d, Delete       - 删除项目"),
            Line::from("  c               - 清理项目依赖"),
            Line::from("  i               - 切换忽略状态"),
            Line::from("  e               - 使用nvim编辑项目"),
            Line::from(""),
            Line::from(vec![
                Span::styled("鼠标操作:", Style::default().add_modifier(Modifier::BOLD))
            ]),
            Line::from(""),
            Line::from("  点击            - 选择项目"),
            Line::from("  滚轮            - 滚动项目列表"),
            Line::from("  点击标签        - 切换视图"),
            Line::from(""),
            Line::from("按 Enter 或 h 返回项目列表"),
        ];
        
        let paragraph = Paragraph::new(help_text)
            .block(block)
            .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, area);
    }
    
    /// 绘制确认对话框
    fn draw_confirm_dialog(&self, f: &mut Frame, area: Rect) {
        let popup_area = self.centered_rect(50, 20, area);
        
        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("确认操作")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Red));
        
        let text = vec![
            Line::from(""),
            Line::from("确认删除选中的项目？"),
            Line::from(""),
            Line::from("按 'y' 确认，按任意键取消"),
        ];
        
        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, popup_area);
    }
    
    /// 绘制错误屏幕
    fn draw_error_screen(&self, f: &mut Frame, area: Rect, error: &str) {
        let block = Block::default()
            .title("错误")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Red));
        
        let paragraph = Paragraph::new(error)
            .block(block)
            .style(Style::default().fg(Color::White));
        
        f.render_widget(paragraph, area);
    }
    
    /// 绘制优化的单行状态栏（左侧状态，右侧进度）
    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_area = Rect {
            x: area.x,
            y: area.y + area.height - 1, // 单行状态栏
            width: area.width,
            height: 1,
        };
        
        // 构建左侧状态信息
        let left_status_text = match self.state {
            AppState::ProjectList => {
                let calculating_count = self.calculation_tasks.len();
                format!("{} | 项目: {} | 选中: {}/{}{}", 
                    self.status_message,
                    self.projects.len(),
                    if self.projects.is_empty() { 0 } else { self.selected_project + 1 },
                    self.projects.len(),
                    if calculating_count > 0 { format!(" | 计算中: {}", calculating_count) } else { String::new() }
                )
            }
            _ => self.status_message.clone(),
        };
        
        // 构建右侧进度信息 - 优先显示统一进度，然后是扫描进度，最后是快捷键提示
        let right_progress_text = {
            let progress_display = self.progress_info.format_display();
            if !progress_display.is_empty() {
                progress_display
            } else if !self.scan_progress.is_empty() {
                self.scan_progress.clone()
            } else {
                "快捷键: ↑/↓ 选择 | Enter 详情 | d 删除 | c 清理 | i 忽略 | e 编辑 | r 刷新 | q 退出".to_string()
            }
        };
        
        // 计算布局：左侧状态信息，右侧进度信息
        let left_width = left_status_text.len() as u16;
        let right_width = right_progress_text.len() as u16;
        let total_width = status_area.width;
        
        // 检查是否有进度信息需要高亮显示
        let has_progress = self.progress_info.progress_type != ProgressType::Idle || !self.scan_progress.is_empty();
        
        // 创建组合文本，中间用空格填充
        let combined_text = if left_width + right_width + 3 <= total_width {
            // 有足够空间显示两边内容
            let padding = total_width - left_width - right_width;
            format!("{}{}{}", left_status_text, " ".repeat(padding as usize), right_progress_text)
        } else {
            // 空间不够，优先显示左侧状态，截断右侧
            if left_width + 3 < total_width {
                let remaining = total_width - left_width - 3;
                let truncated_right = if right_width <= remaining {
                    right_progress_text
                } else {
                    format!("{}...", &right_progress_text[..remaining.saturating_sub(3) as usize])
                };
                format!("{}   {}", left_status_text, truncated_right)
            } else {
                // 连左侧都显示不下，只显示左侧并截断
                if left_width <= total_width {
                    left_status_text
                } else {
                    format!("{}...", &left_status_text[..total_width.saturating_sub(3) as usize])
                }
            }
        };
        
        // 根据是否有进度信息选择不同的颜色方案
        let status_style = if has_progress {
            // 有进度时使用更明显的背景色
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            // 无进度时使用较暗的背景色
            Style::default().bg(Color::Black).fg(Color::Gray)
        };
        
        // 创建状态栏
        let status = Paragraph::new(combined_text).style(status_style);
        
        f.render_widget(status, status_area);
    }
    
    /// 计算居中的矩形区域
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);
        
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
    
    /// 开始扫描项目
    async fn start_scan(&mut self) -> Result<()> {
        self.state = AppState::Scanning;
        self.status_message = "正在扫描项目...".to_string();
        self.projects.clear();
        
        // 初始化扫描进度状态
        self.progress_info = ProgressInfo {
            progress_type: ProgressType::Scanning,
            current: 0,
            total: None,
            current_item: String::new(),
            extra_info: String::new(),
        };
        
        // 使用最简单的扫描方式：直接遍历目录查找项目标识文件
        let scan_paths = self.scan_paths.clone();
        for path_str in &scan_paths {
            let path = std::path::Path::new(path_str);
            
            if !path.exists() || !path.is_dir() {
                continue;
            }
            
            self.scan_directory_simple(path).await?;
        }
        
        // 扫描完成，开始异步计算大小
        self.state = AppState::ProjectList;
        self.status_message = format!("扫描完成！发现 {} 个项目", self.projects.len());
        
        // 启动异步大小计算任务
        self.start_async_size_calculation().await?;
        
        Ok(())
    }
    
    /// 简单扫描目录
    async fn scan_directory_simple(&mut self, dir: &std::path::Path) -> Result<()> {
        use tokio::fs;
        use std::collections::VecDeque;
        
        let mut queue = VecDeque::new();
        queue.push_back(dir.to_path_buf());
        let mut scanned_count = 0;
        
        while let Some(current_dir) = queue.pop_front() {
            scanned_count += 1;
            
            // 限制扫描深度和数量，避免卡死
            if scanned_count > 1000 {
                break;
            }
            
            // 跳过大型目录
            if let Some(dir_name) = current_dir.file_name().and_then(|n| n.to_str()) {
                if matches!(dir_name, "node_modules" | ".git" | "target" | "dist" | "build" | "venv" | ".venv") {
                    continue;
                }
            }
            
            // 检查是否是项目
            if self.is_project_directory(&current_dir).await {
                let project_name = current_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                
                let project = Project {
                    name: project_name.clone(),
                    path: current_dir.clone(),
                    project_type: self.detect_project_type(&current_dir).await,
                    code_size: 0,
                    total_size: 0,
                    gitignore_excluded_size: 0,
                    code_file_count: 0,
                    dependency_file_count: 0,
                    total_file_count: 0,
                    gitignore_excluded_file_count: 0,
                    last_modified: chrono::Utc::now(),
                    git_info: None,
                    dependencies: Vec::new(),
                    is_ignored: false,
                    description: None,
                    dependency_calculation_status: DependencyCalculationStatus::NotCalculated,
                    cached_dependency_size: None,
                };
                
                self.projects.push(project.clone());
                
                // 为当前目录项目也启动异步计算详细信息
                let project_path = current_dir.clone();
                let project_name = project.name.clone();
                let sender = self.event_handler.sender.clone();
                
                let cancel_token = self.cancellation_token.clone();
                tokio::spawn(async move {
                    // 先发送开始计算事件
                    let _ = sender.send(Event::ProjectCalculationStarted {
                        project_name: project_name.clone(),
                    });
                    
                    Self::calculate_project_details(project_path, project_name, sender, cancel_token).await;
                });
                
                // 发现项目后不再扫描其子目录
                continue;
            }
            
            // 扫描子目录
            if let Ok(mut entries) = fs::read_dir(&current_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(metadata) = entry.metadata().await {
                        if metadata.is_dir() {
                            queue.push_back(entry.path());
                        }
                    }
                }
            }
            
            // 防止界面卡死，定期让出控制权
            if scanned_count % 10 == 0 {
                tokio::task::yield_now().await;
            }
        }
        
        Ok(())
    }
    
    /// 检查目录是否是项目
    async fn is_project_directory(&self, dir: &std::path::Path) -> bool {
        let project_files = [
            "package.json",    // Node.js
            "Cargo.toml",      // Rust
            "pyproject.toml",  // Python
            "requirements.txt", // Python
            "go.mod",          // Go
            "pom.xml",         // Java Maven
            "build.gradle",    // Java Gradle
            "CMakeLists.txt",  // C++
            ".git",            // Git repo
        ];
        
        for file in &project_files {
            if dir.join(file).exists() {
                return true;
            }
        }
        
        false
    }
    
    /// 检测项目类型
    async fn detect_project_type(&self, dir: &std::path::Path) -> crate::models::ProjectType {
        use crate::models::ProjectType;
        
        if dir.join("package.json").exists() {
            ProjectType::NodeJs
        } else if dir.join("Cargo.toml").exists() {
            ProjectType::Rust
        } else if dir.join("pyproject.toml").exists() || dir.join("requirements.txt").exists() {
            ProjectType::Python
        } else if dir.join("go.mod").exists() {
            ProjectType::Go
        } else if dir.join("pom.xml").exists() || dir.join("build.gradle").exists() {
            ProjectType::Java
        } else if dir.join("CMakeLists.txt").exists() {
            ProjectType::Cpp
        } else if dir.join(".git").exists() {
            ProjectType::Git
        } else {
            ProjectType::Unknown
        }
    }
    
    /// 启动异步大小计算任务
    async fn start_async_size_calculation(&mut self) -> Result<()> {
        let projects_for_calc = self.projects.clone();
        let sender = self.event_handler.sender.clone();
        
        // 在后台异步计算每个项目的大小
        tokio::spawn(async move {
            use crate::scanner::SizeCalculator;
            use crate::config::Config;
            
            // 加载配置并创建带缓存的大小计算器
            let config = Config::load_or_create_default().unwrap_or_default();
            let mut size_calculator = SizeCalculator::new_with_cache(config.cache.to_size_cache_config())
                .await
                .unwrap_or_else(|_| SizeCalculator::new());
            
            for (index, project) in projects_for_calc.iter().enumerate() {
                // 计算项目大小
                if let Ok(size_info) = size_calculator.calculate_project_size(&project.path).await {
                    // 发送更新事件
                    let _ = sender.send(Event::ProjectSizeUpdated {
                        project_index: index,
                        code_size: size_info.code_size,
                        total_size: size_info.total_size,
                        gitignore_excluded_size: size_info.gitignore_excluded_size,
                        code_file_count: size_info.code_file_count,
                        dependency_file_count: size_info.dependency_file_count,
                        total_file_count: size_info.total_file_count,
                        gitignore_excluded_file_count: size_info.gitignore_excluded_file_count,
                    });
                }
                
                // 防止计算过快导致界面更新频繁
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        });
        
        Ok(())
    }
    
    /// 异步扫描项目
    #[allow(dead_code)]
    async fn scan_projects_async(
        paths: Vec<String>, 
        config: Config, 
        progress_sender: mpsc::UnboundedSender<Event>
    ) -> Result<Vec<Project>> {
                
        tracing::info!("开始异步扫描项目，路径: {:?}", paths);
        
        let mut all_projects = Vec::new();
        let file_walker = FileWalker::new(config.clone());
        
        for path in paths {
            tracing::info!("扫描路径: {}", path);
            let _ = progress_sender.send(Event::ScanProgress(format!("正在扫描: {}", path)));
            
            // 扫描路径查找项目
            match file_walker.scan_paths(&[path.clone()]).await {
                Ok(detected_projects) => {
                    tracing::info!("FileWalker 返回了 {} 个检测到的项目", detected_projects.len());
                    for detected in detected_projects {
                        // 立即计算依赖大小（基于已检测的依赖信息）
                        let immediate_dependency_size: u64 = detected.dependencies.iter().map(|d| d.size).sum();
                        let dependency_file_count: usize = detected.dependencies.iter().map(|d| d.package_count.unwrap_or(0)).sum();
                        
                        // 快速创建项目对象，显示初始依赖大小
                        let project = Project {
                            name: detected.name.clone(),
                            path: detected.path.clone(),
                            project_type: detected.project_type,
                            code_size: 0, // 稍后异步计算
                            total_size: immediate_dependency_size, // 使用立即计算的依赖大小
                            gitignore_excluded_size: 0, // 稍后异步计算
                            code_file_count: 0, // 稍后异步计算
                            dependency_file_count, // 使用立即计算的依赖文件数
                            total_file_count: dependency_file_count, // 临时使用依赖文件数
                            gitignore_excluded_file_count: 0, // 稍后异步计算
                            last_modified: chrono::Utc::now(), // 使用当前时间作为默认值
                            git_info: None, // 稍后异步分析
                            dependencies: detected.dependencies,
                            is_ignored: false,
                            description: detected.description,
                            // 总是设为未计算状态，即使有立即计算的依赖大小
                            // 这样用户能看到"等待计算"状态，然后看到异步计算的进度
                            dependency_calculation_status: DependencyCalculationStatus::NotCalculated,
                            cached_dependency_size: Some(immediate_dependency_size), // 使用立即计算的依赖大小作为初始值
                        };
                        
                        // 立即发送项目，让用户能快速看到项目列表
                        tracing::info!("发送项目到 TUI: {}", project.name);
                        if let Err(e) = progress_sender.send(Event::ProjectFound(project.clone())) {
                            tracing::error!("发送项目失败: {}", e);
                        }
                        all_projects.push(project);
                        
                        // 在后台异步计算详细信息
                        let project_path = detected.path.clone();
                        let project_name = detected.name.clone();
                        let sender = progress_sender.clone();
                        
                        // 标记项目为正在计算状态
                        let sender_for_status = progress_sender.clone();
                        let project_name_for_status = detected.name.clone();
                        
                        tokio::spawn(async move {
                            // 先发送开始计算事件，更新项目状态
                            let _ = sender_for_status.send(Event::ProjectCalculationStarted {
                                project_name: project_name_for_status,
                            });
                            
                            Self::calculate_project_details(project_path, project_name, sender, CancellationToken::new()).await;
                        });
                    }
                }
                Err(e) => {
                    let _ = progress_sender.send(Event::ScanProgress(
                        format!("扫描路径 {} 失败: {}", path, e)
                    ));
                }
            }
        }
        
        Ok(all_projects)
    }
    
    /// 在后台异步计算项目的详细信息（大小、Git 信息等）
    async fn calculate_project_details(
        project_path: std::path::PathBuf,
        project_name: String,
        progress_sender: mpsc::UnboundedSender<Event>,
        cancellation_token: CancellationToken,
    ) {
        // 使用select来同时监听取消信号和计算任务
        tokio::select! {
            // 监听取消信号
            _ = cancellation_token.cancelled() => {
                // 任务被取消，这是正常情况
                tracing::debug!("计算 {} 的项目详细信息被取消", project_name);
            }
            // 执行计算任务（带超时）
            result = tokio::time::timeout(
                std::time::Duration::from_secs(300), // 5分钟超时
                Self::calculate_project_details_impl(project_path.clone(), project_name.clone(), progress_sender.clone(), cancellation_token.clone())
            ) => {
                match result {
                    Ok(_) => {
                        // 计算正常完成
                        tracing::debug!("项目 {} 详细信息计算完成", project_name);
                    }
                    Err(_) => {
                        // 计算超时
                        tracing::warn!("计算 {} 的项目详细信息超时", project_name);
                    }
                }
            }
        }
    }
    
    /// 实际的项目详细信息计算逻辑
    async fn calculate_project_details_impl(
        project_path: std::path::PathBuf,
        project_name: String,
        progress_sender: mpsc::UnboundedSender<Event>,
        cancellation_token: CancellationToken,
    ) {
        use crate::scanner::{GitAnalyzer, SizeCalculator};
        use crate::config::Config;
        
        // 通知开始计算
        let _ = progress_sender.send(Event::ScanProgress(
            format!("开始计算 {} 的详细信息...", project_name)
        ));
        
        let git_analyzer = GitAnalyzer::new();
        
        // 加载配置并创建带缓存的大小计算器
        let config = Config::load_or_create_default().unwrap_or_default();
        let mut size_calculator = SizeCalculator::new_with_cache(config.cache.to_size_cache_config())
            .await
            .unwrap_or_else(|_| SizeCalculator::new());
        
        // 通知开始分析Git信息
        let _ = progress_sender.send(Event::ScanProgress(
            format!("分析 {} 的Git信息...", project_name)
        ));
        
        // 分析 Git 信息（相对较快）
        let git_info = match git_analyzer.analyze_repository(&project_path) {
            Ok(info) => info,
            Err(e) => {
                tracing::warn!("分析 {} 的 Git 信息失败: {}", project_name, e);
                let _ = progress_sender.send(Event::ScanProgress(
                    format!("分析 {} 的 Git 信息失败，跳过", project_name)
                ));
                None
            }
        };
        
        // 通知开始计算大小
        let _ = progress_sender.send(Event::ScanProgress(
            format!("计算 {} 的项目大小...", project_name)
        ));
        
        // 创建进度回调
        let progress_callback = {
            let sender = progress_sender.clone();
            std::sync::Arc::new(move |name: String, processed: usize, total: Option<usize>, path: String, bytes: u64, stage| {
                let _ = sender.send(Event::SizeCalculationProgress {
                    project_name: name,
                    processed_files: processed,
                    total_files: total,
                    current_path: path,
                    bytes_processed: bytes,
                    stage,
                });
            })
        };
        
        // 使用select来同时监听取消信号和大小计算任务
        let calculation_result = tokio::select! {
            // 监听取消信号
            _ = cancellation_token.cancelled() => {
                tracing::debug!("计算 {} 的项目大小被取消", project_name);
                return;
            }
            // 执行大小计算任务
            result = size_calculator.calculate_project_size_parallel(
                &project_path, 
                Some(progress_callback), 
                project_name.clone()
            ) => result
        };
        
        match calculation_result {
            Ok(size_info) => {
                // 发送详细信息更新事件
                let _ = progress_sender.send(Event::ProjectDetailsUpdated {
                    project_name: project_name.clone(),
                    code_size: size_info.code_size,
                    dependency_size: size_info.dependency_size,
                    total_size: size_info.total_size,
                    gitignore_excluded_size: size_info.gitignore_excluded_size,
                    code_file_count: size_info.code_file_count,
                    dependency_file_count: size_info.dependency_file_count,
                    total_file_count: size_info.total_file_count,
                    gitignore_excluded_file_count: size_info.gitignore_excluded_file_count,
                    git_info,
                });
                
                // 发送完成消息
                let _ = progress_sender.send(Event::ScanProgress(
                    format!("已完成 {} 的详细信息计算", project_name)
                ));
            }
            Err(e) => {
                // 检查是否是取消导致的错误
                if cancellation_token.is_cancelled() {
                    tracing::debug!("计算 {} 的项目大小被取消: {}", project_name, e);
                    return;
                } else {
                    // 真正的计算失败，记录为警告而不是错误
                    tracing::warn!("计算 {} 的项目大小失败: {}", project_name, e);
                    let _ = progress_sender.send(Event::ScanProgress(
                        format!("计算 {} 的详细信息失败: {}，将使用默认值", project_name, e)
                    ));
                }
                
                // 发送带有默认值的更新事件，确保项目状态更新
                let _ = progress_sender.send(Event::ProjectDetailsUpdated {
                    project_name: project_name.clone(),
                    code_size: 0,
                    dependency_size: 0,
                    total_size: 0,
                    gitignore_excluded_size: 0,
                    code_file_count: 0,
                    dependency_file_count: 0,
                    total_file_count: 0,
                    gitignore_excluded_file_count: 0,
                    git_info,
                });
            }
        }
    }
    
    /// 切换视图标签
    fn switch_tab(&mut self) {
        self.current_tab = match self.current_tab {
            TabView::Projects => TabView::Statistics,
            TabView::Statistics => TabView::GitStatus,
            TabView::GitStatus => TabView::Projects,
        };
    }
    
    /// 清理当前项目
    async fn clean_current_project(&mut self) -> Result<()> {
        if let Some(project) = self.projects.get(self.selected_project) {
            self.status_message = format!("正在清理项目: {}", project.name);
            
            let project_path = project.path.clone();
            let project_name = project.name.clone();
            let sender = self.event_handler.sender.clone();
            
            tokio::spawn(async move {
                match Self::clean_project_dependencies(&project_path).await {
                    Ok(cleaned_size) => {
                        let _ = sender.send(Event::ScanProgress(
                            format!("已清理项目 {} 的依赖，释放了 {} 空间", 
                                project_name, 
                                crate::utils::size_format::format_size(cleaned_size)
                            )
                        ));
                    }
                    Err(e) => {
                        let _ = sender.send(Event::ScanProgress(
                            format!("清理项目 {} 失败: {}", project_name, e)
                        ));
                    }
                }
            });
        }
        Ok(())
    }
    
    /// 切换忽略项目状态
    async fn toggle_ignore_project(&mut self) -> Result<()> {
        if let Some(project) = self.projects.get_mut(self.selected_project) {
            project.is_ignored = !project.is_ignored;
            let status = if project.is_ignored { "已忽略" } else { "已取消忽略" };
            self.status_message = format!("项目 {} {}", project.name, status);
            
            // TODO: 将忽略状态保存到配置文件
            let project_path = project.path.clone();
            let is_ignored = project.is_ignored;
            self.save_ignore_status(&project_path, is_ignored).await?;
        }
        Ok(())
    }
    
    /// 删除当前项目
    async fn delete_current_project(&mut self) -> Result<()> {
        if let Some(project) = self.projects.get(self.selected_project) {
            let project_path = project.path.clone();
            let project_name = project.name.clone();
            
            self.status_message = format!("正在删除项目: {}", project_name);
            
            let sender = self.event_handler.sender.clone();
            tokio::spawn(async move {
                match Self::delete_project_to_trash(&project_path).await {
                    Ok(_) => {
                        let _ = sender.send(Event::ScanProgress(
                            format!("已将项目 {} 移动到回收站", project_name)
                        ));
                    }
                    Err(e) => {
                        let _ = sender.send(Event::ScanProgress(
                            format!("删除项目 {} 失败: {}", project_name, e)
                        ));
                    }
                }
            });
            
            // 从列表中移除项目
            self.projects.remove(self.selected_project);
            if self.selected_project >= self.projects.len() && !self.projects.is_empty() {
                self.selected_project = self.projects.len() - 1;
            }
        }
        Ok(())
    }
    
    /// 清理项目依赖
    async fn clean_project_dependencies(project_path: &std::path::Path) -> Result<u64> {
        use std::fs;
        
        let mut total_cleaned = 0u64;
        
        // 清理常见的依赖目录
        let dependency_dirs = [
            "node_modules",
            "target",
            "build",
            "dist",
            "__pycache__",
            ".venv",
            "venv",
        ];
        
        for dep_dir in dependency_dirs {
            let dep_path = project_path.join(dep_dir);
            if dep_path.exists() && dep_path.is_dir() {
                match Self::calculate_directory_size(&dep_path).await {
                    Ok(size) => {
                        total_cleaned += size;
                        if let Err(e) = fs::remove_dir_all(&dep_path) {
                            eprintln!("删除目录 {} 失败: {}", dep_path.display(), e);
                        }
                    }
                    Err(e) => {
                        eprintln!("计算目录 {} 大小失败: {}", dep_path.display(), e);
                    }
                }
            }
        }
        
        Ok(total_cleaned)
    }
    
    /// 删除项目到回收站
    async fn delete_project_to_trash(project_path: &std::path::Path) -> Result<()> {
        // 使用 trash crate 安全删除到回收站
        trash::delete(project_path)
            .map_err(|e| anyhow::anyhow!("无法删除项目到回收站: {}", e))
    }
    
    /// 保存项目忽略状态到配置
    async fn save_ignore_status(&self, project_path: &std::path::Path, is_ignored: bool) -> Result<()> {
        // TODO: 实现配置文件更新逻辑
        // 这里可以将忽略的项目路径保存到配置文件中
        println!("保存忽略状态: {} -> {}", project_path.display(), is_ignored);
        Ok(())
    }
    
    /// 计算目录大小
    async fn calculate_directory_size(dir: &std::path::Path) -> Result<u64> {
        use std::fs;
        use tokio::task;
        
        let dir = dir.to_path_buf();
        task::spawn_blocking(move || {
            fn calculate_size(path: &std::path::Path) -> Result<u64> {
                let mut total = 0u64;
                
                if path.is_file() {
                    return Ok(fs::metadata(path)?.len());
                }
                
                if path.is_dir() {
                    for entry in fs::read_dir(path)? {
                        let entry = entry?;
                        total += calculate_size(&entry.path())?;
                    }
                }
                
                Ok(total)
            }
            
            calculate_size(&dir)
        }).await?
    }
    
    /// 暂停终端（为启动外部编辑器做准备）
    fn suspend_terminal() -> Result<()> {
        // 离开备用屏幕
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        // 禁用原始模式
        disable_raw_mode()?;
        Ok(())
    }
    
    /// 恢复终端（从外部编辑器返回后）
    fn restore_terminal() -> Result<()> {
        // 启用原始模式
        enable_raw_mode()?;
        // 进入备用屏幕
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(())
    }
    
    /// 使用nvim打开项目目录  
    async fn spawn_nvim(&mut self, project_path: &std::path::Path) -> Result<bool> {
        use std::process::Command;
        
        // 设置状态为外部编辑器
        self.state = AppState::ExternalEditor;
        
        // 暂停事件处理器
        self.event_handler.pause();
        
        // 暂停终端
        Self::suspend_terminal()?;
        
        // 启动nvim（设置工作目录为项目路径）
        let status = Command::new("nvim")
            .current_dir(project_path)  // 设置工作目录
            .arg(".")                   // 在当前目录打开
            .status();
        
        // 恢复终端
        Self::restore_terminal()?;
        
        // 恢复事件处理器
        self.event_handler.resume();
        
        // 恢复到项目列表状态
        self.state = AppState::ProjectList;
        
        match status {
            Ok(exit_status) => {
                if exit_status.success() {
                    self.status_message = "已完成编辑".to_string();
                } else {
                    self.status_message = "编辑器异常退出".to_string();
                }
            }
            Err(e) => {
                self.status_message = format!("启动nvim失败: {}. 请确保nvim已安装", e);
                return Err(anyhow::anyhow!("启动nvim失败: {}", e));
            }
        }
        
        // 返回true表示需要重绘界面
        Ok(true)
    }
    
    /// 清理已完成的任务
    fn cleanup_finished_tasks(&mut self) {
        self.calculation_tasks.retain(|_project_name, handle| {
            !handle.is_finished()
        });
    }
    
    /// 检查是否可以启动新任务
    #[allow(dead_code)]
    fn can_start_new_task(&self) -> bool {
        self.calculation_tasks.len() < self.max_concurrent_tasks
    }
    
    /// 等待任务槽位可用
    #[allow(dead_code)]
    async fn wait_for_task_slot(&mut self) {
        while !self.can_start_new_task() {
            self.cleanup_finished_tasks();
            if !self.can_start_new_task() {
                tokio::task::yield_now().await;
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }
    }
    
    /// 启动项目计算任务（带并发控制）
    #[allow(dead_code)]
    async fn start_project_calculation_task(
        &mut self, 
        project_path: std::path::PathBuf,
        project_name: String,
        sender: mpsc::UnboundedSender<Event>
    ) {
        // 清理已完成的任务
        self.cleanup_finished_tasks();
        
        // 等待任务槽位可用
        self.wait_for_task_slot().await;
        
        // 保存项目名用于任务管理
        let task_key = project_name.clone();
        
        // 启动新任务
        let handle = tokio::spawn(async move {
            // 先发送开始计算事件
            let _ = sender.send(Event::ProjectCalculationStarted {
                project_name: project_name.clone(),
            });
            
            Self::calculate_project_details(project_path, project_name, sender, CancellationToken::new()).await;
        });
        
        self.calculation_tasks.insert(task_key, handle);
    }
    
    /// 清理所有运行中的任务
    async fn cleanup_all_tasks(&mut self) {
        if self.calculation_tasks.is_empty() {
            return;
        }
        
        tracing::debug!("开始清理 {} 个计算任务", self.calculation_tasks.len());
        
        // 首先发送取消信号给所有任务
        self.cancellation_token.cancel();
        
        let tasks: Vec<_> = self.calculation_tasks.drain().collect();
        let mut handles = Vec::new();
        
        for (_project_name, handle) in tasks {
            handles.push(handle);
        }
        
        if !handles.is_empty() {
            // 给任务一些时间优雅退出
            let timeout = tokio::time::Duration::from_secs(3);
            match tokio::time::timeout(timeout, futures::future::join_all(handles)).await {
                Ok(_) => tracing::debug!("所有计算任务已优雅完成"),
                Err(_) => {
                    tracing::debug!("等待任务完成超时，任务将被丢弃");
                    // 超时的任务会自动被丢弃
                }
            }
        }
    }
}