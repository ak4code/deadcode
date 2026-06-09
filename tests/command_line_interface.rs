//! Интеграционные тесты интерфейса командной строки.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Возвращает путь к демонстрационному Django проекту.
fn fixture_project_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_project")
}

/// Запускает собранный бинарный файл `dc` с аргументами.
fn run_dc(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dc"))
        .args(arguments)
        .output()
        .expect("запуск бинарного файла dc")
}

#[test]
fn findings_produce_exit_code_one_and_text_report() {
    let fixture = fixture_project_path();
    let output = run_dc(&["--target-path", fixture.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("abandoned_view"), "{stdout}");
    assert!(stdout.contains("не используется"), "{stdout}");
}

#[test]
fn json_format_produces_parseable_output() {
    let fixture = fixture_project_path();
    let output = run_dc(&[
        "--target-path",
        fixture.to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("корректный JSON");
    assert!(parsed["findings"].is_array());
}

#[test]
fn kind_filter_limits_report_to_requested_kinds() {
    let fixture = fixture_project_path();
    let output = run_dc(&[
        "--target-path",
        fixture.to_str().unwrap(),
        "--kind",
        "variable",
    ]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("LEGACY_FLAG"), "{stdout}");
    assert!(!stdout.contains("abandoned_view"), "{stdout}");
    assert!(!stdout.contains("unused_helper"), "{stdout}");
}

#[test]
fn kind_filter_accepts_comma_separated_values() {
    let fixture = fixture_project_path();
    let output = run_dc(&[
        "--target-path",
        fixture.to_str().unwrap(),
        "--kind",
        "function,method",
    ]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("abandoned_view"), "{stdout}");
    assert!(stdout.contains("unused_helper"), "{stdout}");
    assert!(!stdout.contains("LEGACY_FLAG"), "{stdout}");
}

#[test]
fn kind_filter_without_matches_produces_exit_code_zero() {
    let fixture = fixture_project_path();
    let output = run_dc(&[
        "--target-path",
        fixture.to_str().unwrap(),
        "--kind",
        "class",
    ]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Мертвый код не найден"), "{stdout}");
}

#[test]
fn missing_target_directory_produces_exit_code_two() {
    let output = run_dc(&["--target-path", "/nonexistent/project"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("целевая директория не существует"),
        "{stderr}"
    );
}

#[test]
fn missing_explicit_configuration_produces_exit_code_two() {
    let fixture = fixture_project_path();
    let output = run_dc(&[
        "--target-path",
        fixture.to_str().unwrap(),
        "--config-path",
        "/nonexistent/.dc.toml",
    ]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("файл конфигурации не найден"), "{stderr}");
}
