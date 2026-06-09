use dc::{run_analysis, AnalyzerConfiguration};

/// Возвращает путь к демонстрационному Django проекту.
fn fixture_project_path() -> String {
    format!(
        "{}/tests/fixtures/demo_project",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn detects_dead_code_and_respects_django_heuristics() {
    let configuration = AnalyzerConfiguration::default();
    let report = run_analysis(&fixture_project_path(), &configuration);
    let dead_names: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.qualified_name.as_str())
        .collect();

    // Действительно мертвый код обнаруживается.
    assert!(dead_names.contains(&"shop.views.abandoned_view"), "{dead_names:?}");
    assert!(dead_names.contains(&"shop.models.Product.unused_helper"), "{dead_names:?}");
    assert!(dead_names.contains(&"shop.tasks.forgotten_helper"), "{dead_names:?}");
    assert!(dead_names.contains(&"shop.utils.resolve_callable"), "{dead_names:?}");
    assert!(dead_names.contains(&"shop.utils.LEGACY_FLAG"), "{dead_names:?}");

    // Представление, зарегистрированное в urls.py, живое.
    assert!(!dead_names.contains(&"shop.views.product_list"), "{dead_names:?}");
    // Метод модели из list_display живой.
    assert!(!dead_names.contains(&"shop.models.Product.display_price"), "{dead_names:?}");
    // Задача Celery и ее вспомогательная функция живые.
    assert!(!dead_names.contains(&"shop.tasks.refresh_catalog"), "{dead_names:?}");
    assert!(!dead_names.contains(&"shop.tasks._collect_catalog_rows"), "{dead_names:?}");
    // Функция, вызываемая через getattr, живая.
    assert!(!dead_names.contains(&"shop.utils.dynamic_target"), "{dead_names:?}");
    // Обработчик сигнала живой.
    assert!(!dead_names.contains(&"shop.signals.handle_product_saved"), "{dead_names:?}");
    // Management команда и ее метод handle живые.
    assert!(!dead_names
        .contains(&"shop.management.commands.sync_products.Command"), "{dead_names:?}");
    assert!(!dead_names
        .contains(&"shop.management.commands.sync_products.Command.handle"), "{dead_names:?}");
    // Класс админки, конфигурация приложения и модель живые.
    assert!(!dead_names.contains(&"shop.admin.ProductAdmin"), "{dead_names:?}");
    assert!(!dead_names.contains(&"shop.apps.ShopConfig"), "{dead_names:?}");
    assert!(!dead_names.contains(&"shop.models.Product"), "{dead_names:?}");
    // Переменная urlpatterns читается Django неявно.
    assert!(!dead_names.contains(&"shop.urls.urlpatterns"), "{dead_names:?}");

    // Директория migrations исключена из анализа.
    assert!(
        !dead_names.iter().any(|name| name.contains("migration")),
        "{dead_names:?}"
    );
}

#[test]
fn report_contains_analysis_statistics() {
    let configuration = AnalyzerConfiguration::default();
    let report = run_analysis(&fixture_project_path(), &configuration);
    assert!(report.analyzed_file_count >= 9);
    assert!(report.extracted_entity_count > report.findings.len());
}
