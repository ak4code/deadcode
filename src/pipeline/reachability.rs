//! Этапы построения графа зависимостей и вычисления достижимости.

use std::collections::{HashMap, HashSet};

use petgraph::graph::{DiGraph, NodeIndex};

use crate::config::AnalyzerConfiguration;
use crate::heuristics;
use crate::model::{CodeEntity, FileAnalysis};

/// Вычисляет недостижимые сущности по результатам анализа файлов.
///
/// Строится ориентированный граф зависимостей. Узлами выступают модули
/// и извлеченные сущности. Ребра создаются по ссылкам на имена. Обход
/// графа выполняется от точек входа. Сущности вне множества достижимости
/// считаются мертвым кодом.
///
/// :param file_analyses: Результаты анализа всех файлов проекта.
/// :param configuration: Конфигурация анализатора.
/// :return: Список недостижимых сущностей, отсортированный по расположению.
pub fn find_unreachable_entities<'analysis>(
    file_analyses: &'analysis [FileAnalysis],
    configuration: &'analysis AnalyzerConfiguration,
) -> Vec<&'analysis CodeEntity> {
    let mut dependency_graph = DiGraph::<&str, ()>::new();
    let mut scope_nodes: HashMap<&str, NodeIndex> = HashMap::new();
    let mut name_index: HashMap<&str, Vec<NodeIndex>> = HashMap::new();
    let mut entities_by_node: HashMap<NodeIndex, &CodeEntity> = HashMap::new();

    for file_analysis in file_analyses {
        let module_node = dependency_graph.add_node(file_analysis.module_path.as_str());
        scope_nodes.insert(file_analysis.module_path.as_str(), module_node);
    }

    for file_analysis in file_analyses {
        for code_entity in &file_analysis.entities {
            let entity_node = dependency_graph.add_node(code_entity.qualified_name.as_str());
            scope_nodes.insert(code_entity.qualified_name.as_str(), entity_node);
            name_index
                .entry(code_entity.simple_name.as_str())
                .or_default()
                .push(entity_node);
            entities_by_node.insert(entity_node, code_entity);
        }
    }

    add_reference_edges(
        file_analyses,
        &mut dependency_graph,
        &scope_nodes,
        &name_index,
    );
    add_containment_edges(&entities_by_node, &mut dependency_graph, &scope_nodes);

    let dynamic_reference_pool = build_dynamic_reference_pool(file_analyses, configuration);
    let reachable_nodes = compute_reachable_nodes(
        file_analyses,
        &dependency_graph,
        &scope_nodes,
        &entities_by_node,
        &dynamic_reference_pool,
    );

    let mut unreachable_entities: Vec<&CodeEntity> = entities_by_node
        .iter()
        .filter(|(node_index, _)| !reachable_nodes.contains(node_index))
        .map(|(_, code_entity)| *code_entity)
        .collect();
    unreachable_entities.sort_by(|first, second| {
        first
            .file_path
            .cmp(&second.file_path)
            .then(first.line_number.cmp(&second.line_number))
            .then(first.qualified_name.cmp(&second.qualified_name))
    });
    unreachable_entities
}

/// Добавляет в граф ребра по ссылкам на имена.
///
/// :param file_analyses: Результаты анализа файлов.
/// :param dependency_graph: Мутируемая ссылка на граф зависимостей.
/// :param scope_nodes: Отображение имен областей видимости на узлы графа.
/// :param name_index: Отображение простых имен на узлы сущностей.
fn add_reference_edges(
    file_analyses: &[FileAnalysis],
    dependency_graph: &mut DiGraph<&str, ()>,
    scope_nodes: &HashMap<&str, NodeIndex>,
    name_index: &HashMap<&str, Vec<NodeIndex>>,
) {
    for file_analysis in file_analyses {
        for scoped_reference in &file_analysis.scoped_references {
            let Some(&source_node) =
                scope_nodes.get(scoped_reference.scope_qualified_name.as_str())
            else {
                continue;
            };
            let Some(target_nodes) = name_index.get(scoped_reference.referenced_name.as_str())
            else {
                continue;
            };
            for &target_node in target_nodes {
                if source_node != target_node {
                    dependency_graph.add_edge(source_node, target_node, ());
                }
            }
        }
    }
}

