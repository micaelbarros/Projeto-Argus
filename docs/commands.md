# Referência de Comandos — Projeto ARGUS

## Opções Globais

| Flag | Descrição |
|---|---|
| `--json` | Formata a saída em JSON estruturado |
| `-v`, `--verbose` | Habilita logs detalhados (`debug`) em `stderr` |
| `-q`, `--quiet` | Reduz saída informativa |
| `--config <FILE>` | Usa um arquivo TOML específico |
| `-o`, `--output <FILE>` | Grava a saída em arquivo |
| `--force` | Confirma sobrescrita de arquivos existentes |
| `-V`, `--version` | Exibe versão do binário |
| `-h`, `--help` | Exibe ajuda contextual |

## Core

- `argus init`: gera `config.toml` padrão e inicializa o banco local `argus.db`.
- `argus doctor`: verifica configuração, SQLite, módulos ativos e interfaces de rede.
- `argus tui [PATH]`: inicia o dashboard interativo Ratatui.
- `argus history [--limit N]`: lista execuções do DevPilot, métricas NetScope e alertas.
- `argus report [--path PATH] [--format json|markdown] [--ping HOST]`: consolida um relatório completo.

## DevPilot

- `argus devpilot analyze [PATH]`: análise completa de linguagens, frameworks, dependências, TODOs, arquivos grandes, ciclos, Git e saúde.
- `argus devpilot health [PATH]`: exibe nota de saúde (0–100) e recomendações acionáveis.
- `argus devpilot git [PATH] [--commits N]`: detalha branch, working tree e commits recentes.
- `argus devpilot docs [PATH]`: gera documentação Markdown inicial do repositório.

## NetScope

- `argus netscope interfaces`: lista interfaces de rede, status, IPs e contadores RX/TX.
- `argus netscope monitor [--samples N] [--interval-ms N]`: calcula taxas RX/TX por intervalo (`--samples 0` executa até `Ctrl+C`).
- `argus netscope ping <TARGET> [--count N] [--timeout-ms N] [--port PORT]`: mede latência min/média/máx e perda de pacotes sem privilégios administrativos.
- `argus netscope connections [--protocol tcp|udp]`: exibe conexões TCP/UDP e processo associado quando permitido pelo SO.
- `argus netscope ports <HOST> --ports 80,443`: verifica portas TCP em hosts autorizados em `netscope.authorized_cidrs`.
- `argus netscope discover <CIDR>`: descobre hosts ativos em blocos CIDR autorizados (`/24` a `/32`).
- `argus netscope alerts [--all]`: consulta alertas gerados pelas regras de limite.
