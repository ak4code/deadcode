//! Этап извлечения: парсинг файлов и обход синтаксических деревьев.
//!
//! Из каждого файла извлекаются определения сущностей, ссылки на имена
//! с привязкой к областям видимости и динамические строковые ссылки.

use std::collections::HashSet;
use std::path::{Component, Path};

use tree_sitter::{Node, Parser, Tree};

use crate::config::AnalyzerConfiguration;
use crate::heuristics;
use crate::model::{CodeEntity, EntityKind, FileAnalysis, ScopedReference, SkippedFile};

/// Парсер исходного кода Python, переиспользуемый рабочим потоком.
pub struct PythonSourceParser {
    inner_parser: Parser,
}

impl PythonSourceParser {
    /// Создает парсер с подключенной грамматикой Python.
    ///
    /// :return: Готовый к работе парсер.
    pub fn new() -> Self {
        let mut inner_parser = Parser::new();
        inner_parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .expect("грамматика tree-sitter-python несовместима с версией tree-sitter");
        Self { inner_parser }
    }

    /// Строит синтаксическое дерево исходного кода.
    ///
    /// :param source_code: Исходный код файла на Python.
    /// :return: Синтаксическое дерево либо `None` при сбое парсера.
    fn parse(&mut self, source_code: &str) -> Option<Tree> {
        self.inner_parser.parse(source_code, None)
    }
}

impl Default for PythonSourceParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Выполняет полный анализ одного файла Python.
///
/// :param source_parser: Переиспользуемый парсер рабочего потока.
/// :param file_path: Путь к файлу Python.
/// :param project_root: Корневая директория проекта.
/// :param configuration: Конфигурация анализатора.
/// :return: Результат анализа либо описание причины пропуска файла.
pub fn analyze_python_file(
    source_parser: &mut PythonSourceParser,
    file_path: &Path,
    project_root: &Path,
    configuration: &AnalyzerConfiguration,
) -> Result<FileAnalysis, SkippedFile> {
    let source_code = std::fs::read_to_string(file_path).map_err(|read_error| SkippedFile {
        file_path: file_path.to_path_buf(),
        reason: format!("ошибка чтения: {read_error}"),
    })?;
    let syntax_tree = source_parser
        .parse(&source_code)
        .ok_or_else(|| SkippedFile {
            file_path: file_path.to_path_buf(),
            reason: "парсер tree-sitter не построил синтаксическое дерево".to_string(),
        })?;

    let mut entity_extractor = EntityExtractor::new(
        &source_code,
        file_path,
        compute_module_path(file_path, project_root),
        configuration,
    );
    entity_extractor.visit_node(syntax_tree.root_node());
    Ok(entity_extractor.into_analysis())
}

/// Вычисляет точечный путь модуля по расположению файла.
///
/// :param file_path: Путь к файлу Python.
/// :param project_root: Корневая директория проекта.
/// :return: Точечный путь модуля вида `package.module`.
fn compute_module_path(file_path: &Path, project_root: &Path) -> String {
    let relative_path = file_path.strip_prefix(project_root).unwrap_or(file_path);
    let mut module_segments: Vec<String> = relative_path
        .components()
        .filter_map(|path_component| match path_component {
            Component::Normal(component_name) => {
                component_name.to_str().map(|name| name.to_string())
            }
            _ => None,
        })
        .collect();
    if let Some(last_segment) = module_segments.last_mut() {
        *last_segment = last_segment.trim_end_matches(".py").to_string();
    }
    if module_segments.last().map(String::as_str) == Some("__init__") {
        module_segments.pop();
    }
    module_segments.join(".")
}

/// Вид области видимости в стеке обхода.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Class,
    Function,
}

/// Область видимости в стеке обхода.
#[derive(Debug)]
struct Scope {
    /// Простое имя области видимости.
    name: String,
    /// Вид области видимости.
    kind: ScopeKind,
    /// Признак класса, методы которого вызывает фреймворк.
    is_framework_driven: bool,
}

/// Обходчик синтаксического дерева одного файла.
struct EntityExtractor<'source> {
    source_code: &'source str,
    file_path: &'source Path,
    module_path: String,
    configuration: &'source AnalyzerConfiguration,
    is_management_command_file: bool,
    scope_stack: Vec<Scope>,
    entities: Vec<CodeEntity>,
    references: HashSet<ScopedReference>,
    dynamic_references: HashSet<String>,
}

