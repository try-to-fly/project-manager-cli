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
    execute,
};
use anyhow::Result;

use crate::config::Config;
use crate::models::Project;
use crate::scanner::FileWalker;
use crate::tui::events::{Event, EventHandler, keys};
use crate::tui::screens::MainScreen;
use tokio::sync::mpsc;

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
    
    /// 错误状态
    Error(String),
    
    /// 退出中
    Quitting,
}

/// 主应用程序
pub struct App {
    /// 应用配置
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
    show_details: bool,
    
    /// 当前视图标签
    current_tab: TabView,
    
    /// 扫描进度信息
    scan_progress: String,
    
    /// 事件处理器
    event_handler: EventHandler,
    
    /// 主屏幕
    main_screen: MainScreen,
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
            event_handler: EventHandler::new(),
            main_screen: MainScreen::new(),
        }
    }
    
    /// 运行应用程序
    pub async fn run(&mut self) -> Result<()> {
        // 设置终端
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        // 启动事件处理
        self.event_handler.start();
        
        // 开始扫描
        self.start_scan().await?;
        
        // 主事件循环
        let result = self.main_loop(&mut terminal).await;
        
        // 恢复终端
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        
        result
    }
    
    /// 主事件循环
    async fn main_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        loop {
            // 绘制界面
            terminal.draw(|f| self.draw(f))?;
            
            // 处理事件
            match self.event_handler.next().await? {
                Event::Key(key) => {
                    if keys::is_quit_key(&key) {
                        self.state = AppState::Quitting;
                        break;
                    }
                    
                    self.handle_key_event(key).await?;
                }
                Event::Resize(w, h) => {
                    // 终端大小调整会自动处理
                }
                Event::ScanComplete => {
                    self.state = AppState::ProjectList;
                    self.status_message = format!("扫描完成！发现 {} 个项目", self.projects.len());
                }
                Event::ScanProgress(progress) => {
                    self.scan_progress = progress;
                }
                Event::ProjectFound(project) => {
                    self.projects.push(project);
                }
                Event::ProjectSizeUpdated { project_index, code_size, total_size } => {
                    // 更新指定项目的大小信息
                    if let Some(project) = self.projects.get_mut(project_index) {
                        project.code_size = code_size;
                        project.total_size = total_size;
                    }
                }
                Event::Refresh => {
                    self.start_scan().await?;
                }
                Event::Quit => {
                    break;
                }
                Event::Tick => {
                    // 定时更新
                }
            }
            
            if self.state == AppState::Quitting {
                break;
            }
        }
        
        Ok(())
    }
    
    /// 处理键盘事件
    async fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match self.state {
            AppState::ProjectList => {
                self.handle_project_list_keys(key).await?;
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
            _ => {}
        }
        
        Ok(())
    }
    
    /// 处理项目列表键盘事件
    async fn handle_project_list_keys(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
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
        }
        
        Ok(())
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
        let size = f.area();
        
        match self.state {
            AppState::Starting => {
                self.draw_loading_screen(f, size);
            }
            AppState::Scanning => {
                self.draw_scanning_screen(f, size);
            }
            AppState::ProjectList => {
                self.main_screen.draw_project_list(f, size, &self.projects, self.selected_project, &self.current_tab);
            }
            AppState::ProjectDetail => {
                if let Some(project) = self.projects.get(self.selected_project) {
                    self.main_screen.draw_project_detail(f, size, project);
                }
            }
            AppState::Help => {
                self.draw_help_screen(f, size);
            }
            AppState::ConfirmDialog => {
                self.main_screen.draw_project_list(f, size, &self.projects, self.selected_project, &self.current_tab);
                self.draw_confirm_dialog(f, size);
            }
            AppState::Error(ref error) => {
                self.draw_error_screen(f, size, error);
            }
            _ => {}
        }
        
        // 绘制状态栏
        self.draw_status_bar(f, size);
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
            Line::from("  q, ESC, Ctrl+C  - 退出应用程序"),
            Line::from("  r, F5           - 刷新项目列表"),
            Line::from("  h, ?, F1        - 显示帮助信息"),
            Line::from("  Tab             - 切换视图标签"),
            Line::from(""),
            Line::from("  ↑/↓, k/j        - 导航项目列表"),
            Line::from("  Enter, Space    - 查看项目详情"),
            Line::from("  d, Delete       - 删除项目"),
            Line::from("  c               - 清理项目依赖"),
            Line::from("  i               - 切换忽略状态"),
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
    
    /// 绘制状态栏
    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_area = Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: area.width,
            height: 1,
        };
        
        let status_text = match self.state {
            AppState::ProjectList => {
                format!("{} | 项目: {} | 选中: {}/{}", 
                    self.status_message,
                    self.projects.len(),
                    if self.projects.is_empty() { 0 } else { self.selected_project + 1 },
                    self.projects.len()
                )
            }
            _ => self.status_message.clone(),
        };
        
        let status = Paragraph::new(status_text)
            .style(Style::default().bg(Color::Blue).fg(Color::White));
        
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
                    last_modified: chrono::Utc::now(),
                    git_info: None,
                    dependencies: Vec::new(),
                    is_ignored: false,
                    description: None,
                };
                
                self.projects.push(project);
                
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
            let size_calculator = SizeCalculator::new();
            
            for (index, project) in projects_for_calc.iter().enumerate() {
                // 计算项目大小
                if let Ok(size_info) = size_calculator.calculate_project_size(&project.path).await {
                    // 发送更新事件
                    let _ = sender.send(Event::ProjectSizeUpdated {
                        project_index: index,
                        code_size: size_info.code_size,
                        total_size: size_info.total_size,
                    });
                }
                
                // 防止计算过快导致界面更新频繁
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        });
        
        Ok(())
    }
    
    /// 异步扫描项目
    async fn scan_projects_async(
        paths: Vec<String>, 
        config: Config, 
        progress_sender: mpsc::UnboundedSender<Event>
    ) -> Result<Vec<Project>> {
        use crate::scanner::FileWalker;
        
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
                        // 快速创建项目对象，不进行耗时的大小计算和 Git 分析
                        let project = Project {
                            name: detected.name.clone(),
                            path: detected.path.clone(),
                            project_type: detected.project_type,
                            code_size: 0, // 稍后异步计算
                            total_size: 0, // 稍后异步计算
                            last_modified: chrono::Utc::now(), // 使用当前时间作为默认值
                            git_info: None, // 稍后异步分析
                            dependencies: detected.dependencies,
                            is_ignored: false,
                            description: detected.description,
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
                        
                        tokio::spawn(async move {
                            Self::calculate_project_details(project_path, project_name, sender).await;
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
        progress_sender: mpsc::UnboundedSender<Event>
    ) {
        use crate::scanner::{GitAnalyzer, SizeCalculator};
        
        let git_analyzer = GitAnalyzer::new();
        let size_calculator = SizeCalculator::new();
        
        // 分析 Git 信息（相对较快）
        let git_info = git_analyzer.analyze_repository(&project_path).unwrap_or(None);
        
        // 计算项目大小（可能较慢）
        let size_info = size_calculator.calculate_project_size(&project_path).await.unwrap_or_default();
        
        // 发送更新后的项目信息
        let _ = progress_sender.send(Event::ScanProgress(
            format!("已分析项目详情: {}", project_name)
        ));
        
        // 这里可以考虑发送一个新的事件类型来更新已有项目的详细信息
        // 例如: Event::ProjectUpdated { name, git_info, size_info }
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
        use crate::utils::size_format;
        
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
}