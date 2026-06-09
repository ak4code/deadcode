//! Загрузка конфигурации анализатора.
//!
//! Источники конфигурации в порядке приоритета:
//! 1. Явно указанный файл (`--config-path`).
//! 2. `.dc.toml` в корне анализируемого проекта.
//! 3. Секция `[tool.dc]` в `pyproject.toml` анализируемого проекта.
//! 4. Значения по умолчанию.

use std::path::Path;

use serde::Deserialize;

use crate::error::DcError;

/// Настройки анализатора.
///
/// Незнакомые ключи конфигурации считаются ошибкой: это защищает
/// пользователя от опечаток, незаметно отключающих настройки.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnalyzerConfiguration {
    /// Имена директорий, исключаемых из анализа.
    pub exclude_directories: Vec<String>,
    /// Дополнительные декораторы, помечающие функции как точки входа.
    ///
    /// Допускается полное точечное имя (`broker.subscribe`) или
    /// последний сегмент имени декоратора (`subscribe`).
    pub extra_entry_point_decorators: Vec<String>,
    /// Дополнительные имена, всегда считающиеся используемыми.
    ///
    /// Применяются для подавления ложных срабатываний на коде,
    /// вызываемом способами, неизвестными анализатору.
    pub extra_dynamic_names: Vec<String>,
    /// Дополнительные маркеры базовых классов под управлением фреймворка.
    ///
    /// Методы классов, унаследованных от базы с маркером в имени,
    /// вызываются фреймворком и не считаются мертвым кодом. Признак
    /// распространяется по иерархии наследования внутри проекта.
    pub extra_framework_base_markers: Vec<String>,
}

impl Default for AnalyzerConfiguration {
    fn default() -> Self {
        Self {
            exclude_directories: vec![
                ".venv".to_string(),
                "migrations".to_string(),
                "tests".to_string(),
            ],
            extra_entry_point_decorators: Vec::new(),
            extra_dynamic_names: Vec::new(),
            extra_framework_base_markers: Vec::new(),
        }
    }
}

/// Загружает конфигурацию анализатора.
///
/// Явно указанный, но отсутствующий файл считается ошибкой. Найденный,
/// но некорректный файл также считается ошибкой: молчаливый откат
/// к значениям по умолчанию скрывал бы проблемы конфигурации.
///
/// :param explicit_config_path: Явно указанный путь к файлу конфигурации.
/// :param target_path: Корневая директория анализируемого проекта.
/// :return: Конфигурация анализатора либо ошибка загрузки.
pub fn load_configuration(
    explicit_config_path: Option<&Path>,
    target_path: &Path,
) -> Result<AnalyzerConfiguration, DcError> {
    if let Some(config_path) = explicit_config_path {
        if !config_path.is_file() {
            return Err(DcError::ConfigurationNotFound {
                path: config_path.to_path_buf(),
            });
        }
        return parse_standalone_file(config_path);
    }

    let standalone_path = target_path.join(".dc.toml");
    if standalone_path.is_file() {
        return parse_standalone_file(&standalone_path);
    }

    let pyproject_path = target_path.join("pyproject.toml");
    if pyproject_path.is_file() {
        if let Some(configuration) = parse_pyproject_section(&pyproject_path)? {
            return Ok(configuration);
        }
    }

    Ok(AnalyzerConfiguration::default())
}

/// Читает содержимое файла конфигурации.
///
/// :param config_path: Путь к файлу конфигурации.
/// :return: Содержимое файла либо ошибка чтения.
fn read_configuration_file(config_path: &Path) -> Result<String, DcError> {
    std::fs::read_to_string(config_path).map_err(|read_error| DcError::ConfigurationInvalid {
        path: config_path.to_path_buf(),
        message: read_error.to_string(),
    })
}

/// Разбирает отдельный файл конфигурации `.dc.toml`.
///
/// :param config_path: Путь к файлу конфигурации.
/// :return: Конфигурация либо ошибка разбора.
fn parse_standalone_file(config_path: &Path) -> Result<AnalyzerConfiguration, DcError> {
    let raw_content = read_configuration_file(config_path)?;
    toml::from_str(&raw_content).map_err(|parse_error| DcError::ConfigurationInvalid {
        path: config_path.to_path_buf(),
        message: parse_error.to_string(),
    })
}

/// Разбирает секцию `[tool.dc]` файла `pyproject.toml`.
///
/// :param pyproject_path: Путь к файлу `pyproject.toml`.
/// :return: Конфигурация, `None` при отсутствии секции, либо ошибка разбора.
fn parse_pyproject_section(
    pyproject_path: &Path,
) -> Result<Option<AnalyzerConfiguration>, DcError> {
    let raw_content = read_configuration_file(pyproject_path)?;
    let document: toml::Value =
        toml::from_str(&raw_content).map_err(|parse_error| DcError::ConfigurationInvalid {
            path: pyproject_path.to_path_buf(),
            message: parse_error.to_string(),
        })?;
    let Some(dc_section) = document.get("tool").and_then(|tool| tool.get("dc")) else {
        return Ok(None);
    };
    dc_section
        .clone()
        .try_into()
        .map(Some)
        .map_err(
            |parse_error: toml::de::Error| DcError::ConfigurationInvalid {
                path: pyproject_path.to_path_buf(),
                message: parse_error.to_string(),
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration_excludes_service_directories() {
        let configuration = AnalyzerConfiguration::default();
        assert!(configuration
            .exclude_directories
            .contains(&".venv".to_string()));
        assert!(configuration
            .exclude_directories
            .contains(&"migrations".to_string()));
        assert!(configuration
            .exclude_directories
            .contains(&"tests".to_string()));
    }

    #[test]
    fn extra_framework_base_markers_are_parsed() {
        let configuration: AnalyzerConfiguration =
            toml::from_str("extra_framework_base_markers = [\"Repository\"]")
                .expect("корректная конфигурация");
        assert_eq!(
            configuration.extra_framework_base_markers,
            vec!["Repository".to_string()]
        );
    }

    #[test]
    fn unknown_configuration_keys_are_rejected() {
        let parse_result: Result<AnalyzerConfiguration, _> =
            toml::from_str("exclude_direcotries = [\".venv\"]");
        assert!(parse_result.is_err());
    }
}
