use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};

use crate::models::Project;
use crate::tui::app::TabView;
use crate::utils::{size_format, time_format};
use crate::models::DependencyCalculationStatus;

/// 主屏幕组件 - 负责绘制项目列表和详情页面
pub struct MainScreen {
    /// 列表状态
    list_state: ListState,
}

impl MainScreen {
    /// 创建新的主屏幕
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
        }
    }
    
    /// 绘制项目列表视图
    pub fn draw_project_list(
        &mut self,
        f: &mut Frame,
        area: Rect,
        projects: &[Project],
        selected_index: usize,
        current_tab: &TabView,
    ) {
        // 创建布局
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // 标签栏
                Constraint::Min(0),    // 主内容区
            ])
            .split(area);
        
        // 绘制标签栏
        self.draw_tab_bar(f, chunks[0], current_tab);
        
        // 根据当前标签绘制不同内容
        match current_tab {
            TabView::Projects => {
                self.draw_projects_view(f, chunks[1], projects, selected_index);
            }
            TabView::Statistics => {
                self.draw_statistics_view(f, chunks[1], projects);
            }
            TabView::GitStatus => {
                self.draw_git_status_view(f, chunks[1], projects);
            }
        }
    }
    
    /// 绘制项目详情页面
    pub fn draw_project_detail(&mut self, f: &mut Frame, area: Rect, project: &Project) {
        // 创建布局
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 标题
                Constraint::Min(0),     // 详情内容
                Constraint::Length(3),  // 操作提示
            ])
            .split(area);
        
        // 绘制标题
        let title_block = Block::default()
            .title(format!("项目详情: {}", project.name))
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        
        let title_paragraph = Paragraph::new("")
            .block(title_block);
        
        f.render_widget(title_paragraph, chunks[0]);
        
        // 绘制详情内容
        self.draw_project_details(f, chunks[1], project);
        
        // 绘制操作提示
        let help_text = vec![
            Line::from(vec![
                Span::raw("按 "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" 或 "),
                Span::styled("Backspace", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" 返回项目列表"),
            ])
        ];
        
        let help_block = Block::default()
            .title("操作")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));
        
        let help_paragraph = Paragraph::new(help_text)
            .block(help_block);
        
        f.render_widget(help_paragraph, chunks[2]);
    }
    
    /// 绘制标签栏
    fn draw_tab_bar(&self, f: &mut Frame, area: Rect, current_tab: &TabView) {
        let tab_titles = vec!["项目列表", "统计信息", "Git状态"];
        
        let selected_tab = match current_tab {
            TabView::Projects => 0,
            TabView::Statistics => 1,
            TabView::GitStatus => 2,
        };
        
        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("视图"))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(selected_tab);
        
        f.render_widget(tabs, area);
    }
    
    /// 绘制项目列表视图
    fn draw_projects_view(&mut self, f: &mut Frame, area: Rect, projects: &[Project], selected_index: usize) {
        if projects.is_empty() {
            let empty_message = Paragraph::new("未发现任何项目\n\n按 'r' 刷新扫描")
                .block(Block::default().title("项目列表").borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            
            f.render_widget(empty_message, area);
            return;
        }
        
        // 创建项目列表项
        let items: Vec<ListItem> = projects
            .iter()
            .enumerate()
            .map(|(i, project)| {
                let style = if project.is_ignored {
                    Style::default().fg(Color::Gray)
                } else if i == selected_index {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };
                
                // 构建项目信息行
                let mut spans = vec![
                    Span::styled(
                        format!("{:<30}", project.name),
                        style.add_modifier(Modifier::BOLD)
                    ),
                ];
                
                // 添加项目类型图标
                let type_icon = match project.project_type.as_str() {
                    "git" => "📁",
                    "nodejs" => "📦",
                    "python" => "🐍",
                    "rust" => "🦀",
                    "go" => "🐹",
                    "java" => "☕",
                    "cpp" => "⚡",
                    _ => "📄",
                };
                
                spans.push(Span::styled(
                    format!(" {} ", type_icon),
                    style
                ));
                
                // 添加大小信息
                spans.push(Span::styled(
                    format!("{:<12}", size_format::format_size(project.size())),
                    style
                ));
                
                // 添加最后修改时间
                let modified_time = std::time::SystemTime::UNIX_EPOCH + 
                    std::time::Duration::from_secs(project.last_modified.timestamp() as u64);
                spans.push(Span::styled(
                    format!(" {}", time_format::format_time(modified_time)),
                    style.fg(Color::Gray)
                ));
                
                // 添加状态标识
                if project.is_ignored {
                    spans.push(Span::styled(" [已忽略]", Style::default().fg(Color::Red)));
                }
                
                if project.has_uncommitted_changes() {
                    spans.push(Span::styled(" [未提交]", Style::default().fg(Color::Yellow)));
                }
                
                // 添加依赖计算状态
                let dependency_status = project.dependency_status_display();
                if !dependency_status.is_empty() {
                    let status_color = match project.dependency_calculation_status {
                        DependencyCalculationStatus::Calculating => Color::Cyan,
                        DependencyCalculationStatus::NotCalculated => Color::Gray,
                        DependencyCalculationStatus::Failed(_) => Color::Red,
                        _ => Color::Gray,
                    };
                    spans.push(Span::styled(
                        format!(" [{}]", dependency_status), 
                        Style::default().fg(status_color)
                    ));
                }
                
                ListItem::new(Line::from(spans))
            })
            .collect();
        
        // 更新列表状态
        self.list_state.select(Some(selected_index));
        
        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!("项目列表 ({} 个项目)", projects.len()))
                    .borders(Borders::ALL)
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            );
        
        f.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    /// 绘制统计信息视图
    fn draw_statistics_view(&self, f: &mut Frame, area: Rect, projects: &[Project]) {
        let mut stats_text = vec![
            Line::from(vec![
                Span::styled("项目统计信息", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
            ]),
            Line::from(""),
        ];
        
        // 总体统计
        let total_projects = projects.len();
        let ignored_projects = projects.iter().filter(|p| p.is_ignored).count();
        let active_projects = total_projects - ignored_projects;
        
        stats_text.push(Line::from(vec![
            Span::styled("总项目数: ", Style::default().fg(Color::White)),
            Span::styled(total_projects.to_string(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        
        stats_text.push(Line::from(vec![
            Span::styled("活跃项目: ", Style::default().fg(Color::White)),
            Span::styled(active_projects.to_string(), Style::default().fg(Color::Green)),
        ]));
        
        stats_text.push(Line::from(vec![
            Span::styled("已忽略项目: ", Style::default().fg(Color::White)),
            Span::styled(ignored_projects.to_string(), Style::default().fg(Color::Gray)),
        ]));
        
        stats_text.push(Line::from(""));
        
        // 按类型统计
        stats_text.push(Line::from(vec![
            Span::styled("按类型分布:", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))
        ]));
        
        let mut type_counts = std::collections::HashMap::new();
        for project in projects {
            if !project.is_ignored {
                *type_counts.entry(project.project_type.as_str()).or_insert(0) += 1;
            }
        }
        
        for (project_type, count) in type_counts {
            let type_name = match project_type {
                "git" => "Git 仓库",
                "nodejs" => "Node.js",
                "python" => "Python",
                "rust" => "Rust",
                "go" => "Go",
                "java" => "Java",
                "cpp" => "C++",
                _ => "其他",
            };
            
            stats_text.push(Line::from(vec![
                Span::styled(format!("  {}: ", type_name), Style::default().fg(Color::White)),
                Span::styled(count.to_string(), Style::default().fg(Color::Green)),
            ]));
        }
        
        stats_text.push(Line::from(""));
        
        // 大小统计
        let total_size: u64 = projects.iter().filter(|p| !p.is_ignored).map(|p| p.size()).sum();
        let total_dependency_size: u64 = projects.iter()
            .filter(|p| !p.is_ignored)
            .map(|p| p.dependency_size())
            .sum();
        
        stats_text.push(Line::from(vec![
            Span::styled("存储使用:", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))
        ]));
        
        stats_text.push(Line::from(vec![
            Span::styled("  代码总大小: ", Style::default().fg(Color::White)),
            Span::styled(size_format::format_size(total_size), Style::default().fg(Color::Green)),
        ]));
        
        stats_text.push(Line::from(vec![
            Span::styled("  依赖总大小: ", Style::default().fg(Color::White)),
            Span::styled(size_format::format_size(total_dependency_size), Style::default().fg(Color::Yellow)),
        ]));
        
        // Git 统计
        let git_projects: Vec<_> = projects.iter().filter(|p| !p.is_ignored && p.git_info.is_some()).collect();
        let uncommitted_changes = git_projects.iter().filter(|p| p.has_uncommitted_changes()).count();
        
        stats_text.push(Line::from(""));
        stats_text.push(Line::from(vec![
            Span::styled("Git 状态:", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))
        ]));
        
        stats_text.push(Line::from(vec![
            Span::styled("  Git 仓库数: ", Style::default().fg(Color::White)),
            Span::styled(git_projects.len().to_string(), Style::default().fg(Color::Green)),
        ]));
        
        stats_text.push(Line::from(vec![
            Span::styled("  有未提交更改: ", Style::default().fg(Color::White)),
            Span::styled(uncommitted_changes.to_string(), Style::default().fg(Color::Red)),
        ]));
        
        let stats_paragraph = Paragraph::new(stats_text)
            .block(Block::default().title("统计信息").borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        f.render_widget(stats_paragraph, area);
    }
    
    /// 绘制 Git 状态视图
    fn draw_git_status_view(&self, f: &mut Frame, area: Rect, projects: &[Project]) {
        let git_projects: Vec<_> = projects.iter()
            .filter(|p| !p.is_ignored && p.git_info.is_some())
            .collect();
        
        if git_projects.is_empty() {
            let empty_message = Paragraph::new("未发现 Git 仓库")
                .block(Block::default().title("Git 状态").borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));
            
            f.render_widget(empty_message, area);
            return;
        }
        
        let items: Vec<ListItem> = git_projects
            .iter()
            .map(|project| {
                let git_info = project.git_info.as_ref().unwrap();
                
                let mut spans = vec![
                    Span::styled(
                        format!("{:<25}", project.name),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    ),
                ];
                
                // 分支信息
                spans.push(Span::styled(
                    format!(" [{}]", git_info.current_branch.as_deref().unwrap_or("unknown")),
                    Style::default().fg(Color::Green)
                ));
                
                // 远程仓库
                if let Some(remote_url) = &git_info.remote_url {
                    let display_url = if remote_url.len() > 40 {
                        format!("{}...", &remote_url[..37])
                    } else {
                        remote_url.clone()
                    };
                    spans.push(Span::styled(
                        format!(" {}", display_url),
                        Style::default().fg(Color::Blue)
                    ));
                }
                
                // 未提交更改状态
                if project.has_uncommitted_changes() {
                    spans.push(Span::styled(
                        " [未提交更改]",
                        Style::default().fg(Color::Red)
                    ));
                } else {
                    spans.push(Span::styled(
                        " [清洁]",
                        Style::default().fg(Color::Green)
                    ));
                }
                
                // 最后提交时间
                if let Some(last_commit) = git_info.last_commit_time {
                    let commit_time = std::time::SystemTime::UNIX_EPOCH + 
                        std::time::Duration::from_secs(last_commit.timestamp() as u64);
                    spans.push(Span::styled(
                        format!(" ({})", time_format::format_time(commit_time)),
                        Style::default().fg(Color::Gray)
                    ));
                }
                
                ListItem::new(Line::from(spans))
            })
            .collect();
        
        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!("Git 状态 ({} 个仓库)", git_projects.len()))
                    .borders(Borders::ALL)
            );
        
        f.render_widget(list, area);
    }
    
    /// 绘制项目详情内容
    fn draw_project_details(&self, f: &mut Frame, area: Rect, project: &Project) {
        // 创建两列布局
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // 左列：基本信息
                Constraint::Percentage(50), // 右列：Git 信息和统计
            ])
            .split(area);
        
        // 左列：基本信息
        self.draw_basic_info(f, chunks[0], project);
        
        // 右列：Git 信息和统计
        self.draw_extended_info(f, chunks[1], project);
    }
    
    /// 绘制基本信息
    fn draw_basic_info(&self, f: &mut Frame, area: Rect, project: &Project) {
        let mut info_text = vec![
            Line::from(vec![
                Span::styled("基本信息", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
            ]),
            Line::from(""),
        ];
        
        info_text.push(Line::from(vec![
            Span::styled("项目名称: ", Style::default().fg(Color::White)),
            Span::styled(&project.name, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        
        info_text.push(Line::from(vec![
            Span::styled("项目路径: ", Style::default().fg(Color::White)),
            Span::raw(project.path.display().to_string()),
        ]));
        
        info_text.push(Line::from(vec![
            Span::styled("项目类型: ", Style::default().fg(Color::White)),
            Span::styled(project.type_display_name(), Style::default().fg(Color::Yellow)),
        ]));
        
        info_text.push(Line::from(vec![
            Span::styled("项目大小: ", Style::default().fg(Color::White)),
            Span::styled(size_format::format_size(project.size()), Style::default().fg(Color::Green)),
        ]));
        
        // 依赖大小显示（根据计算状态）
        let dependency_status = project.dependency_status_display();
        if dependency_status.is_empty() {
            info_text.push(Line::from(vec![
                Span::styled("依赖大小: ", Style::default().fg(Color::White)),
                Span::styled(size_format::format_size(project.dependency_size()), Style::default().fg(Color::Yellow)),
            ]));
        } else {
            info_text.push(Line::from(vec![
                Span::styled("依赖大小: ", Style::default().fg(Color::White)),
                Span::styled(size_format::format_size(project.dependency_size()), Style::default().fg(Color::Yellow)),
                Span::styled(format!(" ({})", dependency_status), Style::default().fg(Color::Gray)),
            ]));
        }
        
        let modified_time = std::time::SystemTime::UNIX_EPOCH + 
            std::time::Duration::from_secs(project.last_modified.timestamp() as u64);
        info_text.push(Line::from(vec![
            Span::styled("最后修改: ", Style::default().fg(Color::White)),
            Span::styled(time_format::format_time(modified_time), Style::default().fg(Color::Magenta)),
        ]));
        
        info_text.push(Line::from(vec![
            Span::styled("状态: ", Style::default().fg(Color::White)),
            if project.is_ignored {
                Span::styled("已忽略", Style::default().fg(Color::Red))
            } else {
                Span::styled("活跃", Style::default().fg(Color::Green))
            },
        ]));
        
        let info_paragraph = Paragraph::new(info_text)
            .block(Block::default().title("详细信息").borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        f.render_widget(info_paragraph, area);
    }
    
    /// 绘制扩展信息（Git 和统计）
    fn draw_extended_info(&self, f: &mut Frame, area: Rect, project: &Project) {
        let mut info_text = vec![];
        
        // Git 信息
        if let Some(git_info) = &project.git_info {
            info_text.push(Line::from(vec![
                Span::styled("Git 信息", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
            ]));
            info_text.push(Line::from(""));
            
            info_text.push(Line::from(vec![
                Span::styled("当前分支: ", Style::default().fg(Color::White)),
                Span::styled(git_info.current_branch.as_deref().unwrap_or("unknown"), Style::default().fg(Color::Green)),
            ]));
            
            if let Some(remote_url) = &git_info.remote_url {
                info_text.push(Line::from(vec![
                    Span::styled("远程仓库: ", Style::default().fg(Color::White)),
                    Span::styled(remote_url, Style::default().fg(Color::Blue)),
                ]));
            }
            
            if let Some(last_commit) = git_info.last_commit_time {
                let commit_time = std::time::SystemTime::UNIX_EPOCH + 
                    std::time::Duration::from_secs(last_commit.timestamp() as u64);
                info_text.push(Line::from(vec![
                    Span::styled("最后提交: ", Style::default().fg(Color::White)),
                    Span::styled(time_format::format_time(commit_time), Style::default().fg(Color::Magenta)),
                ]));
            }
            
            info_text.push(Line::from(vec![
                Span::styled("工作区状态: ", Style::default().fg(Color::White)),
                if project.has_uncommitted_changes() {
                    Span::styled("有未提交更改", Style::default().fg(Color::Red))
                } else {
                    Span::styled("清洁", Style::default().fg(Color::Green))
                },
            ]));
            
            info_text.push(Line::from(""));
        }
        
        // 统计信息
        info_text.push(Line::from(vec![
            Span::styled("文件统计", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
        ]));
        info_text.push(Line::from(""));
        
        info_text.push(Line::from(vec![
            Span::styled("代码文件数: ", Style::default().fg(Color::White)),
            Span::styled(project.file_count().to_string(), Style::default().fg(Color::Green)),
        ]));
        
        if project.total_files() > project.file_count() {
            info_text.push(Line::from(vec![
                Span::styled("总文件数: ", Style::default().fg(Color::White)),
                Span::styled(project.total_files().to_string(), Style::default().fg(Color::Cyan)),
            ]));
        }
        
        if project.dependency_files() > 0 {
            info_text.push(Line::from(vec![
                Span::styled("依赖文件数: ", Style::default().fg(Color::White)),
                Span::styled(project.dependency_files().to_string(), Style::default().fg(Color::Yellow)),
            ]));
        }
        
        if project.gitignore_excluded_file_count > 0 {
            info_text.push(Line::from(vec![
                Span::styled("已忽略文件数: ", Style::default().fg(Color::White)),
                Span::styled(project.gitignore_excluded_file_count.to_string(), Style::default().fg(Color::Red)),
            ]));
        }
        
        if project.dependency_size() > 0 {
            info_text.push(Line::from(vec![
                Span::styled("依赖占比: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:.1}%", 
                        (project.dependency_size() as f64 / (project.size() + project.dependency_size()) as f64) * 100.0
                    ),
                    Style::default().fg(Color::Yellow)
                ),
            ]));
        }
        
        let info_paragraph = Paragraph::new(info_text)
            .block(Block::default().title("Git & 统计").borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        f.render_widget(info_paragraph, area);
    }
}

impl Default for MainScreen {
    fn default() -> Self {
        Self::new()
    }
}