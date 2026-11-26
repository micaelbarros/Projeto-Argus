//! DP-012: Markdown project documentation / README generator.

use argus_core::application::services::format_bytes;
use argus_core::domain::project::{DirectoryNode, ProjectAnalysis};
use std::fmt::Write;

/// Generates a structured Markdown overview / README draft from a `ProjectAnalysis`.
pub fn generate_project_markdown(analysis: &ProjectAnalysis) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# {}\n", analysis.project.name);
    let _ = writeln!(
        out,
        "> Documentação gerada automaticamente pelo **Projeto ARGUS — DevPilot** em {}.\n",
        analysis.analyzed_at.format("%Y-%m-%d %H:%M UTC")
    );

    let _ = writeln!(out, "## Visão Geral\n");
    let _ = writeln!(out, "| Métrica | Valor |");
    let _ = writeln!(out, "|---|---|");
    let _ = writeln!(out, "| Arquivos analisados | {} |", analysis.project.total_files);
    let _ = writeln!(out, "| Diretórios | {} |", analysis.project.total_dirs);
    let _ = writeln!(out, "| Total de linhas | {} |", analysis.project.total_lines);
    let _ = writeln!(
        out,
        "| Tamanho total | {} |",
        format_bytes(analysis.project.total_bytes)
    );
    let _ = writeln!(
        out,
        "| Saúde do projeto | **{}/100 ({})** |\n",
        analysis.health.score, analysis.health.grade
    );

    if !analysis.languages.is_empty() {
        let _ = writeln!(out, "## Linguagens Detectadas\n");
        let _ = writeln!(out, "| Linguagem | Arquivos | Linhas | Participação |");
        let _ = writeln!(out, "|---|---:|---:|---:|");
        for lang in &analysis.languages {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {:.2}% |",
                lang.language, lang.files, lang.lines, lang.percentage
            );
        }
        let _ = writeln!(out);
    }

    if !analysis.frameworks.is_empty() {
        let _ = writeln!(out, "## Frameworks e Ferramentas\n");
        for fw in &analysis.frameworks {
            let _ = writeln!(out, "- **{}** ({}) — `{}`", fw.name, fw.category, fw.evidence);
        }
        let _ = writeln!(out);
    }

    if !analysis.dependencies.is_empty() {
        let _ = writeln!(
            out,
            "## Dependências Declaradas ({})\n",
            analysis.dependencies.len()
        );
        let _ = writeln!(out, "| Pacote | Versão | Ecossistema | Tipo |");
        let _ = writeln!(out, "|---|---|---|---|");
        for dep in analysis.dependencies.iter().take(40) {
            let kind = if dep.is_dev { "dev" } else { "runtime" };
            let _ = writeln!(
                out,
                "| `{}` | `{}` | {} | {} |",
                dep.name, dep.version, dep.ecosystem, kind
            );
        }
        if analysis.dependencies.len() > 40 {
            let _ = writeln!(
                out,
                "\n*... e mais {} dependência(s).*",
                analysis.dependencies.len() - 40
            );
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "## Estrutura de Diretórios\n");
    let _ = writeln!(out, "```text");
    render_tree(&analysis.directory_tree, "", true, &mut out, 0, 3);
    let _ = writeln!(out, "```\n");

    if let Some(git) = &analysis.git {
        let _ = writeln!(out, "## Informações Git\n");
        let _ = writeln!(out, "- **Branch atual:** `{}`", git.branch);
        let _ = writeln!(
            out,
            "- **Estado:** {}",
            if git.is_dirty { "Alterações pendentes" } else { "Limpo" }
        );
        let _ = writeln!(out, "- **Commits analisados:** {}\n", git.total_commits);
        if !git.recent_commits.is_empty() {
            let _ = writeln!(out, "### Commits recentes\n");
            for commit in git.recent_commits.iter().take(10) {
                let _ = writeln!(
                    out,
                    "- `{}` {} — *{}*",
                    commit.short_hash, commit.summary, commit.author
                );
            }
            let _ = writeln!(out);
        }
    }

    let _ = writeln!(out, "## Diagnóstico de Saúde\n");
    for check in &analysis.health.passed_checks {
        let _ = writeln!(out, "- [x] {check}");
    }
    for issue in &analysis.health.issues {
        let _ = writeln!(
            out,
            "- [ ] **[{}]** {} — {}",
            issue.code, issue.title, issue.recommendation
        );
    }

    out
}

fn render_tree(
    node: &DirectoryNode,
    prefix: &str,
    is_last: bool,
    out: &mut String,
    depth: usize,
    max_depth: usize,
) {
    if depth == 0 {
        let _ = writeln!(out, "{}/", node.name);
    } else {
        let branch = if is_last { "└── " } else { "├── " };
        let suffix = if node.is_dir { "/" } else { "" };
        let _ = writeln!(out, "{prefix}{branch}{}{suffix}", node.name);
    }

    if depth >= max_depth {
        return;
    }

    let next_prefix = if depth == 0 {
        String::new()
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    };

    let count = node.children.len();
    for (idx, child) in node.children.iter().enumerate() {
        render_tree(
            child,
            &next_prefix,
            idx + 1 == count,
            out,
            depth + 1,
            max_depth,
        );
    }
}