/// Добавляет ребра от сущностей к содержащим их областям видимости.
///
/// Живой метод делает живым содержащий его класс, поскольку определение
/// класса исполняется при импорте модуля.
///
/// :param entities_by_node: Отображение узлов графа на сущности.
/// :param dependency_graph: Мутируемая ссылка на граф зависимостей.
/// :param scope_nodes: Отображение имен областей видимости на узлы графа.
fn add_containment_edges(
    entities_by_node: &HashMap<NodeIndex, &CodeEntity>,
    dependency_graph: &mut DiGraph<&str, ()>,
    scope_nodes: &HashMap<&str, NodeIndex>,
) {
    for (&entity_node, code_entity) in entities_by_node {
        if let Some(&container_node) = scope_nodes.get(code_entity.containing_scope.as_str()) {
            if container_node != entity_node {
                dependency_graph.add_edge(entity_node, container_node, ());
            }
        }
    }
}

/// Собирает общий пул динамических строковых ссылок проекта.
///
/// Точечные строки вида `myapp.views.my_view` разрешаются до простого
/// имени функции. Имена из конфигурации пользователя дополняют пул.
///
/// :param file_analyses: Результаты анализа файлов.
/// :param configuration: Конфигурация анализатора.
/// :return: Множество имен из динамических ссылок.
fn build_dynamic_reference_pool<'analysis>(
    file_analyses: &'analysis [FileAnalysis],
    configuration: &'analysis AnalyzerConfiguration,
) -> HashSet<&'analysis str> {
    let mut dynamic_reference_pool = HashSet::new();
    let configured_names = configuration.extra_dynamic_names.iter().map(String::as_str);
    let extracted_names = file_analyses
        .iter()
        .flat_map(|file_analysis| file_analysis.dynamic_references.iter().map(String::as_str));
    for dynamic_name in configured_names.chain(extracted_names) {
        dynamic_reference_pool.insert(dynamic_name);
        dynamic_reference_pool.insert(heuristics::last_dotted_segment(dynamic_name));
    }
    dynamic_reference_pool
}

/// Вычисляет множество достижимых узлов графа.
///
/// Корнями обхода выступают модули, явные точки входа и сущности,
/// имена которых найдены в пуле динамических ссылок.
///
/// :param file_analyses: Результаты анализа файлов.
/// :param dependency_graph: Граф зависимостей.
/// :param scope_nodes: Отображение имен областей видимости на узлы графа.
/// :param entities_by_node: Отображение узлов графа на сущности.
/// :param dynamic_reference_pool: Пул динамических строковых ссылок.
/// :return: Множество достижимых узлов.
fn compute_reachable_nodes(
    file_analyses: &[FileAnalysis],
    dependency_graph: &DiGraph<&str, ()>,
    scope_nodes: &HashMap<&str, NodeIndex>,
    entities_by_node: &HashMap<NodeIndex, &CodeEntity>,
    dynamic_reference_pool: &HashSet<&str>,
) -> HashSet<NodeIndex> {
    let mut pending_nodes: Vec<NodeIndex> = Vec::new();
    for file_analysis in file_analyses {
        if let Some(&module_node) = scope_nodes.get(file_analysis.module_path.as_str()) {
            pending_nodes.push(module_node);
        }
    }
    for (&entity_node, code_entity) in entities_by_node {
        let is_dynamic_reference_target =
            dynamic_reference_pool.contains(code_entity.simple_name.as_str());
        if code_entity.is_entry_point || is_dynamic_reference_target {
            pending_nodes.push(entity_node);
        }
    }

    let mut reachable_nodes = HashSet::new();
    while let Some(current_node) = pending_nodes.pop() {
        if !reachable_nodes.insert(current_node) {
            continue;
        }
        for neighbor_node in dependency_graph.neighbors(current_node) {
            if !reachable_nodes.contains(&neighbor_node) {
                pending_nodes.push(neighbor_node);
            }
        }
    }
    reachable_nodes
}
