//! Библиотека утилиты `dc` — высокопроизводительного поиска мертвого кода
//! в проектах на Python и Django.
//!
//! # Архитектура
//!
//! Библиотека разделена на четыре слоя:
//!
//! - [`model`] — доменные типы: сущности кода, ссылки, находки и отчеты;
//! - [`pipeline`] — конвейер анализа: сбор файлов, параллельный парсинг,
//!   извлечение сущностей, граф зависимостей и достижимость (приватный
//!   модуль, наружу видна только функция [`run_analysis`]);
//! - [`config`] и [`error`] — конфигурация и ошибки;
//! - [`render`] — представление отчетов в текстовом и JSON форматах.
//!
//! # Пример
//!
//! ```no_run
//! use std::path::Path;
//! use dc::{load_configuration, render_report, run_analysis, ReportFormat};
//!
//! let target = Path::new(".");
//! let configuration = load_configuration(None, target).expect("конфигурация");
//! let report = run_analysis(target, &configuration);
//! print!("{}", render_report(&report, ReportFormat::Text, false));
//! ```

mod heuristics;
mod pipeline;

pub mod config;
pub mod error;
pub mod model;
pub mod render;

pub use config::{load_configuration, AnalyzerConfiguration};
pub use error::DcError;
pub use model::{
    AnalysisReport, CodeEntity, DeadCodeFinding, EntityKind, FileAnalysis, ScopedReference,
    SkippedFile,
};
pub use pipeline::run_analysis;
pub use render::{render_report, ReportFormat};
