use std::path::{Component, Path, PathBuf};

use ignore::WalkBuilder;

use crate::configuration::AnalyzerConfiguration;

/// Собирает пути ко всем анализируемым файлам Python в целевой директории.
///
/// Обход дерева каталогов выполняется с учетом правил `.gitignore`.
/// Скрытые директории и директории из списка исключений пропускаются.
///
/// :param target_path: Корневая директория анализируемого проекта.
/// :param configuration: Конфигурация анализатора со списком исключений.
/// :return: Отсортированный список путей к файлам Python.
pub fn collect_python_files(target_path: &str, configuration: &AnalyzerConfiguration) -> Vec<PathBuf> {
    let project_root = Path::new(target_path);
    let mut python_files = Vec::new();

    for walk_entry in WalkBuilder::new(project_root).build() {
        let Ok(directory_entry) = walk_entry else {
            continue;
        };
        let entry_path = directory_entry.path();
        let is_regular_file = directory_entry
            .file_type()
            .map(|file_type| file_type.is_file())
            .unwrap_or(false);
        if !is_regular_file {
            continue;
        }
        if entry_path.extension().and_then(|extension| extension.to_str()) != Some("py") {
            continue;
        }
        if is_excluded_path(entry_path, project_root, &configuration.exclude_directories) {
            continue;
        }
        python_files.push(entry_path.to_path_buf());
    }

    python_files.sort();
    python_files
}

/// Проверяет вхождение пути в список исключенных директорий.
///
/// Сравнение выполняется по компонентам пути относительно корня проекта.
///
/// :param entry_path: Полный путь к проверяемому файлу.
/// :param project_root: Корневая директория проекта.
/// :param excluded_directories: Имена исключенных директорий.
/// :return: Признак исключения файла из анализа.
fn is_excluded_path(entry_path: &Path, project_root: &Path, excluded_directories: &[String]) -> bool {
    let relative_path = entry_path.strip_prefix(project_root).unwrap_or(entry_path);
    relative_path.components().any(|path_component| match path_component {
        Component::Normal(component_name) => component_name
            .to_str()
            .map(|name| excluded_directories.iter().any(|excluded| excluded == name))
            .unwrap_or(false),
        _ => false,
    })
}
