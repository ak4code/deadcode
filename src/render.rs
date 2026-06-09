//! Формирование отчетов анализа в текстовом и JSON форматах.

use std::fmt::Write;

use clap::ValueEnum;

use crate::model::AnalysisReport;

/// Формат вывода отчета.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReportFormat {
    /// Человекочитаемый текстовый отчет.
    Text,
    /// Машиночитаемый отчет в формате JSON для интеграции с CI.
    Json,
}

/// Формирует отчет анализа в выбранном формате.
///
/// :param analysis_report: Итоговый отчет анализа.
/// :param report_format: Формат вывода отчета.
/// :param verbose: Признак вывода дополнительной статистики.
/// :return: Готовый к печати текст отчета.
pub fn render_report(
    analysis_report: &AnalysisReport,
    report_format: ReportFormat,
    verbose: bool,
) -> String {
    match report_format {
        ReportFormat::Text => render_text(analysis_report, verbose),
        ReportFormat::Json => render_json(analysis_report),
    }
}

/// Формирует человекочитаемый текстовый отчет.
///
/// :param analysis_report: Итоговый отчет анализа.
/// :param verbose: Признак вывода дополнительной статистики.
/// :return: Текст отчета.
fn render_text(analysis_report: &AnalysisReport, verbose: bool) -> String {
    let mut output = String::new();

    if verbose {
        let _ = writeln!(
            output,
            "Проанализировано файлов: {}",
            analysis_report.analyzed_file_count
        );
        let _ = writeln!(
            output,
            "Извлечено сущностей: {}",
            analysis_report.extracted_entity_count
        );
        let _ = writeln!(
            output,
            "Пропущено файлов: {}",
            analysis_report.skipped_files.len()
        );
        let _ = writeln!(output);
    }

    if analysis_report.findings.is_empty() {
        let _ = writeln!(output, "Мертвый код не найден.");
        return output;
    }

    let _ = writeln!(
        output,
        "Найдено объектов мертвого кода: {}",
        analysis_report.findings.len()
    );
    let _ = writeln!(output);
    for finding in &analysis_report.findings {
        let _ = writeln!(
            output,
            "{}:{}: {} `{}` не используется",
            finding.file_path.display(),
            finding.line_number,
            finding.entity_kind.label(),
            finding.qualified_name,
        );
    }
    output
}

/// Формирует машиночитаемый отчет в формате JSON.
///
/// :param analysis_report: Итоговый отчет анализа.
/// :return: Текст отчета в формате JSON.
fn render_json(analysis_report: &AnalysisReport) -> String {
    let mut output = serde_json::to_string_pretty(analysis_report)
        .expect("сериализация отчета в JSON не может завершиться ошибкой");
    output.push('\n');
    output
}
