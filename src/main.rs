//! Точка входа утилиты командной строки `dc`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use mimalloc::MiMalloc;

use dc::{load_configuration, render_report, run_analysis, AnalysisReport, DcError, ReportFormat};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// Код завершения при обнаружении мертвого кода.
const EXIT_CODE_FINDINGS: u8 = 1;

/// Код завершения при ошибке конфигурации или запуска.
const EXIT_CODE_ERROR: u8 = 2;

#[derive(Parser, Debug)]
#[command(
    name = "dc",
    version,
    about = "Поиск мертвого кода в Django проектах",
    after_help = "Коды завершения:\n  0 — мертвый код не найден\n  1 — найден мертвый код\n  2 — ошибка конфигурации или запуска"
)]
struct CommandLineArguments {
    /// Корневая директория анализируемого проекта.
    #[arg(short, long, default_value = ".")]
    target_path: PathBuf,

    /// Путь к файлу конфигурации. По умолчанию ищутся `.dc.toml`
    /// и секция `[tool.dc]` в `pyproject.toml` корня проекта.
    #[arg(short, long)]
    config_path: Option<PathBuf>,

    /// Формат вывода отчета.
    #[arg(short, long, value_enum, default_value_t = ReportFormat::Text)]
    format: ReportFormat,

    /// Вывод дополнительной статистики анализа.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

/// Инициализирует процесс анализа и выводит результаты.
///
/// :return: Статус завершения программы.
fn main() -> ExitCode {
    let command_line_arguments = CommandLineArguments::parse();
    match execute(&command_line_arguments) {
        Ok(analysis_report) => {
            report_skipped_files(&analysis_report);
            print!(
                "{}",
                render_report(
                    &analysis_report,
                    command_line_arguments.format,
                    command_line_arguments.verbose
                )
            );
            if analysis_report.findings.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(EXIT_CODE_FINDINGS)
            }
        }
        Err(execution_error) => {
            eprintln!("dc: {execution_error}");
            ExitCode::from(EXIT_CODE_ERROR)
        }
    }
}

/// Выполняет загрузку конфигурации и запуск анализа.
///
/// :param command_line_arguments: Разобранные аргументы командной строки.
/// :return: Итоговый отчет анализа либо ошибка выполнения.
fn execute(command_line_arguments: &CommandLineArguments) -> Result<AnalysisReport, DcError> {
    let target_path: &Path = &command_line_arguments.target_path;
    if !target_path.is_dir() {
        return Err(DcError::TargetNotFound {
            path: target_path.to_path_buf(),
        });
    }
    let analyzer_configuration =
        load_configuration(command_line_arguments.config_path.as_deref(), target_path)?;
    Ok(run_analysis(target_path, &analyzer_configuration))
}

/// Выводит предупреждения о пропущенных файлах в поток ошибок.
///
/// :param analysis_report: Итоговый отчет анализа.
fn report_skipped_files(analysis_report: &AnalysisReport) {
    for skipped_file in &analysis_report.skipped_files {
        eprintln!(
            "dc: пропущен файл {}: {}",
            skipped_file.file_path.display(),
            skipped_file.reason
        );
    }
}
