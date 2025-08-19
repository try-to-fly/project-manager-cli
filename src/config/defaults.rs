use std::collections::HashSet;

pub struct DefaultConfig;

impl DefaultConfig {
    /// 默认忽略的目录名
    pub fn default_ignore_dirs() -> HashSet<String> {
        let mut dirs = HashSet::new();
        
        // macOS 系统目录
        dirs.insert("System".to_string());
        dirs.insert("Library".to_string());
        dirs.insert("Applications".to_string());
        dirs.insert("private".to_string());
        dirs.insert("usr".to_string());
        dirs.insert("var".to_string());
        dirs.insert("tmp".to_string());
        dirs.insert("dev".to_string());
        dirs.insert("proc".to_string());
        dirs.insert("sys".to_string());
        
        // macOS 用户特殊目录
        dirs.insert("Music".to_string());
        dirs.insert("Movies".to_string());
        dirs.insert("Pictures".to_string());
        dirs.insert("Desktop".to_string());
        dirs.insert(".Trash".to_string());
        dirs.insert(".DocumentRevisions-V100".to_string());
        dirs.insert(".fseventsd".to_string());
        dirs.insert(".Spotlight-V100".to_string());
        dirs.insert(".TemporaryItems".to_string());
        dirs.insert(".Trashes".to_string());
        dirs.insert(".vol".to_string());
        
        // 常见的依赖目录
        dirs.insert("node_modules".to_string());
        dirs.insert("target".to_string());
        dirs.insert("dist".to_string());
        dirs.insert("build".to_string());
        dirs.insert(".git".to_string());
        dirs.insert(".svn".to_string());
        dirs.insert("__pycache__".to_string());
        dirs.insert(".pytest_cache".to_string());
        dirs.insert("venv".to_string());
        dirs.insert("env".to_string());
        dirs.insert(".venv".to_string());
        dirs.insert(".env".to_string());
        
        // IDE 和工具目录
        dirs.insert(".vscode".to_string());
        dirs.insert(".idea".to_string());
        dirs.insert(".vs".to_string());
        
        dirs
    }
    
    /// 默认忽略的文件扩展名
    pub fn default_ignore_extensions() -> HashSet<String> {
        let mut exts = HashSet::new();
        
        // 日志文件
        exts.insert("log".to_string());
        
        // 临时文件
        exts.insert("tmp".to_string());
        exts.insert("temp".to_string());
        exts.insert("swp".to_string());
        exts.insert("~".to_string());
        
        // 缓存文件
        exts.insert("cache".to_string());
        
        exts
    }
    
    /// 默认扫描的根目录
    pub fn default_scan_paths() -> Vec<String> {
        vec![
            dirs::home_dir()
                .map(|p| p.join("Documents").to_string_lossy().to_string())
                .unwrap_or_else(|| "~/Documents".to_string()),
            dirs::home_dir()
                .map(|p| p.join("Code").to_string_lossy().to_string())
                .unwrap_or_else(|| "~/Code".to_string()),
            dirs::home_dir()
                .map(|p| p.join("Projects").to_string_lossy().to_string())
                .unwrap_or_else(|| "~/Projects".to_string()),
            dirs::home_dir()
                .map(|p| p.join("Development").to_string_lossy().to_string())
                .unwrap_or_else(|| "~/Development".to_string()),
        ]
    }
}