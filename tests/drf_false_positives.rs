//! Регрессионные тесты ложных срабатываний на проекте с Django REST Framework.
//!
//! Фикстура воспроизводит типичный DRF проект: сериализаторы с хуками
//! `validate` / `validate_*`, permission-классы, методы `perform_*`
//! во ViewSet и функции, используемые только внутри своего файла.

use std::path::{Path, PathBuf};

use dc::{run_analysis, AnalyzerConfiguration};

/// Возвращает путь к демонстрационному DRF проекту.
fn fixture_project_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/drf_project")
}

/// Возвращает полные имена находок мертвого кода для фикстуры.
fn collect_dead_names() -> Vec<String> {
    run_analysis(&fixture_project_path(), &AnalyzerConfiguration::default())
        .findings
        .into_iter()
        .map(|finding| finding.qualified_name)
        .collect()
}

#[test]
fn framework_hooks_are_not_reported_as_dead() {
    let dead_names = collect_dead_names();
    let contains = |name: &str| dead_names.iter().any(|dead| dead == name);

    // Методы permission-класса вызываются DRF по контракту.
    assert!(
        !contains("store.permissions.IsOrderOwner.has_permission"),
        "{dead_names:?}"
    );
    assert!(
        !contains("store.permissions.IsOrderOwner.has_object_permission"),
        "{dead_names:?}"
    );
    // Хуки сериализатора вызываются DRF по соглашению.
    assert!(
        !contains("store.serializers.BaseOrderSerializer.validate"),
        "{dead_names:?}"
    );
    assert!(
        !contains("store.serializers.OrderSerializer.validate_email"),
        "{dead_names:?}"
    );
    assert!(
        !contains("store.serializers.OrderSerializer.get_total_display"),
        "{dead_names:?}"
    );
    assert!(
        !contains("store.serializers.OrderSerializer.create"),
        "{dead_names:?}"
    );
    // Хук ViewSet вызывается DRF при создании объекта.
    assert!(
        !contains("store.views.OrderViewSet.perform_create"),
        "{dead_names:?}"
    );
}

#[test]
fn same_file_usage_keeps_functions_alive() {
    let dead_names = collect_dead_names();
    let contains = |name: &str| dead_names.iter().any(|dead| dead == name);

    // Функция используется в этом же файле хуком validate_email.
    assert!(
        !contains("store.serializers.normalize_email"),
        "{dead_names:?}"
    );
    // Функция используется в этом же файле функцией apply_discount.
    assert!(
        !contains("store.services.compute_discount"),
        "{dead_names:?}"
    );
}

#[test]
fn base_classes_and_their_subclasses_are_alive() {
    let dead_names = collect_dead_names();
    let contains = |name: &str| dead_names.iter().any(|dead| dead == name);

    // Базовый класс используется через наследование в этом же файле.
    assert!(!contains("store.models.BaseTimestamped"), "{dead_names:?}");
    assert!(
        !contains("store.serializers.BaseOrderSerializer"),
        "{dead_names:?}"
    );
    // Классы, зарегистрированные через router и атрибуты ViewSet, живые.
    assert!(!contains("store.views.OrderViewSet"), "{dead_names:?}");
    assert!(
        !contains("store.serializers.OrderSerializer"),
        "{dead_names:?}"
    );
    assert!(
        !contains("store.permissions.IsOrderOwner"),
        "{dead_names:?}"
    );
}

#[test]
fn pydantic_models_and_nested_config_are_alive() {
    let dead_names = collect_dead_names();
    let contains = |name: &str| dead_names.iter().any(|dead| dead == name);

    // Вложенный класс Config читается pydantic по соглашению.
    assert!(
        !contains("store.schemas.OrderSchema.Config"),
        "{dead_names:?}"
    );
    // Методы наследника BaseModel вызываются pydantic и кодом проекта.
    assert!(
        !contains("store.schemas.OrderSchema.summary_line"),
        "{dead_names:?}"
    );
    assert!(!contains("store.schemas.OrderSchema"), "{dead_names:?}");
}

#[test]
fn elasticsearch_documents_and_overrides_are_alive() {
    let dead_names = collect_dead_names();
    let contains = |name: &str| dead_names.iter().any(|dead| dead == name);

    // Класс зарегистрирован декоратором registry.register_document.
    assert!(!contains("store.documents.OrderIndex"), "{dead_names:?}");
    // Вложенные классы Index и Django читаются django-elasticsearch-dsl.
    assert!(
        !contains("store.documents.OrderIndex.Index"),
        "{dead_names:?}"
    );
    assert!(
        !contains("store.documents.OrderIndex.Django"),
        "{dead_names:?}"
    );
    // Методы prepare_* вызываются при индексации документов.
    assert!(
        !contains("store.documents.OrderIndex.prepare_email"),
        "{dead_names:?}"
    );
    // Метод базового класса под управлением фреймворка живой.
    assert!(
        !contains("store.documents.BaseSearchIndex.serialize_payload"),
        "{dead_names:?}"
    );
    // Переопределение в наследнике живое: признак управления фреймворком
    // распространяется по иерархии наследования транзитивно.
    assert!(
        !contains("store.documents.OrderIndex.serialize_payload"),
        "{dead_names:?}"
    );
}

#[test]
fn genuinely_dead_code_is_still_detected() {
    let dead_names = collect_dead_names();
    assert_eq!(
        dead_names,
        vec!["store.services.dead_service".to_string()],
        "ожидается ровно одна находка"
    );
}
