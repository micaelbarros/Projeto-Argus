# Guia de Desenvolvimento — Projeto ARGUS

## Requisitos de Ambiente

- Rust estável (`rust-toolchain.toml` define `stable` com `rustfmt` e `clippy`).
- Em Windows: Visual Studio Build Tools (MSVC).
- Em Linux: `build-essential` / `gcc`.

## Comandos Úteis

```bash
# Formatação
cargo fmt --all

# Lint estrito
cargo clippy --workspace --all-targets -- -D warnings

# Todos os testes (unitários, proptest e integração)
cargo test --workspace

# Benchmark do scanner DevPilot
cargo bench --bench scanner_benchmark

# Execução da CLI em desenvolvimento
cargo run -p argus-cli --bin argus -- devpilot analyze .
```

## Adicionando um Novo Analisador no DevPilot

Implemente `argus_devpilot::ProjectAnalyzer` e registre-o via `DevPilotService::with_analyzer(...)`.
O analisador recebe `AnalyzerContext` somente leitura com arquivos, linguagens, frameworks e dependências já coletados.
