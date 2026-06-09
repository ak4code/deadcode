//! Типы ошибок утилиты `dc`.

use std::fmt;
use std::path::PathBuf;

/// Ошибка работы утилиты `dc`.
///
/// Ошибки конфигурации фатальны и завершают программу с кодом `2`.
/// Ошибки чтения и парсинга отдельных файлов проекта не прерывают анализ:
/// они накапливаются в отчете как пропущенные файлы.
#[derive(Debug)]
pub enum DcError {
    /// Файл конфигурации не найден по явно указанному пути.
    ConfigurationNotFound {
        /// Путь к отсутствующему файлу конфигурации.
        path: PathBuf,
    },
    /// Файл конфигурации не удалось прочитать или разобрать.
    ConfigurationInvalid {
        /// Путь к файлу конфигурации.
        path: PathBuf,
        /// Описание ошибки чтения или разбора.
        message: String,
    },
    /// Целевая директория анализа не существует.
    TargetNotFound {
        /// Путь к отсутствующей директории.
        path: PathBuf,
    },
}

impl fmt::Display for DcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DcError::ConfigurationNotFound { path } => {
                write!(formatter, "файл конфигурации не найден: {}", path.display())
            }
            DcError::ConfigurationInvalid { path, message } => {
                write!(
                    formatter,
                    "некорректная конфигурация {}: {message}",
                    path.display()
                )
            }
            DcError::TargetNotFound { path } => {
                write!(
                    formatter,
                    "целевая директория не существует: {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DcError {}
