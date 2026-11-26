//! DP-013: Project health scoring engine.

use argus_core::domain::project::{
    CircularDependency, FileEntry, GitSummary, HealthIssue, HealthReport, HealthSeverity,
    LargeFileFinding, TodoMarker,
};

/// Evaluates repository hygiene, structure, docs, tests, CI, and complexity into a 0..=100 score.
pub fn compute_health_report(
    files: &[FileEntry],
    todos: &[TodoMarker],
    large_files: &[LargeFileFinding],
    circular_deps: &[CircularDependency],
    git: Option<&GitSummary>,
) -> HealthReport {
    let mut score: i32 = 100;
    let mut passed_checks = Vec::new();
    let mut issues = Vec::new();

    let has_file = |predicate: &dyn Fn(&str) -> bool| -> bool {
        files.iter().any(|f| {
            let rel = f.relative_path.to_string_lossy().replace('\\', "/");
            predicate(&rel)
        })
    };

    if has_file(&|p| {
        let lower = p.to_ascii_lowercase();
        lower == "readme.md" || lower == "readme.txt" || lower == "readme"
    }) {
        passed_checks.push("Documentação inicial (README) presente".to_string());
    } else {
        score -= 15;
        issues.push(HealthIssue {
            code: "DOC-001".to_string(),
            severity: HealthSeverity::Warning,
            title: "README ausente na raiz".to_string(),
            detail: "Nenhum arquivo README.md foi encontrado na raiz do projeto.".to_string(),
            recommendation: "Execute `argus devpilot docs .` para gerar um README inicial."
                .to_string(),
        });
    }

    if has_file(&|p| {
        let lower = p.to_ascii_lowercase();
        lower == "license" || lower == "license.md" || lower == "license.txt"
    }) {
        passed_checks.push("Arquivo de licença (LICENSE) presente".to_string());
    } else {
        score -= 5;
        issues.push(HealthIssue {
            code: "DOC-002".to_string(),
            severity: HealthSeverity::Info,
            title: "LICENSE ausente".to_string(),
            detail: "O projeto não declara licença explícita na raiz.".to_string(),
            recommendation: "Adicione um arquivo LICENSE (ex.: MIT ou Apache-2.0).".to_string(),
        });
    }

    if has_file(&|p| p == ".gitignore") {
        passed_checks.push("Arquivo .gitignore configurado".to_string());
    } else {
        score -= 8;
        issues.push(HealthIssue {
            code: "GIT-001".to_string(),
            severity: HealthSeverity::Warning,
            title: ".gitignore ausente".to_string(),
            detail: "Sem .gitignore, artefatos de build e segredos podem ser versionados."
                .to_string(),
            recommendation: "Crie um arquivo .gitignore adequado à stack do projeto.".to_string(),
        });
    }

    if has_file(&|p| {
        let lower = p.to_ascii_lowercase();
        lower.starts_with("tests/")
            || lower.contains("/tests/")
            || lower.contains("_test.")
            || lower.contains(".test.")
            || lower.contains(".spec.")
    }) {
        passed_checks.push("Suíte de testes automatizados detectada".to_string());
    } else {
        score -= 15;
        issues.push(HealthIssue {
            code: "QA-001".to_string(),
            severity: HealthSeverity::Warning,
            title: "Testes automatizados não encontrados".to_string(),
            detail: "Nenhum diretório `tests/` ou arquivo de teste padrão foi detectado."
                .to_string(),
            recommendation: "Adicione testes unitários e de integração para regras críticas."
                .to_string(),
        });
    }

    if has_file(&|p| p.starts_with(".github/workflows/")) {
        passed_checks.push("Pipeline de CI (.github/workflows) detectado".to_string());
    } else {
        score -= 7;
        issues.push(HealthIssue {
            code: "CI-001".to_string(),
            severity: HealthSeverity::Info,
            title: "Pipeline CI não detectado".to_string(),
            detail: "Nenhum workflow de integração contínua encontrado em `.github/workflows/`."
                .to_string(),
            recommendation: "Configure CI para rodar formatação, lint e testes automaticamente."
                .to_string(),
        });
    }

    if circular_deps.is_empty() {
        passed_checks.push("Nenhuma dependência circular interna detectada".to_string());
    } else {
        let penalty = (circular_deps.len() as i32 * 8).min(24);
        score -= penalty;
        issues.push(HealthIssue {
            code: "ARCH-001".to_string(),
            severity: HealthSeverity::Critical,
            title: format!(
                "{} dependência(s) circular(es) detectada(s)",
                circular_deps.len()
            ),
            detail: "Ciclos entre módulos aumentam acoplamento e dificultam manutenção."
                .to_string(),
            recommendation: "Extraia contratos/interfaces compartilhadas para quebrar os ciclos."
                .to_string(),
        });
    }

    if large_files.is_empty() {
        passed_checks.push("Nenhum arquivo excessivamente grande detectado".to_string());
    } else {
        let penalty = (large_files.len() as i32 * 4).min(12);
        score -= penalty;
        issues.push(HealthIssue {
            code: "SIZE-001".to_string(),
            severity: HealthSeverity::Warning,
            title: format!(
                "{} arquivo(s) acima do limite configurado",
                large_files.len()
            ),
            detail: "Arquivos muito grandes prejudicam revisão de código e tempo de clone."
                .to_string(),
            recommendation: "Modularize arquivos extensos ou mova binários para fora do Git."
                .to_string(),
        });
    }

    if todos.len() > 20 {
        score -= 8;
        issues.push(HealthIssue {
            code: "DEBT-001".to_string(),
            severity: HealthSeverity::Warning,
            title: format!("Alta densidade de marcadores TODO/FIXME ({})", todos.len()),
            detail: "Acúmulo elevado de dívida técnica sinalizada em comentários.".to_string(),
            recommendation: "Converta pendências críticas em issues rastreadas.".to_string(),
        });
    } else {
        passed_checks.push(format!(
            "Marcadores TODO/FIXME sob controle ({})",
            todos.len()
        ));
    }

    let sensitive_files = files.iter().filter(|f| f.is_sensitive).count();
    if sensitive_files > 0 {
        score -= 10;
        issues.push(HealthIssue {
            code: "SEC-001".to_string(),
            severity: HealthSeverity::Critical,
            title: format!("{sensitive_files} arquivo(s) sensível(is) presente(s) no workspace"),
            detail: "Arquivos como `.env` ou chaves privadas exigem atenção para não irem ao Git."
                .to_string(),
            recommendation: "Garanta que arquivos `.env` e chaves estejam listados no `.gitignore`."
                .to_string(),
        });
    }

    if let Some(g) = git {
        if g.is_dirty {
            score -= 3;
            issues.push(HealthIssue {
                code: "GIT-002".to_string(),
                severity: HealthSeverity::Info,
                title: "Working tree com alterações não commitadas".to_string(),
                detail: format!(
                    "{} arquivo(s) pendente(s) na branch '{}'.",
                    g.modified_files.len(),
                    g.branch
                ),
                recommendation: "Revise e faça commit das alterações antes de gerar releases."
                    .to_string(),
            });
        } else {
            passed_checks.push(format!("Repositório Git limpo na branch '{}'", g.branch));
        }
    }

    let final_score = score.clamp(0, 100) as u8;
    let grade = match final_score {
        90..=100 => "A",
        80..=89 => "B",
        70..=79 => "C",
        60..=69 => "D",
        _ => "F",
    }
    .to_string();

    HealthReport {
        score: final_score,
        grade,
        passed_checks,
        issues,
    }
}
