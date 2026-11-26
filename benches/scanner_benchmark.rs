//! Criterion benchmark for DevPilot workspace scanning and analysis performance.

use argus::core::config::ArgusConfig;
use argus::devpilot::domain::AnalyzeRequest;
use argus::devpilot::DevPilotService;
use argus::infrastructure::{Git2Adapter, LocalFileSystemAdapter};
use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;
use std::sync::Arc;
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;

fn bench_devpilot_scanner(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    for i in 0..80 {
        let sub = dir.path().join(format!("mod_{}", i % 8));
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            sub.join(format!("file_{i}.rs")),
            format!("// TODO: benchmark item {i}\npub fn f_{i}() -> usize {{ {i} }}\n"),
        )
        .unwrap();
    }
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"bench\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1.0\"\n",
    )
    .unwrap();

    let service = DevPilotService::new(
        Arc::new(LocalFileSystemAdapter::new()),
        Arc::new(Git2Adapter::new()),
        None,
    );
    let config = ArgusConfig::default();
    let req = AnalyzeRequest {
        path: dir.path().to_path_buf(),
        include_git: false,
        max_commits: 0,
    };
    let cancel = CancellationToken::new();

    c.bench_function("devpilot_analyze_80_files", |b| {
        b.iter(|| {
            let analysis = service.analyze(&req, &config, &cancel).unwrap();
            assert_eq!(analysis.project.total_files, 81);
        });
    });
}

criterion_group!(benches, bench_devpilot_scanner);
criterion_main!(benches);
