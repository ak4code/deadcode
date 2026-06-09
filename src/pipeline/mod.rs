//! Конвейер анализа проекта.
//!
//! Анализ состоит из пяти последовательных этапов:
//! 1. Сбор файлов ([`collect`]).
//! 2. Параллельный парсинг ([`extract`]).
//! 3. Извлечение сущностей ([`extract`]).
//! 4. Построение графа зависимостей ([`reachability`]).
//! 5. Вычисление достижимости от точек входа ([`reachability`]).

mod collect;
mod extract;
mod reachability;

use std::path::Path;

use rayon::iter::Either;
use rayon::prelude::*;

use crate::config::AnalyzerConfiguration;
use crate::model::{AnalysisReport, DeadCodeFinding, FileAnalysis, SkippedFile};

/// Запускает полный цикл анализа проекта.
///
/// Файлы парсятся параллельно; парсер `tree-sitter` создается один раз
/// на рабочий поток. Ошибки чтения и парсинга отдельных файлов
/// не прерывают анализ и попадают в отчет как пропущенные файлы.
///
/// :param target_path: Корневая директория анализируемого проекта.
/// :param configuration: Конфигурация анализатора.
/// :return: Итоговый отчет анализа.
pub fn run_analysis(target_path: &Path, configuration: &AnalyzerConfiguration) -> AnalysisReport {
    let python_files = collect::collect_python_files(target_path, configuration);

    let (file_analyses, mut skipped_files): (Vec<FileAnalysis>, Vec<SkippedFile>) = python_files
        .par_iter()
        .map_init(
            extract::PythonSourceParser::new,
            |source_parser, python_file| {
                extract::analyze_python_file(source_parser, python_file, target_path, configuration)
            },
        )
        .partition_map(|analysis_result| match analysis_result {
            Ok(file_analysis) => Either::Left(file_analysis),
            Err(skipped_file) => Either::Right(skipped_file),
        });
    skipped_files.sort_by(|first, second| first.file_path.cmp(&second.file_path));

    let findings = reachability::find_unreachable_entities(&file_analyses, configuration)
        .into_iter()
        .map(DeadCodeFinding::from)
        .collect();

    AnalysisReport {
        findings,
        skipped_files,
        analyzed_file_count: file_analyses.len(),
        extracted_entity_count: file_analyses
            .iter()
            .map(|file_analysis| file_analysis.entities.len())
            .sum(),
    }
}
