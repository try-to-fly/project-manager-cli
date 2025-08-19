pub mod file_walker;
pub mod git_analyzer;
pub mod git_ignore_analyzer;
pub mod parallel_file_walker;
pub mod project_detector;
pub mod size_cache;
pub mod size_calculator;

pub use file_walker::{FileWalker, ScanProgress};
pub use git_analyzer::{GitAnalyzer, RepositoryStats};
pub use git_ignore_analyzer::{GitIgnoreAnalyzer, IgnoreStats};
pub use project_detector::{ProjectDetector, DetectedProject};
pub use size_cache::{SizeCache, CachedSizeInfo, CacheConfig, CacheStats, CacheStatus};
pub use size_calculator::{SizeCalculator, ProjectSizeInfo, DirectorySizeInfo};
pub use parallel_file_walker::{ParallelFileWalker, FileInfo, SizeCalculationResult, ScanProgress as ParallelScanProgress, ScanStage};