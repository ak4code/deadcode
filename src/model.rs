//! Доменные типы анализатора: сущности кода, ссылки и отчеты.

use std::path::PathBuf;

use serde::Serialize;

/// Вид извлеченной сущности кода.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    /// Функция уровня модуля.
    Function,
    /// Класс.
    Class,
    /// Метод класса.
    Method,
    /// Переменная уровня модуля.
    Variable,
}

impl EntityKind {
    /// Возвращает локализованное название вида сущности.
    ///
    /// :return: Название вида сущности для текстового отчета.
    pub fn label(&self) -> &'static str {
        match self {
            EntityKind::Function => "функция",
            EntityKind::Class => "класс",
            EntityKind::Method => "метод",
            EntityKind::Variable => "переменная",
        }
    }
}

/// Извлеченная из исходного кода сущность.
#[derive(Debug, Clone)]
pub struct CodeEntity {
    /// Простое имя сущности.
    pub simple_name: String,
    /// Полное точечное имя сущности.
    pub qualified_name: String,
    /// Полное имя области видимости, содержащей сущность.
    pub containing_scope: String,
    /// Вид сущности.
    pub kind: EntityKind,
    /// Путь к файлу с определением.
    pub file_path: PathBuf,
    /// Номер строки определения.
    pub line_number: usize,
    /// Признак точки входа анализа достижимости.
    pub is_entry_point: bool,
    /// Простые имена базовых классов: заполняется только для классов.
    pub superclass_names: Vec<String>,
}

/// Ссылка на имя из конкретной области видимости.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopedReference {
    /// Полное имя области видимости, содержащей ссылку.
    pub scope_qualified_name: String,
    /// Простое имя, на которое выполнена ссылка.
    pub referenced_name: String,
}

/// Результат анализа одного файла Python.
#[derive(Debug)]
pub struct FileAnalysis {
    /// Точечный путь модуля.
    pub module_path: String,
    /// Извлеченные сущности.
    pub entities: Vec<CodeEntity>,
    /// Уникальные ссылки на имена с привязкой к областям видимости.
    pub scoped_references: Vec<ScopedReference>,
    /// Пул динамических строковых ссылок.
    pub dynamic_references: Vec<String>,
}

/// Файл, пропущенный при анализе из-за ошибки чтения или парсинга.
#[derive(Debug, Clone, Serialize)]
pub struct SkippedFile {
    /// Путь к пропущенному файлу.
    pub file_path: PathBuf,
    /// Причина пропуска файла.
    pub reason: String,
}

/// Одна находка мертвого кода.
#[derive(Debug, Serialize)]
pub struct DeadCodeFinding {
    /// Путь к файлу с определением.
    pub file_path: PathBuf,
    /// Номер строки определения.
    pub line_number: usize,
    /// Вид сущности.
    #[serde(rename = "kind")]
    pub entity_kind: EntityKind,
    /// Полное точечное имя сущности.
    pub qualified_name: String,
}

impl From<&CodeEntity> for DeadCodeFinding {
    fn from(code_entity: &CodeEntity) -> Self {
        Self {
            file_path: code_entity.file_path.clone(),
            line_number: code_entity.line_number,
            entity_kind: code_entity.kind,
            qualified_name: code_entity.qualified_name.clone(),
        }
    }
}

/// Итоговый отчет анализа проекта.
#[derive(Debug, Serialize)]
pub struct AnalysisReport {
    /// Находки мертвого кода, отсортированные по расположению.
    pub findings: Vec<DeadCodeFinding>,
    /// Файлы, пропущенные из-за ошибок чтения или парсинга.
    pub skipped_files: Vec<SkippedFile>,
    /// Количество успешно проанализированных файлов.
    pub analyzed_file_count: usize,
    /// Количество извлеченных сущностей.
    pub extracted_entity_count: usize,
}
