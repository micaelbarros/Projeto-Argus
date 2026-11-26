# Projeto ARGUS

> Aplicação **local-first CLI + TUI** em **Rust** que reúne inteligência de engenharia de software (**DevPilot**) e observabilidade de redes (**NetScope**) em um **Monólito Modular com Arquitetura Hexagonal**.

---

## Visão Geral

O **Projeto ARGUS** centraliza em um único executável local, rápido e multiplataforma (Windows e Linux):

- **DevPilot (Engenharia de Software):** detecção de linguagens e frameworks, árvore de diretórios com filtros `.gitignore`, leitura de manifestos de dependências (`Cargo.toml`, `package.json`, `requirements.txt`, `pyproject.toml`, `go.mod`), detecção de `TODO/FIXME/HACK`, arquivos excessivamente grandes, detecção de dependências circulares, inspeção Git via `libgit2`, relatório de saúde (0–100), geração de documentação Markdown e sistema extensível de analisadores (`ProjectAnalyzer`).
- **NetScope (Observabilidade de Redes):** listagem e estado de interfaces, monitoramento contínuo de tráfego com cálculo de taxas RX/TX (KB/s e Mbps), conexões ativas TCP/UDP com associação de processos quando permitido, teste de latência com mínimo/média/máximo e perda de pacotes, verificação de portas e descoberta de hosts restritas a CIDRs autorizados, e alertas configuráveis.
- **ARGUS Core & Persistência:** configuração TOML multiplataforma (`config.toml`), histórico local opcional em SQLite (`argus.db`), exportação de relatórios em **JSON** e **Markdown**, cancelamento cooperativo (`Ctrl+C`) e dashboard interativo **TUI** com Ratatui + Crossterm.

---

## Arquitetura

```text
                    USER
                     │
             ┌───────┴───────┐
             ▼               ▼
         argus-cli       argus-tui
             └───────┬───────┘
                     ▼
          APPLICATION & MODULES
             ┌───────┴───────┐
             ▼               ▼
      argus-devpilot  argus-netscope
             └───────┬───────┘
                     ▼
        argus-core (Domain + Ports)
                     ▲
                     │ implements
                     │
           argus-infrastructure
        ┌───────┬────┴────┬────────┐
        ▼       ▼         ▼        ▼
       FS      Git2    Network   SQLite / Exporters
```

As dependências apontam sempre para o núcleo (`argus-core`), que não conhece Ratatui, SQLite, `git2` ou APIs específicas do sistema operacional. Consulte [docs/architecture.md](docs/architecture.md) e [docs/adr/0001-hexagonal-modular-monolith.md](docs/adr/0001-hexagonal-modular-monolith.md).

---

## Estrutura do Workspace

| Crate | Responsabilidade |
|---|---|
| `crates/argus-cli` | Binário `argus`, interpretação de comandos (`clap`), logs e injeção de dependências |
| `crates/argus-core` | Entidades de domínio, portas hexagonais, configuração TOML, erros e cálculos puros |
| `crates/argus-devpilot` | Detectores, analisadores extensíveis, saúde do projeto e geração de docs Markdown |
| `crates/argus-netscope` | Monitoramento de interfaces, taxas de transferência, conexões, latência e políticas de autorização |
| `crates/argus-infrastructure` | Adaptadores concretos: Filesystem seguro, `git2`, rede nativa, SQLite e exportadores |
| `crates/argus-tui` | Interface interativa Ratatui + Crossterm com 5 telas e atalhos rápidos |

---

## Instalação e Execução

### Pré-requisitos
- Rust estável (`>= 1.80`) com `cargo`, `rustfmt` e `clippy`.

### Compilar e executar localmente

```bash
cargo build --release -p argus-cli --bin argus
```

Ou executar diretamente via Cargo:

```bash
cargo run -p argus-cli --bin argus -- --version
cargo run -p argus-cli --bin argus -- init
cargo run -p argus-cli --bin argus -- doctor
cargo run -p argus-cli --bin argus -- tui .
```

---

## Comandos Principais

```bash
# Informações e diagnóstico
argus --version
argus init
argus doctor

# Dashboard interativo no terminal (TUI)
argus tui .

# DevPilot — Engenharia de software
argus devpilot analyze .
argus devpilot health .
argus devpilot git .
argus devpilot docs . --output DOCS.md

# NetScope — Observabilidade de redes
argus netscope interfaces
argus netscope monitor --samples 5 --interval-ms 1000
argus netscope ping 1.1.1.1 --count 4
argus netscope connections --protocol tcp
argus netscope ports 127.0.0.1 --ports 80,443,8080
argus netscope discover 127.0.0.1/32
argus netscope alerts

# Histórico e relatórios unificados
argus history --limit 20
argus report --path . --format json
argus report --path . --format markdown --output relatorio.md
```

Opções globais disponíveis em todos os comandos: `--json`, `--verbose`, `--quiet`, `--config <FILE>`, `--output <FILE>`, `--force`, `--help`.

Consulte a referência completa em [docs/commands.md](docs/commands.md).

---

## Atalhos da TUI

| Tecla | Ação |
|---|---|
| `1` | Dashboard Geral |
| `2` | DevPilot (Saúde, Dependências, TODOs, Git) |
| `3` | NetScope (Interfaces/Taxas, Conexões TCP/UDP) |
| `4` | Histórico SQLite |
| `C` | Configurações ativas |
| `Tab` / `←` `→` | Alternar sub-abas |
| `↑` `↓` / `j` `k` | Rolar listas |
| `R` | Atualizar dados |
| `?` | Ajuda |
| `Q` / `Esc` | Sair |

---

## Qualidade, Testes e Benchmarks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo bench --bench scanner_benchmark
```

---

## Segurança e Privacidade

- **Local-first:** nenhuma telemetria ou conexão externa automática durante análises de projetos.
- **Menor privilégio:** funcionalidades operam sem privilégios administrativos, informando limitações quando aplicável.
- **Proteção contra segredos e traversal:** arquivos `.env` e chaves privadas nunca são lidos ou exibidos; links simbólicos fora do diretório raiz não são seguidos; nenhum script do repositório analisado é executado.
- **Sondagens autorizadas:** `ports` e `discover` aceitam apenas alvos explicitamente declarados em `netscope.authorized_cidrs` no `config.toml`, com rate limiting.

Mais detalhes em [docs/security.md](docs/security.md).

---

## Licença

Distribuído sob a licença MIT. Veja [LICENSE](LICENSE).