impl<'source> EntityExtractor<'source> {
    /// Создает обходчик для одного файла.
    ///
    /// :param source_code: Исходный код файла.
    /// :param file_path: Путь к файлу.
    /// :param module_path: Точечный путь модуля.
    /// :param configuration: Конфигурация анализатора.
    /// :return: Готовый к обходу экземпляр.
    fn new(
        source_code: &'source str,
        file_path: &'source Path,
        module_path: String,
        configuration: &'source AnalyzerConfiguration,
    ) -> Self {
        Self {
            source_code,
            file_path,
            module_path,
            configuration,
            is_management_command_file: heuristics::is_management_command_path(file_path),
            scope_stack: Vec::new(),
            entities: Vec::new(),
            references: HashSet::new(),
            dynamic_references: HashSet::new(),
        }
    }

    /// Завершает обход и возвращает результат анализа файла.
    ///
    /// :return: Результат анализа файла.
    fn into_analysis(self) -> FileAnalysis {
        FileAnalysis {
            module_path: self.module_path,
            entities: self.entities,
            scoped_references: self.references.into_iter().collect(),
            dynamic_references: self.dynamic_references.into_iter().collect(),
        }
    }

    /// Возвращает текст узла синтаксического дерева.
    ///
    /// :param node: Узел дерева tree-sitter.
    /// :return: Срез исходного кода, соответствующий узлу.
    fn node_text(&self, node: Node) -> &'source str {
        node.utf8_text(self.source_code.as_bytes()).unwrap_or("")
    }

    /// Возвращает полное имя текущей области видимости.
    ///
    /// :return: Точечное имя текущей области видимости.
    fn current_scope_qualified_name(&self) -> String {
        if self.scope_stack.is_empty() {
            return self.module_path.clone();
        }
        let scope_segments: Vec<&str> = self
            .scope_stack
            .iter()
            .map(|scope| scope.name.as_str())
            .collect();
        format!("{}.{}", self.module_path, scope_segments.join("."))
    }

    /// Рекурсивно обходит узлы синтаксического дерева.
    ///
    /// :param current_node: Текущий узел дерева tree-sitter.
    fn visit_node(&mut self, current_node: Node) {
        match current_node.kind() {
            "decorated_definition" => self.process_decorated_definition(current_node),
            "function_definition" | "class_definition" => {
                self.process_definition(current_node, &[]);
            }
            "call" => self.process_call(current_node),
            "assignment" => self.process_assignment(current_node),
            "identifier" => self.record_reference(self.node_text(current_node)),
            _ => self.visit_children(current_node),
        }
    }

    /// Обходит все дочерние узлы текущего узла.
    ///
    /// :param current_node: Текущий узел дерева tree-sitter.
    fn visit_children(&mut self, current_node: Node) {
        let mut tree_cursor = current_node.walk();
        for child_node in current_node.children(&mut tree_cursor) {
            self.visit_node(child_node);
        }
    }

    /// Записывает ссылку на имя из текущей области видимости.
    ///
    /// :param referenced_name: Простое имя, на которое выполнена ссылка.
    fn record_reference(&mut self, referenced_name: &str) {
        if referenced_name.is_empty() {
            return;
        }
        self.references.insert(ScopedReference {
            scope_qualified_name: self.current_scope_qualified_name(),
            referenced_name: referenced_name.to_string(),
        });
    }

    /// Обрабатывает определение с декораторами.
    ///
    /// Декораторы анализируются на принадлежность к точкам входа
    /// и на строковые ссылки `pytest.mark.usefixtures`.
    ///
    /// :param decorated_node: Узел `decorated_definition`.
    fn process_decorated_definition(&mut self, decorated_node: Node) {
        let mut decorator_names = Vec::new();
        let mut tree_cursor = decorated_node.walk();
        let decorator_nodes: Vec<Node> = decorated_node
            .children(&mut tree_cursor)
            .filter(|child_node| child_node.kind() == "decorator")
            .collect();

        for decorator_node in decorator_nodes {
            let normalized_decorator =
                heuristics::normalize_decorator_expression(self.node_text(decorator_node));
            if normalized_decorator.contains("usefixtures") {
                self.collect_string_literals_into_pool(decorator_node);
            }
            decorator_names.push(normalized_decorator);
            self.visit_children(decorator_node);
        }

        if let Some(definition_node) = decorated_node.child_by_field_name("definition") {
            self.process_definition(definition_node, &decorator_names);
        }
    }

    /// Обрабатывает определение функции, метода или класса.
    ///
    /// :param definition_node: Узел определения.
    /// :param decorator_names: Нормализованные имена декораторов определения.
    fn process_definition(&mut self, definition_node: Node, decorator_names: &[String]) {
        let Some(name_node) = definition_node.child_by_field_name("name") else {
            return;
        };
        let simple_name = self.node_text(name_node).to_string();
        let containing_scope = self.current_scope_qualified_name();
        let qualified_name = format!("{containing_scope}.{simple_name}");
        let is_class_definition = definition_node.kind() == "class_definition";
        let entity_kind = if is_class_definition {
            EntityKind::Class
        } else if matches!(
            self.scope_stack.last(),
            Some(Scope {
                kind: ScopeKind::Class,
                ..
            })
        ) {
            EntityKind::Method
        } else {
            EntityKind::Function
        };
        let is_entry_point =
            self.determine_entry_point(&simple_name, entity_kind, definition_node, decorator_names);

        self.entities.push(CodeEntity {
            simple_name: simple_name.clone(),
            qualified_name,
            containing_scope,
            kind: entity_kind,
            file_path: self.file_path.to_path_buf(),
            line_number: name_node.start_position().row + 1,
            is_entry_point,
        });

        if is_class_definition {
            self.process_admin_class_attributes(definition_node);
        }

        let scope_kind = if is_class_definition {
            ScopeKind::Class
        } else {
            ScopeKind::Function
        };
        let is_framework_driven = is_class_definition
            && heuristics::is_framework_driven_base(self.superclasses_text(definition_node));
        self.scope_stack.push(Scope {
            name: simple_name,
            kind: scope_kind,
            is_framework_driven,
        });
        let mut tree_cursor = definition_node.walk();
        let child_nodes: Vec<Node> = definition_node.children(&mut tree_cursor).collect();
        for child_node in child_nodes {
            if child_node.id() == name_node.id() {
                continue;
            }
            self.visit_node(child_node);
        }
        self.scope_stack.pop();
    }

    /// Определяет принадлежность сущности к точкам входа анализа.
    ///
    /// :param simple_name: Простое имя сущности.
    /// :param entity_kind: Вид сущности.
    /// :param definition_node: Узел определения.
    /// :param decorator_names: Нормализованные имена декораторов.
    /// :return: Признак точки входа.
    fn determine_entry_point(
        &self,
        simple_name: &str,
        entity_kind: EntityKind,
        definition_node: Node,
        decorator_names: &[String],
    ) -> bool {
        if heuristics::is_dunder_name(simple_name) {
            return true;
        }
        match entity_kind {
            EntityKind::Class => {
                if self.is_management_command_file && simple_name == "Command" {
                    return true;
                }
                if heuristics::is_implicit_class_name(simple_name) {
                    return true;
                }
                if decorator_names
                    .iter()
                    .any(|decorator| heuristics::is_admin_register_decorator(decorator))
                {
                    return true;
                }
                heuristics::is_app_config_class(
                    &self.module_path,
                    self.superclasses_text(definition_node),
                )
            }
            EntityKind::Function | EntityKind::Method => {
                let is_entry_by_decorator = decorator_names.iter().any(|decorator| {
                    heuristics::is_entry_point_decorator(decorator)
                        || heuristics::matches_configured_decorator(
                            decorator,
                            &self.configuration.extra_entry_point_decorators,
                        )
                });
                if is_entry_by_decorator {
                    return true;
                }
                if heuristics::is_test_function_name(simple_name) {
                    return true;
                }
                let is_property = decorator_names
                    .iter()
                    .any(|decorator| heuristics::is_property_decorator(decorator));
                entity_kind == EntityKind::Method
                    && (is_property
                        || heuristics::is_implicit_method_name(simple_name)
                        || self.is_inside_framework_driven_class())
            }
            EntityKind::Variable => false,
        }
    }

    /// Возвращает текст списка базовых классов определения класса.
    ///
    /// :param definition_node: Узел `class_definition`.
    /// :return: Текст списка базовых классов либо пустая строка.
    fn superclasses_text(&self, definition_node: Node) -> &'source str {
        definition_node
            .child_by_field_name("superclasses")
            .map(|superclasses_node| self.node_text(superclasses_node))
            .unwrap_or("")
    }

    /// Проверяет, объявлен ли метод внутри класса, управляемого фреймворком.
    ///
    /// :return: Признак метода класса, унаследованного от базы фреймворка.
    fn is_inside_framework_driven_class(&self) -> bool {
        matches!(
            self.scope_stack.last(),
            Some(Scope {
                kind: ScopeKind::Class,
                is_framework_driven: true,
                ..
            })
        )
    }

    /// Обрабатывает вызов функции и применяет эвристики динамических ссылок.
    ///
    /// :param call_node: Узел `call`.
    fn process_call(&mut self, call_node: Node) {
        if let Some(function_node) = call_node.child_by_field_name("function") {
            let function_text = self.node_text(function_node);
            let function_name = heuristics::last_dotted_segment(function_text);
            let positional_arguments = self.collect_positional_arguments(call_node);

            if heuristics::DYNAMIC_REFERENCE_BUILTINS.contains(&function_name) {
                self.add_string_argument_to_pool(positional_arguments.get(1).copied());
            } else if function_name == "import_module" {
                self.add_string_argument_to_pool(positional_arguments.first().copied());
            } else if heuristics::URL_REGISTRATION_FUNCTIONS.contains(&function_name) {
                self.process_url_registration(positional_arguments.get(1).copied());
            }
        }
        self.visit_children(call_node);
    }

    /// Собирает позиционные аргументы вызова.
    ///
    /// :param call_node: Узел `call`.
    /// :return: Список узлов позиционных аргументов.
    fn collect_positional_arguments<'tree>(&self, call_node: Node<'tree>) -> Vec<Node<'tree>> {
        let Some(arguments_node) = call_node.child_by_field_name("arguments") else {
            return Vec::new();
        };
        let mut tree_cursor = arguments_node.walk();
        arguments_node
            .named_children(&mut tree_cursor)
            .filter(|argument_node| {
                argument_node.kind() != "keyword_argument" && argument_node.kind() != "comment"
            })
            .collect()
    }

    /// Добавляет строковый аргумент вызова в пул динамических ссылок.
    ///
    /// :param argument_node: Узел аргумента вызова.
    fn add_string_argument_to_pool(&mut self, argument_node: Option<Node>) {
        let Some(argument_node) = argument_node else {
            return;
        };
        if let Some(literal_value) = self.string_literal_value(argument_node) {
            self.dynamic_references.insert(literal_value);
        }
    }

    /// Обрабатывает аргумент представления в вызовах `path`, `re_path`, `url`.
    ///
    /// Строковая ссылка вида `myapp.views.my_view` разрешается до имени
    /// функции. Прямая ссылка на функцию помечает ее как точку входа.
    ///
    /// :param view_argument: Узел второго позиционного аргумента.
    fn process_url_registration(&mut self, view_argument: Option<Node>) {
        let Some(view_argument) = view_argument else {
            return;
        };
        match view_argument.kind() {
            "string" => {
                if let Some(literal_value) = self.string_literal_value(view_argument) {
                    self.dynamic_references.insert(literal_value);
                }
            }
            "identifier" | "attribute" => {
                let view_name = heuristics::last_dotted_segment(self.node_text(view_argument));
                self.dynamic_references.insert(view_name.to_string());
            }
            _ => {}
        }
    }

    /// Обрабатывает присваивание значения переменной.
    ///
    /// Переменные уровня модуля регистрируются как сущности. Строки из
    /// списка `__all__` добавляются в пул динамических ссылок. Левая часть
    /// присваивания не считается ссылкой на имя.
    ///
    /// :param assignment_node: Узел `assignment`.
    fn process_assignment(&mut self, assignment_node: Node) {
        let left_node = assignment_node.child_by_field_name("left");
        if let Some(left_node) = left_node {
            if left_node.kind() == "identifier" {
                let variable_name = self.node_text(left_node).to_string();
                if variable_name == "__all__" {
                    self.collect_string_literals_into_pool(assignment_node);
                } else if self.scope_stack.is_empty() {
                    self.register_module_variable(&variable_name, left_node);
                }
            }
        }

        let mut tree_cursor = assignment_node.walk();
        let child_nodes: Vec<Node> = assignment_node.children(&mut tree_cursor).collect();
        for child_node in child_nodes {
            let is_assigned_identifier = left_node
                .map(|left| left.id() == child_node.id() && left.kind() == "identifier")
                .unwrap_or(false);
            if is_assigned_identifier {
                continue;
            }
            self.visit_node(child_node);
        }
    }

    /// Регистрирует переменную уровня модуля как сущность.
    ///
    /// :param variable_name: Простое имя переменной.
    /// :param name_node: Узел имени переменной.
    fn register_module_variable(&mut self, variable_name: &str, name_node: Node) {
        let containing_scope = self.current_scope_qualified_name();
        let is_entry_point = heuristics::is_dunder_name(variable_name)
            || heuristics::is_implicit_variable_name(variable_name)
            || heuristics::is_settings_module(&self.module_path);
        self.entities.push(CodeEntity {
            simple_name: variable_name.to_string(),
            qualified_name: format!("{containing_scope}.{variable_name}"),
            containing_scope,
            kind: EntityKind::Variable,
            file_path: self.file_path.to_path_buf(),
            line_number: name_node.start_position().row + 1,
            is_entry_point,
        });
    }

    /// Извлекает строковые ссылки из атрибутов класса `admin.ModelAdmin`.
    ///
    /// Проверяются коллекции `list_display`, `list_filter`, `actions`
    /// и `readonly_fields`.
    ///
    /// :param class_node: Узел `class_definition`.
    fn process_admin_class_attributes(&mut self, class_node: Node) {
        if !self.superclasses_text(class_node).contains("ModelAdmin") {
            return;
        }
        let Some(body_node) = class_node.child_by_field_name("body") else {
            return;
        };
        let mut body_cursor = body_node.walk();
        let statement_nodes: Vec<Node> = body_node.named_children(&mut body_cursor).collect();
        for statement_node in statement_nodes {
            let Some(assignment_node) = statement_node.named_child(0) else {
                continue;
            };
            if assignment_node.kind() != "assignment" {
                continue;
            }
            let Some(left_node) = assignment_node.child_by_field_name("left") else {
                continue;
            };
            let attribute_name = self.node_text(left_node);
            if !heuristics::ADMIN_DYNAMIC_ATTRIBUTES.contains(&attribute_name) {
                continue;
            }
            if let Some(right_node) = assignment_node.child_by_field_name("right") {
                if matches!(right_node.kind(), "list" | "tuple" | "set") {
                    self.collect_string_literals_into_pool(right_node);
                }
            }
        }
    }

    /// Собирает все строковые литералы поддерева в пул динамических ссылок.
    ///
    /// :param subtree_root: Корневой узел поддерева.
    fn collect_string_literals_into_pool(&mut self, subtree_root: Node) {
        if let Some(literal_value) = self.string_literal_value(subtree_root) {
            self.dynamic_references.insert(literal_value);
            return;
        }
        let mut tree_cursor = subtree_root.walk();
        let child_nodes: Vec<Node> = subtree_root.children(&mut tree_cursor).collect();
        for child_node in child_nodes {
            self.collect_string_literals_into_pool(child_node);
        }
    }

    /// Извлекает значение строкового литерала.
    ///
    /// :param node: Проверяемый узел дерева.
    /// :return: Содержимое строки либо `None` для других видов узлов.
    fn string_literal_value(&self, node: Node) -> Option<String> {
        if node.kind() != "string" {
            return None;
        }
        let mut tree_cursor = node.walk();
        for child_node in node.children(&mut tree_cursor) {
            if child_node.kind() == "string_content" {
                return Some(self.node_text(child_node).to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_path_is_computed_from_relative_location() {
        let module_path =
            compute_module_path(Path::new("/project/shop/views.py"), Path::new("/project"));
        assert_eq!(module_path, "shop.views");
    }

    #[test]
    fn package_init_module_path_drops_init_segment() {
        let module_path = compute_module_path(
            Path::new("/project/shop/__init__.py"),
            Path::new("/project"),
        );
        assert_eq!(module_path, "shop");
    }
}
