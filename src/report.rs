use std::path::PathBuf;

use crate::entity_extractor::EntityKind;

/// Одна находка мертвого кода.
#[derive(Debug)]
pub struct DeadCodeFinding {
    /// Путь к файлу с определением.
    pub file_path: PathBuf,
    /// Номер строки определения.
    pub line_number: usize,
    /// Вид сущности.
    pub entity_kind: EntityKind,
    /// Полное точечное имя сущности.
    pub qualified_name: String,
}

/// Итоговый отчет анализа проекта.
#[derive(Debug)]
pub struct AnalysisReport {
    /// Находки мертвого кода.
    pub findings: Vec<DeadCodeFinding>,
    /// Количество проанализированных файлов.
    pub analyzed_file_count: usize,
    /// Количество извлеченных сущностей.
    pub extracted_entity_count: usize,
}

/// Выводит отчет анализа в стандартный поток вывода.
///
/// :param analysis_report: Итоговый отчет анализа.
/// :param verbose: Признак вывода дополнительной статистики.
pub fn print_report(analysis_report: &AnalysisReport, verbose: bool) {
    if verbose {
        println!("Проанализировано файлов: {}", analysis_report.analyzed_file_count);
        println!("Извлечено сущностей: {}", analysis_report.extracted_entity_count);
        println!();
    }

    if analysis_report.findings.is_empty() {
        println!("Мертвый код не найден.");
        return;
    }

    println!("Найдено объектов мертвого кода: {}", analysis_report.findings.len());
    println!();
    for finding in &analysis_report.findings {
        println!(
            "{}:{}: {} `{}` не используется",
            finding.file_path.display(),
            finding.line_number,
            finding.entity_kind.label(),
            finding.qualified_name,
        );
    }
}
