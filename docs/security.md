# Segurança e Privacidade — Projeto ARGUS

## Princípios

1. **Local-first:** nenhuma telemetria ou envio automático de dados para servidores externos.
2. **Menor privilégio:** o binário executa sem `root`/Administrador sempre que possível, informando claramente qualquer limitação do sistema operacional.
3. **Zero execução de shell arbitrário:** todas as operações usam APIs nativas em Rust (`ignore`, `git2`, `sysinfo`, `netstat2`, `std::net`, `rusqlite`).

## Controles do DevPilot

- **Escopo estrito:** o caminho raiz é canonicalizado e qualquer tentativa de path traversal (`..`) ou escape via symlink é bloqueada.
- **Proteção de segredos:** arquivos `.env`, `.env.*` (exceto `.env.example`/`.env.sample`/`.env.template`), `.pem`, `.key`, `id_rsa`, `id_ed25519` e `credentials.json` nunca têm conteúdo lido ou exibido.
- **Limites de leitura:** `max_file_size_mb` evita consumo excessivo de memória.
- **Escrita segura:** arquivos existentes nunca são sobrescritos sem a flag explícita `--force`.
- **Zero execução de código do repositório:** manifestos são apenas analisados estaticamente.

## Controles do NetScope

- **Política de autorização explícita (`AuthorizationPolicy`):** `argus netscope ports` e `argus netscope discover` validam o alvo contra `netscope.authorized_cidrs` no `config.toml` (padrão seguro: apenas `127.0.0.1/32`).
- **Limites de escopo e taxa:** descoberta limitada no máximo a blocos `/24`, verificação limitada a 64 portas por chamada e pausa configurável (`probe_rate_limit_ms`) entre sondagens.
- **Sem captura de payload:** nenhum conteúdo de pacote ou credencial de tráfego é capturado ou armazenado.
