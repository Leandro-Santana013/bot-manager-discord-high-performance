use sqlx::PgPool;
use tracing::error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TicketOption {
    pub id: String,
    pub label: String,
    pub description: String,
    pub emoji: String,
    pub reply: String,
}

pub struct TicketDb;

impl TicketDb {
    pub async fn init(pool: &PgPool) {
        let q1 = "CREATE TABLE IF NOT EXISTS tickets_aceitos (
            id_usuario TEXT PRIMARY KEY,
            quantidade INTEGER DEFAULT 0
        )";
        let q2 = "CREATE TABLE IF NOT EXISTS ticket_config (
            chave TEXT PRIMARY KEY,
            valor TEXT
        )";

        if let Err(e) = sqlx::query(q1).execute(pool).await {
            error!("Erro ao criar tabela tickets_aceitos: {}", e);
        }
        if let Err(e) = sqlx::query(q2).execute(pool).await {
            error!("Erro ao criar tabela ticket_config: {}", e);
        }
    }

    pub async fn get_config(pool: &PgPool, key: &str, default_value: &str) -> String {
        let q = "SELECT valor FROM ticket_config WHERE chave = $1";
        match sqlx::query_as::<_, (String,)>(q)
            .bind(key)
            .fetch_optional(pool)
            .await
        {
            Ok(Some((valor,))) => valor,
            _ => default_value.to_string(),
        }
    }

    pub async fn set_config(pool: &PgPool, key: &str, value: &str) -> Result<(), sqlx::Error> {
        let q = "INSERT INTO ticket_config (chave, valor) VALUES ($1, $2) ON CONFLICT(chave) DO UPDATE SET valor = EXCLUDED.valor";
        sqlx::query(q)
            .bind(key)
            .bind(value)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn add_ticket(pool: &PgPool, user_id: &str) -> Result<(), sqlx::Error> {
        let q = "INSERT INTO tickets_aceitos (id_usuario, quantidade) VALUES ($1, 1) ON CONFLICT(id_usuario) DO UPDATE SET quantidade = tickets_aceitos.quantidade + 1";
        sqlx::query(q)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_top_tickets(pool: &PgPool, limit: i32) -> Result<Vec<(String, i32)>, sqlx::Error> {
        let q = "SELECT id_usuario, quantidade FROM tickets_aceitos ORDER BY quantidade DESC LIMIT $1";
        let rows = sqlx::query_as::<_, (String, i32)>(q)
            .bind(limit)
            .fetch_all(pool)
            .await?;
        Ok(rows)
    }

    pub async fn get_ticket_options(pool: &PgPool) -> Vec<TicketOption> {
        let raw = Self::get_config(pool, "ticket_options", "").await;
        if !raw.is_empty() {
            if let Ok(options) = serde_json::from_str::<Vec<TicketOption>>(&raw) {
                return options;
            }
        }

        vec![
            TicketOption {
                id: "denuncia".to_string(),
                label: "Quero fazer uma Denúncia".to_string(),
                description: "Denunciar quebra de regras, spam, ou condutas indevidas.".to_string(),
                emoji: "🚨".to_string(),
                reply: "🚨 **Para criar um atendimento de Denúncia, separe:**\n\n1. Provas do ocorrido (prints/vídeos)\n2. Nome e ID do meliante\n3. Descrição do ocorrido\n\n👇 Clique no botão abaixo quando estiver com tudo em mãos.".to_string(),
            },
            TicketOption {
                id: "duvida".to_string(),
                label: "Dúvidas Gerais".to_string(),
                description: "Falar com o suporte para dúvidas técnicas.".to_string(),
                emoji: "❓".to_string(),
                reply: "❓ **Para tirar uma dúvida com o Suporte:**\n\nPor favor, lembre-se de ler os canais de tutoriais antes de pedir ajuda.\n\n👇 Clique no botão abaixo para chamar a equipe.".to_string(),
            },
            TicketOption {
                id: "parceria".to_string(),
                label: "Contato para Parcerias".to_string(),
                description: "Feche parcerias entre servidores e projetos.".to_string(),
                emoji: "🤝".to_string(),
                reply: "🤝 **Atendimento de Parceria:**\n\nTenha em mãos o link do seu projeto e os requisitos mínimos necessários.\n\n👇 Clique abaixo para iniciar o processo.".to_string(),
            },
        ]
    }
}
