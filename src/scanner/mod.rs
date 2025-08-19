pub mod file_walker;
pub mod git_analyzer;
pub mod project_detector;
pub mod size_calculator;

pub use file_walker::{FileWalker, ScanProgress};
pub use git_analyzer::{GitAnalyzer, RepositoryStats};
pub use project_detector::{ProjectDetector, DetectedProject};
pub use size_calculator::{SizeCalculator, ProjectSizeInfo, DirectorySizeInfo};