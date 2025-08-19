pub mod project;
pub mod scan_result;

pub use project::{Project, ProjectType, GitInfo, DependencyInfo, DependencyType, DependencyCalculationStatus};
pub use scan_result::{ScanResult, ScanStats};