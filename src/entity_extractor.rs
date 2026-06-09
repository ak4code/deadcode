use std::path::{Component, Path, PathBuf};

use tree_sitter::{Node, Parser};

use crate::django_heuristics;

/// Вид извлеченной сущности кода.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Function,
    Class,
    Method,
    Variable,
}

impl EntityKind {
    /// Возвращает локализованное название вида сущности.
    ///
    /// :return: Название вида сущности для отчета.
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
}

/// Ссылка на имя из конкретной области видимости.
#[derive(Debug)]
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
    /// Ссылки на имена с привязкой к областям видимости.
    pub scoped_references: Vec<ScopedReference>,
    /// Пул динамических строковых ссылок.
    pub dynamic_references: Vec<String>,
}

/// Вид области видимости в стеке обхода.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Class,
    Function,
}

/// Обходчик синтаксического дерева одного файла.
struct EntityExtractor<'source> {
    source_code: &'source str,
    file_path: &'source Path,
    is_management_command_file: bool,
    scope_stack: Vec<(String, ScopeKind)>,
    analysis: FileAnalysis,
}

/// Выполняет полный анализ одного файла Python.
///
/// Файл читается с диска, парсится `tree-sitter` и обходится
/// для извлечения сущностей, ссылок и динамических строк.
///
/// :param file_path: Путь к файлу Python.
/// :param project_root: Корневая директория проекта.
/// :return: Результат анализа либо `None` при ошибке чтения или парсинга.
pub fn analyze_python_file(file_path: &Path, project_root: &Path) -> Option<FileAnalysis> {
    let source_code = std::fs::read_to_string(file_path).ok()?;
    let mut python_parser = Parser::new();
    python_parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .ok()?;
    let syntax_tree = python_parser.parse(&source_code, None)?;

    let mut extractor = EntityExtractor {
        source_code: &source_code,
        file_path,
        is_management_command_file: django_heuristics::is_management_command_path(file_path),
        scope_stack: Vec::new(),
        analysis: FileAnalysis {
            module_path: compute_module_path(file_path, project_root),
            entities: Vec::new(),
            scoped_references: Vec::new(),
            dynamic_references: Vec::new(),
        },
    };
    extractor.visit_node(syntax_tree.root_node());
    Some(extractor.analysis)
}

/// Вычисляет точечный путь модуля по расположению файла.
///
/// :param file_path: Путь к файлу Python.
/// :param project_root: Корневая директория проекта.
/// :return: Точечный путь модуля вида `package.module`.
pub fn compute_module_path(file_path: &Path, project_root: &Path) -> String {
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

impl<'source> EntityExtractor<'source> {
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
            return self.analysis.module_path.clone();
        }
        let scope_segments: Vec<&str> = self
            .scope_stack
            .iter()
            .map(|(scope_name, _)| scope_name.as_str())
            .collect();
        format!("{}.{}", self.analysis.module_path, scope_segments.join("."))
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
        let scope_qualified_name = self.current_scope_qualified_name();
        self.analysis.scoped_references.push(ScopedReference {
            scope_qualified_name,
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
                django_heuristics::normalize_decorator_expression(self.node_text(decorator_node));
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
        } else if matches!(self.scope_stack.last(), Some((_, ScopeKind::Class))) {
            EntityKind::Method
        } else {
            EntityKind::Function
        };
        let is_entry_point =
            self.determine_entry_point(&simple_name, entity_kind, definition_node, decorator_names);

        self.analysis.entities.push(CodeEntity {
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
        self.scope_stack.push((simple_name, scope_kind));
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
        if django_heuristics::is_dunder_name(simple_name) {
            return true;
        }
        match entity_kind {
            EntityKind::Class => {
                if self.is_management_command_file && simple_name == "Command" {
                    return true;
                }
                if django_heuristics::is_implicit_class_name(simple_name) {
                    return true;
                }
                if decorator_names
                    .iter()
                    .any(|decorator| django_heuristics::is_admin_register_decorator(decorator))
                {
                    return true;
                }
                let superclasses_text = definition_node
                    .child_by_field_name("superclasses")
                    .map(|superclasses_node| self.node_text(superclasses_node))
                    .unwrap_or("");
                django_heuristics::is_app_config_class(
                    &self.analysis.module_path,
                    superclasses_text,
                )
            }
            EntityKind::Function | EntityKind::Method => {
                if decorator_names
                    .iter()
                    .any(|decorator| django_heuristics::is_entry_point_decorator(decorator))
                {
                    return true;
                }
                entity_kind == EntityKind::Method
                    && django_heuristics::is_implicit_method_name(simple_name)
            }
            EntityKind::Variable => false,
        }
    }

    /// Обрабатывает вызов функции и применяет эвристики динамических ссылок.
    ///
    /// :param call_node: Узел `call`.
    fn process_call(&mut self, call_node: Node) {
        if let Some(function_node) = call_node.child_by_field_name("function") {
            let function_text = self.node_text(function_node);
            let function_name = django_heuristics::last_dotted_segment(function_text);
            let positional_arguments = self.collect_positional_arguments(call_node);

            if django_heuristics::DYNAMIC_REFERENCE_BUILTINS.contains(&function_name) {
                self.add_string_argument_to_pool(positional_arguments.get(1).copied());
            } else if function_name == "import_module" {
                self.add_string_argument_to_pool(positional_arguments.first().copied());
            } else if django_heuristics::URL_REGISTRATION_FUNCTIONS.contains(&function_name) {
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
            self.analysis.dynamic_references.push(literal_value);
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
                    self.analysis.dynamic_references.push(literal_value);
                }
            }
            "identifier" | "attribute" => {
                let view_name =
                    django_heuristics::last_dotted_segment(self.node_text(view_argument));
                self.analysis.dynamic_references.push(view_name.to_string());
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
                    let containing_scope = self.current_scope_qualified_name();
                    let is_entry_point = django_heuristics::is_dunder_name(&variable_name)
                        || django_heuristics::is_implicit_variable_name(&variable_name)
                        || django_heuristics::is_settings_module(&self.analysis.module_path);
                    self.analysis.entities.push(CodeEntity {
                        simple_name: variable_name.clone(),
                        qualified_name: format!("{containing_scope}.{variable_name}"),
                        containing_scope,
                        kind: EntityKind::Variable,
                        file_path: self.file_path.to_path_buf(),
                        line_number: left_node.start_position().row + 1,
                        is_entry_point,
                    });
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

    /// Извлекает строковые ссылки из атрибутов класса `admin.ModelAdmin`.
    ///
    /// Проверяются коллекции `list_display`, `list_filter`, `actions`
    /// и `readonly_fields`.
    ///
    /// :param class_node: Узел `class_definition`.
    fn process_admin_class_attributes(&mut self, class_node: Node) {
        let superclasses_text = class_node
            .child_by_field_name("superclasses")
            .map(|superclasses_node| self.node_text(superclasses_node))
            .unwrap_or("");
        if !superclasses_text.contains("ModelAdmin") {
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
            if !django_heuristics::ADMIN_DYNAMIC_ATTRIBUTES.contains(&attribute_name) {
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
            self.analysis.dynamic_references.push(literal_value);
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
        let module_path = compute_module_path(
            Path::new("/project/shop/views.py"),
            Path::new("/project"),
        );
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
