//! Точка входа утилиты командной строки `dc`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use mimalloc::MiMalloc;

use dc::{
    load_configuration, render_report, run_analysis, AnalysisReport, DcError, EntityKind,
    ReportFormat,
};

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

    /// Виды сущностей в отчете. По умолчанию выводятся все виды.
    /// Допускает повторение и перечисление через запятую.
    #[arg(short, long = "kind", value_enum, value_delimiter = ',')]
    kinds: Vec<FindingKindArgument>,

    /// Вывод дополнительной статистики анализа.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

/// Вид сущности для фильтрации находок из командной строки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum FindingKindArgument {
    /// Функции уровня модуля.
    Function,
    /// Классы.
    Class,
    /// Методы классов.
    Method,
    /// Переменные уровня модуля.
    Variable,
}

impl From<FindingKindArgument> for EntityKind {
    fn from(kind_argument: FindingKindArgument) -> Self {
        match kind_argument {
            FindingKindArgument::Function => EntityKind::Function,
            FindingKindArgument::Class => EntityKind::Class,
            FindingKindArgument::Method => EntityKind::Method,
            FindingKindArgument::Variable => EntityKind::Variable,
        }
    }
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
    let mut analysis_report = run_analysis(target_path, &analyzer_configuration);
    filter_findings_by_kind(&mut analysis_report, &command_line_arguments.kinds);
    Ok(analysis_report)
}

/// Оставляет в отчете только находки запрошенных видов.
///
/// Пустой список видов означает отсутствие фильтрации. Код завершения
/// программы определяется уже отфильтрованными находками.
///
/// :param analysis_report: Итоговый отчет анализа.
/// :param requested_kinds: Виды сущностей из аргументов командной строки.
fn filter_findings_by_kind(
    analysis_report: &mut AnalysisReport,
    requested_kinds: &[FindingKindArgument],
) {
    if requested_kinds.is_empty() {
        return;
    }
    let allowed_kinds: Vec<EntityKind> = requested_kinds
        .iter()
        .map(|kind_argument| EntityKind::from(*kind_argument))
        .collect();
    analysis_report
        .findings
        .retain(|finding| allowed_kinds.contains(&finding.entity_kind));
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
