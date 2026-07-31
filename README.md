# Bizzarrebot - Rust Edition 🦀

Este é o Bizzarrebot, um bot de Discord de altíssimo desempenho totalmente reescrito em Rust usando [Serenity](https://github.com/serenity-rs/serenity) e [SQLx](https://github.com/launchbadge/sqlx). Ele foi projetado para ser leve, incrivelmente rápido e eficiente com uso de memória e CPU, lidando com milhares de eventos por segundo sem suar a camisa.

## 🚀 Funcionalidades Principais

*   **Rastreamento Avançado de Voz (`/tempo`)**: Conta o tempo de cada usuário em canais de voz com distinção entre tempo válido e tempo mutado. Gera cards dinâmicos incríveis (via renderização de imagens em tempo real) mostrando as estatísticas do usuário na semana atual e na passada, com rankings de XP.
*   **Sistema de Tickets Dinâmico (`/config_suporte`)**: Criação de painéis de suporte interativos com múltiplos botões e select menus. Toda a configuração é feita pelo próprio Discord de forma intuitiva, sem precisar mexer em arquivos de configuração locais. Suporta categorização automática de cargos e canais.
*   **Fechamento de Metas Automático**: Sistema de CRON job embutido no Rust para fechamento das metas de voz aos domingos às 23:59.
*   **Integração Nativa PostgreSQL (Supabase)**: Migrado do SQLite para o PostgreSQL via Supabase para escalar perfeitamente em nuvem e ter backups seguros em tempo real.
*   **Arquitetura Baseada em RAM Cache (`DashMap`)**: Todas as sessões de voz e verificações de tempo real rodam exclusivamente na RAM com concorrência `Arc<DashMap>` e só escrevem no banco quando estritamente necessário (quando o usuário sai da sala). Zero gargalos no banco.

## 🛠️ Tecnologias

*   [Rust](https://www.rust-lang.org/)
*   [Serenity](https://github.com/serenity-rs/serenity) (Discord API wrapper)
*   [SQLx](https://github.com/launchbadge/sqlx) (Async PostgreSQL driver)
*   [Tokio](https://tokio.rs/) (Async runtime)
*   [ImageProc / Image](https://github.com/image-rs/image) (Renderização de cards de perfil)
*   [Supabase](https://supabase.com/) (Database hospedado)

## ⚙️ Como Rodar Localmente

### 1. Pré-requisitos
*   Instalar a linguagem [Rust](https://rustup.rs/) (cargo e rustc).
*   Ter um banco de dados PostgreSQL (como o Supabase).
*   Pegar o Token do seu Bot no [Discord Developer Portal](https://discord.com/developers/applications).

### 2. Configurar o `.env`
Crie um arquivo `.env` na raiz do projeto (mesmo nível do `Cargo.toml`) e adicione as chaves:

```env
DISCORD_TOKEN=seu_token_do_discord_aqui
DATABASE_URL=postgres://usuario:senha@aws-0-sa-east-1.pooler.supabase.com:6543/postgres
GUILD_ID=o_id_do_seu_servidor
```

### 3. Compilar e Rodar

Para ambiente de desenvolvimento:
```bash
cargo run
```

Para produção (compilação máxima otimizada, binário super rápido):
```bash
cargo run --release
```

O bot vai aplicar automaticamente as migrations ou construir as tabelas no Supabase caso não existam, e começar a escutar os eventos!

## 📈 Perfomance vs Node.js

Esta reescrita em Rust resolve problemas crônicos que a versão anterior em Node.js apresentava:
*   Redução drástica no uso de RAM (sem o Garbage Collector do V8).
*   Geração de imagens (cards) absurdamente mais rápida devido a compilação nativa (a lib `image` do Rust em `release` é instantânea se comparada ao Node Canvas).
*   Multithreading real no motor `tokio` em comparação ao Single Thread de Event Loop do Node.js.
