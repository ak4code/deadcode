//! Интеграционные тесты загрузки конфигурации анализатора.

use std::fs;
use std::path::{Path, PathBuf};

use dc::{load_configuration, run_analysis, AnalyzerConfiguration, DcError};

/// Создает уникальную временную директорию для теста.
fn create_temp_directory(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("dc-test-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("создание временной директории");
    directory
}

#[test]
fn configuration_is_loaded_from_pyproject_section() {
    let project_directory = create_temp_directory("pyproject");
    fs::write(
        project_directory.join("pyproject.toml"),
        "[tool.dc]\nexclude_directories = [\"vendored\"]\n",
    )
    .expect("запись pyproject.toml");

    let configuration =
        load_configuration(None, &project_directory).expect("корректная конфигурация");
    assert_eq!(
        configuration.exclude_directories,
        vec!["vendored".to_string()]
    );

    let _ = fs::remove_dir_all(&project_directory);
}

#[test]
fn standalone_file_takes_priority_over_pyproject() {
    let project_directory = create_temp_directory("priority");
    fs::write(
        project_directory.join("pyproject.toml"),
        "[tool.dc]\nexclude_directories = [\"from_pyproject\"]\n",
    )
    .expect("запись pyproject.toml");
    fs::write(
        project_directory.join(".dc.toml"),
        "exclude_directories = [\"from_standalone\"]\n",
    )
    .expect("запись .dc.toml");

    let configuration =
        load_configuration(None, &project_directory).expect("корректная конфигурация");
    assert_eq!(
        configuration.exclude_directories,
        vec!["from_standalone".to_string()]
    );

    let _ = fs::remove_dir_all(&project_directory);
}

#[test]
fn invalid_configuration_is_reported_as_error() {
    let project_directory = create_temp_directory("invalid");
    fs::write(
        project_directory.join(".dc.toml"),
        "exclude_directories = 42\n",
    )
    .expect("запись .dc.toml");

    let load_result = load_configuration(None, &project_directory);
    assert!(matches!(
        load_result,
        Err(DcError::ConfigurationInvalid { .. })
    ));

    let _ = fs::remove_dir_all(&project_directory);
}

#[test]
fn explicitly_requested_missing_configuration_is_an_error() {
    let load_result = load_configuration(Some(Path::new("/nonexistent/.dc.toml")), Path::new("."));
    assert!(matches!(
        load_result,
        Err(DcError::ConfigurationNotFound { .. })
    ));
}

#[test]
fn configured_decorators_extend_entry_points() {
    let project_directory = create_temp_directory("extra-decorators");
    fs::write(
        project_directory.join("consumers.py"),
        "@broker.subscribe\ndef consume_events():\n    return None\n",
    )
    .expect("запись consumers.py");

    let default_configuration = AnalyzerConfiguration::default();
    let default_report = run_analysis(&project_directory, &default_configuration);
    assert!(default_report
        .findings
        .iter()
        .any(|finding| finding.qualified_name == "consumers.consume_events"));

    let mut extended_configuration = AnalyzerConfiguration::default();
    extended_configuration
        .extra_entry_point_decorators
        .push("broker.subscribe".to_string());
    let extended_report = run_analysis(&project_directory, &extended_configuration);
    assert!(!extended_report
        .findings
        .iter()
        .any(|finding| finding.qualified_name == "consumers.consume_events"));

    let _ = fs::remove_dir_all(&project_directory);
}
