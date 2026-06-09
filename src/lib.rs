//! Библиотека утилиты `dc` для поиска мертвого кода в Django проектах.

pub mod analyzer;
pub mod configuration;
pub mod dependency_graph;
pub mod django_heuristics;
pub mod entity_extractor;
pub mod file_collector;
pub mod report;

pub use analyzer::run_analysis;
pub use configuration::{load_configuration, AnalyzerConfiguration};
pub use entity_extractor::{CodeEntity, EntityKind, FileAnalysis};
pub use report::{print_report, AnalysisReport, DeadCodeFinding};
