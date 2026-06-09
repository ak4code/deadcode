use std::path::Path;

use rayon::prelude::*;

use crate::configuration::AnalyzerConfiguration;
use crate::dependency_graph::find_unreachable_entities;
use crate::entity_extractor::{analyze_python_file, FileAnalysis};
use crate::file_collector::collect_python_files;
use crate::report::{AnalysisReport, DeadCodeFinding};

/// Запускает полный цикл анализа проекта.
///
/// Анализ состоит из пяти этапов: сбор файлов, параллельный парсинг,
/// извлечение сущностей, построение графа зависимостей и вычисление
/// достижимости от точек входа.
///
/// :param target_path: Корневая директория анализируемого проекта.
/// :param configuration: Конфигурация анализатора.
/// :return: Итоговый отчет анализа.
pub fn run_analysis(target_path: &str, configuration: &AnalyzerConfiguration) -> AnalysisReport {
    let project_root = Path::new(target_path);
    let python_files = collect_python_files(target_path, configuration);

    let file_analyses: Vec<FileAnalysis> = python_files
        .par_iter()
        .filter_map(|python_file| analyze_python_file(python_file, project_root))
        .collect();

    let unreachable_entities = find_unreachable_entities(&file_analyses);
    let findings = unreachable_entities
        .into_iter()
        .map(|code_entity| DeadCodeFinding {
            file_path: code_entity.file_path.clone(),
            line_number: code_entity.line_number,
            entity_kind: code_entity.kind,
            qualified_name: code_entity.qualified_name.clone(),
        })
        .collect();

    AnalysisReport {
        findings,
        analyzed_file_count: file_analyses.len(),
        extracted_entity_count: file_analyses
            .iter()
            .map(|file_analysis| file_analysis.entities.len())
            .sum(),
    }
}
