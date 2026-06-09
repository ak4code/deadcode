use std::path::Path;

use serde::Deserialize;

/// Настройки анализатора, загружаемые из `.dc.toml` или секции `[tool.dc]`
/// файла `pyproject.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AnalyzerConfiguration {
    /// Имена директорий, исключаемых из анализа.
    pub exclude_directories: Vec<String>,
}

impl Default for AnalyzerConfiguration {
    fn default() -> Self {
        Self {
            exclude_directories: vec![
                ".venv".to_string(),
                "migrations".to_string(),
                "tests".to_string(),
            ],
        }
    }
}

/// Загружает конфигурацию анализатора.
///
/// Сначала читается отдельный файл конфигурации. Если он отсутствует,
/// анализатор ищет секцию `[tool.dc]` в `pyproject.toml` текущей директории.
/// При отсутствии обоих источников используются значения по умолчанию.
///
/// :param config_path: Путь к файлу конфигурации `.dc.toml`.
/// :return: Заполненная конфигурация анализатора.
pub fn load_configuration(config_path: &str) -> AnalyzerConfiguration {
    if let Some(configuration) = load_from_standalone_file(Path::new(config_path)) {
        return configuration;
    }
    if let Some(configuration) = load_from_pyproject(Path::new("pyproject.toml")) {
        return configuration;
    }
    AnalyzerConfiguration::default()
}

/// Читает конфигурацию из отдельного файла `.dc.toml`.
///
/// :param standalone_path: Путь к файлу конфигурации.
/// :return: Конфигурация либо `None`, если файл отсутствует или некорректен.
fn load_from_standalone_file(standalone_path: &Path) -> Option<AnalyzerConfiguration> {
    let raw_content = std::fs::read_to_string(standalone_path).ok()?;
    toml::from_str(&raw_content).ok()
}

/// Читает конфигурацию из секции `[tool.dc]` файла `pyproject.toml`.
///
/// :param pyproject_path: Путь к файлу `pyproject.toml`.
/// :return: Конфигурация либо `None`, если секция отсутствует.
fn load_from_pyproject(pyproject_path: &Path) -> Option<AnalyzerConfiguration> {
    let raw_content = std::fs::read_to_string(pyproject_path).ok()?;
    let document: toml::Value = toml::from_str(&raw_content).ok()?;
    let dc_section = document.get("tool")?.get("dc")?.clone();
    dc_section.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration_excludes_service_directories() {
        let configuration = AnalyzerConfiguration::default();
        assert!(configuration.exclude_directories.contains(&".venv".to_string()));
        assert!(configuration.exclude_directories.contains(&"migrations".to_string()));
        assert!(configuration.exclude_directories.contains(&"tests".to_string()));
    }
}
