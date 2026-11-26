# ADR-0001: Arquitetura Hexagonal e Monólito Modular no Projeto ARGUS

- **Status:** Aceito
- **Data:** 2026-11-26

## Contexto

O Projeto ARGUS combina dois domínios distintos (análise de projetos de software e observabilidade de redes) em uma aplicação local CLI + TUI distribuída como executável único.

## Decisão

Adotar **Arquitetura Hexagonal (Ports and Adapters)** organizada como um **Monólito Modular** em Cargo Workspace:

- `argus-core`: núcleo puro com entidades, portas (traits), configuração e erros.
- `argus-devpilot` e `argus-netscope`: módulos de domínio e aplicação independentes entre si.
- `argus-infrastructure`: adaptadores concretos de SO, Git, rede, SQLite e exportação.
- `argus-cli` e `argus-tui`: camadas de apresentação desacopladas.

## Consequências

- **Positivas:** alta testabilidade (mocks/in-memory adapters), compilação paralela por crate, zero acoplamento de regras de negócio a Ratatui/SQLite/Git2 e distribuição simples em binário único.
- **Trade-offs:** exige disciplina na definição de traits de fronteira entre `argus-core` e `argus-infrastructure`.
