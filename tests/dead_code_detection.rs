//! Интеграционные тесты анализа демонстрационного Django проекта.

use std::path::{Path, PathBuf};

use dc::{render_report, run_analysis, AnalyzerConfiguration, ReportFormat};

/// Возвращает путь к демонстрационному Django проекту.
fn fixture_project_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_project")
}

/// Возвращает полные имена находок мертвого кода для фикстуры.
fn collect_dead_names(configuration: &AnalyzerConfiguration) -> Vec<String> {
    run_analysis(&fixture_project_path(), configuration)
        .findings
        .into_iter()
        .map(|finding| finding.qualified_name)
        .collect()
}

#[test]
fn detects_dead_code_and_respects_django_heuristics() {
    let dead_names = collect_dead_names(&AnalyzerConfiguration::default());
    let contains = |name: &str| dead_names.iter().any(|dead| dead == name);

    // Действительно мертвый код обнаруживается.
    assert!(contains("shop.views.abandoned_view"), "{dead_names:?}");
    assert!(
        contains("shop.models.Product.unused_helper"),
        "{dead_names:?}"
    );
    assert!(contains("shop.tasks.forgotten_helper"), "{dead_names:?}");
    assert!(contains("shop.utils.resolve_callable"), "{dead_names:?}");
    assert!(contains("shop.utils.LEGACY_FLAG"), "{dead_names:?}");

    // Представление, зарегистрированное в urls.py, живое.
    assert!(!contains("shop.views.product_list"), "{dead_names:?}");
    // Метод модели из list_display живой.
    assert!(
        !contains("shop.models.Product.display_price"),
        "{dead_names:?}"
    );
    // Задача Celery и ее вспомогательная функция живые.
    assert!(!contains("shop.tasks.refresh_catalog"), "{dead_names:?}");
    assert!(
        !contains("shop.tasks._collect_catalog_rows"),
        "{dead_names:?}"
    );
    // Функция, вызываемая через getattr, живая.
    assert!(!contains("shop.utils.dynamic_target"), "{dead_names:?}");
    // Обработчик сигнала живой.
    assert!(
        !contains("shop.signals.handle_product_saved"),
        "{dead_names:?}"
    );
    // Management команда и ее метод handle живые.
    assert!(
        !contains("shop.management.commands.sync_products.Command"),
        "{dead_names:?}"
    );
    assert!(
        !contains("shop.management.commands.sync_products.Command.handle"),
        "{dead_names:?}"
    );
    // Класс админки, конфигурация приложения и модель живые.
    assert!(!contains("shop.admin.ProductAdmin"), "{dead_names:?}");
    assert!(!contains("shop.apps.ShopConfig"), "{dead_names:?}");
    assert!(!contains("shop.models.Product"), "{dead_names:?}");
    // Переменная urlpatterns читается Django неявно.
    assert!(!contains("shop.urls.urlpatterns"), "{dead_names:?}");

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
    assert!(report.skipped_files.is_empty());
}

#[test]
fn findings_are_sorted_and_deterministic() {
    let configuration = AnalyzerConfiguration::default();
    let first_run = collect_dead_names(&configuration);
    let second_run = collect_dead_names(&configuration);
    assert_eq!(first_run, second_run);

    let report = run_analysis(&fixture_project_path(), &configuration);
    let locations: Vec<(PathBuf, usize)> = report
        .findings
        .iter()
        .map(|finding| (finding.file_path.clone(), finding.line_number))
        .collect();
    let mut sorted_locations = locations.clone();
    sorted_locations.sort();
    assert_eq!(locations, sorted_locations);
}

#[test]
fn empty_exclude_list_reveals_code_in_migrations() {
    let mut configuration = AnalyzerConfiguration::default();
    configuration.exclude_directories.clear();
    let dead_names = collect_dead_names(&configuration);
    assert!(
        dead_names
            .iter()
            .any(|name| name == "shop.migrations.0001_initial.totally_dead_in_migration"),
        "{dead_names:?}"
    );
}

#[test]
fn extra_dynamic_names_suppress_findings() {
    let mut configuration = AnalyzerConfiguration::default();
    configuration
        .extra_dynamic_names
        .push("abandoned_view".to_string());
    let dead_names = collect_dead_names(&configuration);
    assert!(
        !dead_names
            .iter()
            .any(|name| name == "shop.views.abandoned_view"),
        "{dead_names:?}"
    );
}

#[test]
fn json_report_is_machine_readable() {
    let configuration = AnalyzerConfiguration::default();
    let report = run_analysis(&fixture_project_path(), &configuration);
    let rendered = render_report(&report, ReportFormat::Json, false);

    let parsed: serde_json::Value = serde_json::from_str(&rendered).expect("корректный JSON");
    let findings = parsed["findings"].as_array().expect("массив находок");
    assert_eq!(findings.len(), report.findings.len());
    assert!(findings.iter().any(|finding| {
        finding["qualified_name"] == "shop.views.abandoned_view"
            && finding["kind"] == "function"
            && finding["line_number"].is_u64()
    }));
    assert_eq!(
        parsed["analyzed_file_count"].as_u64(),
        Some(report.analyzed_file_count as u64)
    );
}

#[test]
fn text_report_lists_findings_with_locations() {
    let configuration = AnalyzerConfiguration::default();
    let report = run_analysis(&fixture_project_path(), &configuration);
    let rendered = render_report(&report, ReportFormat::Text, true);
    assert!(rendered.contains("Проанализировано файлов:"));
    assert!(rendered.contains("shop.views.abandoned_view"));
    assert!(rendered.contains("не используется"));
}
