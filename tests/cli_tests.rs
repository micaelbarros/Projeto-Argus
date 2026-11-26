//! End-to-end CLI integration tests for the `argus` binary.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn prints_version_and_help() {
    Command::cargo_bin("argus")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("argus 1.0.0"));

    Command::cargo_bin("argus")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Projeto ARGUS"));
}

#[test]
fn init_doctor_analyze_and_report_flow() {
    let dir = tempdir().unwrap();
    let cfg_path = dir.path().join("argus-test.toml");
    let db_path = dir.path().join("argus-test.db");
    let proj_dir = dir.path().join("sample-proj");
    fs::create_dir_all(proj_dir.join("src")).unwrap();
    fs::write(proj_dir.join("README.md"), "# Demo\n").unwrap();
    fs::write(proj_dir.join("src/main.rs"), "fn main() {}\n").unwrap();

    // 1. argus init --config <cfg>
    Command::cargo_bin("argus")
        .unwrap()
        .args(["--config", cfg_path.to_str().unwrap(), "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Projeto ARGUS"));
    assert!(cfg_path.exists());

    // Point DB into tempdir so tests leave zero state on user machine
    let mut toml_text = fs::read_to_string(&cfg_path).unwrap();
    let escaped_db = db_path.to_string_lossy().replace('\\', "/");
    toml_text.push_str(&format!("\ndatabase_path = \"{escaped_db}\"\n"));
    fs::write(&cfg_path, toml_text).unwrap();

    // 2. argus doctor --json
    let doctor_out = Command::cargo_bin("argus")
        .unwrap()
        .args(["--config", cfg_path.to_str().unwrap(), "--json", "doctor"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let doctor_json: serde_json::Value = serde_json::from_slice(&doctor_out).unwrap();
    assert_eq!(doctor_json["app"], "Projeto ARGUS");

    // 3. argus devpilot analyze <proj> --json
    let analyze_out = Command::cargo_bin("argus")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--json",
            "devpilot",
            "analyze",
            proj_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let analyze_json: serde_json::Value = serde_json::from_slice(&analyze_out).unwrap();
    assert_eq!(analyze_json["project"]["total_files"], 2);

    // 4. argus report --format markdown --output <file>
    let report_path = dir.path().join("report.md");
    Command::cargo_bin("argus")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--output",
            report_path.to_str().unwrap(),
            "report",
            "--path",
            proj_dir.to_str().unwrap(),
            "--format",
            "markdown",
        ])
        .assert()
        .success();
    let report_md = fs::read_to_string(&report_path).unwrap();
    assert!(report_md.contains("Relatório Técnico — Projeto ARGUS"));

    // 5. Refusing overwrite without --force
    Command::cargo_bin("argus")
        .unwrap()
        .args([
            "--config",
            cfg_path.to_str().unwrap(),
            "--output",
            report_path.to_str().unwrap(),
            "report",
            "--path",
            proj_dir.to_str().unwrap(),
            "--format",
            "markdown",
        ])
        .assert()
        .failure();
}
