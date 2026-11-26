# Contribuindo com o Projeto ARGUS

Obrigado pelo interesse em contribuir com o **Projeto ARGUS**!

## Diretrizes Arquiteturais

1. **Regra da Dependência (Hexagonal):** `argus-core` nunca deve depender de bibliotecas de interface (`ratatui`), banco de dados (`rusqlite`), `git2` ou APIs específicas de sistema operacional.
2. **Modularidade:** funcionalidades de engenharia de software pertencem a `argus-devpilot`; observabilidade de redes pertence a `argus-netscope`; integrações externas pertencem a `argus-infrastructure`.
3. **Segurança:** nunca execute comandos externos arbitrários, não leia arquivos `.env` nem armazene segredos em logs.

## Fluxo de Trabalho

1. Crie uma branch descritiva (`feat/...`, `fix/...`, `docs/...`).
2. Execute a suíte de validação antes de abrir PR:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
3. Utilize commits semânticos (Conventional Commits).
