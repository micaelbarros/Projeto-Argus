//! DP-008: Oversized file detector.

use argus_core::domain::project::{FileEntry, LargeFileFinding};

/// Returns all files whose size exceeds `threshold_kb * 1024` bytes, sorted largest-first.
pub fn detect_large_files(files: &[FileEntry], threshold_kb: u64) -> Vec<LargeFileFinding> {
    let threshold_bytes = threshold_kb.saturating_mul(1024);
    let mut findings: Vec<LargeFileFinding> = files
        .iter()
        .filter(|f| f.size_bytes > threshold_bytes)
        .map(|f| LargeFileFinding {
            path: f.relative_path.clone(),
            size_bytes: f.size_bytes,
            threshold_bytes,
        })
        .collect();

    findings.sort_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then_with(|| a.path.cmp(&b.path))
    });
    findings
}
