# Changelog — Projeto ARGUS

Todas as mudanças notáveis do Projeto ARGUS são documentadas neste arquivo.

## [1.0.0] — 2026-11-26

### Adicionado
- **Workspace Cargo modular** com 6 crates (`argus-core`, `argus-devpilot`, `argus-netscope`, `argus-infrastructure`, `argus-tui`, `argus-cli`).
- **ARGUS Core:** domínio compartilhado, portas hexagonais, configuração TOML multiplataforma, classificação de erros e cálculos puros com testes baseados em propriedades (`proptest`).
- **DevPilot:** scanner com suporte a `.gitignore`, detecção de linguagens e frameworks, parser de dependências (Cargo, npm, pip, pyproject, go.mod), detecção de TODO/FIXME, arquivos grandes, dependências circulares, Git Insights (`git2`), pontuação de saúde, geração de docs Markdown e `ProjectAnalyzer` extensível.
- **NetScope:** listagem de interfaces, taxas RX/TX, conexões TCP/UDP com processos, testes de latência (min/média/máx e perda), verificação de portas e descoberta de hosts em CIDRs autorizados e alertas configuráveis.
- **Persistência & Exportação:** banco SQLite local (`analysis_runs`, `network_metrics`, `alerts`, `settings`) com retenção configurável e exportadores JSON/Markdown.
- **Interface TUI:** dashboard interativo Ratatui + Crossterm com 5 telas, sparkline de tráfego e modal de ajuda.
- **Qualidade & CI:** testes unitários, de propriedades e de integração, benchmark Criterion e pipeline GitHub Actions para Linux e Windows.
