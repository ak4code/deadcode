//! Этап сбора файлов: обход дерева каталогов проекта.

use std::path::{Component, Path, PathBuf};

use ignore::WalkBuilder;

use crate::config::AnalyzerConfiguration;

/// Собирает пути ко всем анализируемым файлам Python в целевой директории.
///
/// Обход дерева каталогов выполняется с учетом правил `.gitignore`.
/// Скрытые директории и директории из списка исключений пропускаются.
/// Результат отсортирован для детерминированности анализа.
///
/// :param project_root: Корневая директория анализируемого проекта.
/// :param configuration: Конфигурация анализатора со списком исключений.
/// :return: Отсортированный список путей к файлам Python.
pub fn collect_python_files(
    project_root: &Path,
    configuration: &AnalyzerConfiguration,
) -> Vec<PathBuf> {
    let mut python_files: Vec<PathBuf> = WalkBuilder::new(project_root)
        .build()
        .filter_map(Result::ok)
        .filter(|directory_entry| {
            directory_entry
                .file_type()
                .map(|file_type| file_type.is_file())
                .unwrap_or(false)
        })
        .map(|directory_entry| directory_entry.into_path())
        .filter(|entry_path| {
            entry_path
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("py")
        })
        .filter(|entry_path| {
            !is_excluded_path(entry_path, project_root, &configuration.exclude_directories)
        })
        .collect();

    python_files.sort();
    python_files
}

/// Проверяет вхождение пути в список исключенных директорий.
///
/// Сравнение выполняется по компонентам пути относительно корня проекта:
/// имена директорий выше корня не влияют на результат.
///
/// :param entry_path: Полный путь к проверяемому файлу.
/// :param project_root: Корневая директория проекта.
/// :param excluded_directories: Имена исключенных директорий.
/// :return: Признак исключения файла из анализа.
fn is_excluded_path(
    entry_path: &Path,
    project_root: &Path,
    excluded_directories: &[String],
) -> bool {
    let relative_path = entry_path.strip_prefix(project_root).unwrap_or(entry_path);
    relative_path
        .components()
        .any(|path_component| match path_component {
            Component::Normal(component_name) => component_name
                .to_str()
                .map(|name| excluded_directories.iter().any(|excluded| excluded == name))
                .unwrap_or(false),
            _ => false,
        })
}
