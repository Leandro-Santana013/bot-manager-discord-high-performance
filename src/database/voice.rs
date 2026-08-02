use sqlx::PgPool;
use tracing::error;
use sqlx::Row;

pub struct VoiceDb;

pub struct UserVoiceStats {
    pub total_ms: i64,
    pub this_week_ms: i64,
    pub last_week_ms: i64,
    pub this_week_muted_ms: i64,
    pub last_week_muted_ms: i64,
    pub rank: i64,
}

pub struct UserClosingStats {
    pub id_usuario: String,
    pub this_week_ms: i64,
    pub days_inactive: i64,
}

impl VoiceDb {
    pub async fn init(pool: &PgPool) {
        let q1 = "CREATE TABLE IF NOT EXISTS usuarios (
            id_usuario TEXT PRIMARY KEY,
            tempo_total BIGINT DEFAULT 0
        )";
        let q2 = "CREATE TABLE IF NOT EXISTS sessoes_voz (
            id SERIAL PRIMARY KEY,
            id_usuario TEXT,
            tempo BIGINT,
            tempo_mutado BIGINT DEFAULT 0,
            data_sessao TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )";

        let q_index1 = "CREATE INDEX IF NOT EXISTS idx_sessoes_usuario ON sessoes_voz (id_usuario)";
        let q_index2 = "CREATE INDEX IF NOT EXISTS idx_sessoes_data ON sessoes_voz (data_sessao)";

        if let Err(e) = sqlx::query(q1).execute(pool).await {
            error!("Erro ao criar tabela usuarios: {}", e);
        }
        if let Err(e) = sqlx::query(q2).execute(pool).await {
            error!("Erro ao criar tabela sessoes_voz: {}", e);
        }
        let _ = sqlx::query(q_index1).execute(pool).await;
        let _ = sqlx::query(q_index2).execute(pool).await;

        // Migração segura para colunas e tipos BIGINT no PostgreSQL
        let _ = sqlx::query("ALTER TABLE sessoes_voz ADD COLUMN IF NOT EXISTS tempo_mutado BIGINT DEFAULT 0").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE sessoes_voz ALTER COLUMN tempo TYPE BIGINT").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE sessoes_voz ALTER COLUMN tempo_mutado TYPE BIGINT").execute(pool).await;
    }

    pub async fn update_user_time(pool: &PgPool, user_id: &str, time_spent: i64, mute_time_spent: i64) -> Result<(), sqlx::Error> {
        let q1 = "INSERT INTO usuarios (id_usuario, tempo_total) VALUES ($1, $2) ON CONFLICT(id_usuario) DO UPDATE SET tempo_total = usuarios.tempo_total + EXCLUDED.tempo_total";
        sqlx::query(q1)
            .bind(user_id)
            .bind(time_spent)
            .execute(pool)
            .await?;

        let q2 = "INSERT INTO sessoes_voz (id_usuario, tempo, tempo_mutado) VALUES ($1, $2, $3)";
        sqlx::query(q2)
            .bind(user_id)
            .bind(time_spent)
            .bind(mute_time_spent)
            .execute(pool)
            .await?;
            
        Ok(())
    }

    pub async fn get_user_stats(pool: &PgPool, user_id: &str) -> UserVoiceStats {
        let total_row = sqlx::query("SELECT tempo_total FROM usuarios WHERE id_usuario = $1")
            .bind(user_id)
            .fetch_optional(pool).await.unwrap_or(None);
        let total_ms: i64 = total_row.map(|r| r.get("tempo_total")).unwrap_or(0);

        let rank_row = sqlx::query("SELECT COUNT(*) as rank FROM usuarios WHERE tempo_total > $1")
            .bind(total_ms)
            .fetch_one(pool).await;
        let rank: i64 = rank_row.map(|r| r.get::<i64, _>("rank")).unwrap_or(0) + 1;

        let this_week_row = sqlx::query("SELECT CAST(COALESCE(SUM(tempo), 0) AS BIGINT) as total, CAST(COALESCE(SUM(tempo_mutado), 0) AS BIGINT) as total_mutado FROM sessoes_voz WHERE id_usuario = $1 AND data_sessao >= NOW() - INTERVAL '7 days'")
            .bind(user_id)
            .fetch_optional(pool).await.unwrap_or(None);
        let this_week_ms: i64 = this_week_row.as_ref().map(|r| r.try_get("total").unwrap_or(0)).unwrap_or(0);
        let this_week_muted_ms: i64 = this_week_row.as_ref().map(|r| r.try_get("total_mutado").unwrap_or(0)).unwrap_or(0);

        let last_week_row = sqlx::query("SELECT CAST(COALESCE(SUM(tempo), 0) AS BIGINT) as total, CAST(COALESCE(SUM(tempo_mutado), 0) AS BIGINT) as total_mutado FROM sessoes_voz WHERE id_usuario = $1 AND data_sessao >= NOW() - INTERVAL '14 days' AND data_sessao < NOW() - INTERVAL '7 days'")
            .bind(user_id)
            .fetch_optional(pool).await.unwrap_or(None);
        let last_week_ms: i64 = last_week_row.as_ref().map(|r| r.try_get("total").unwrap_or(0)).unwrap_or(0);
        let last_week_muted_ms: i64 = last_week_row.as_ref().map(|r| r.try_get("total_mutado").unwrap_or(0)).unwrap_or(0);

        UserVoiceStats {
            total_ms,
            this_week_ms,
            last_week_ms,
            this_week_muted_ms,
            last_week_muted_ms,
            rank,
        }
    }

    pub async fn get_all_users_time(pool: &PgPool) -> Vec<(String, i64)> {
        let rows = sqlx::query("SELECT id_usuario, tempo_total FROM usuarios")
            .fetch_all(pool).await.unwrap_or_else(|_| vec![]);
        
        let mut list = Vec::new();
        for row in rows {
            let id: String = row.get("id_usuario");
            let time: i64 = row.get("tempo_total");
            list.push((id, time));
        }
        list
    }

    pub async fn get_all_users_closing_stats(pool: &PgPool) -> Vec<UserClosingStats> {
        let rows = sqlx::query("
            SELECT u.id_usuario,
                   (SELECT SUM(tempo) FROM sessoes_voz s WHERE s.id_usuario = u.id_usuario AND s.data_sessao >= NOW() - INTERVAL '7 days') as week_ms,
                   (SELECT CAST(EXTRACT(DAY FROM (NOW() - MAX(data_sessao))) AS INTEGER) FROM sessoes_voz s WHERE s.id_usuario = u.id_usuario) as inactive_days
            FROM usuarios u
        ").fetch_all(pool).await.unwrap_or_else(|_| vec![]);

        let mut list = Vec::new();
        for row in rows {
            let id_usuario: String = row.get("id_usuario");
            let this_week_ms: i64 = row.try_get("week_ms").unwrap_or(0);
            let days_inactive: i64 = row.try_get("inactive_days").unwrap_or(0);
            
            list.push(UserClosingStats {
                id_usuario,
                this_week_ms,
                days_inactive,
            });
        }
        list
    }

    pub async fn reset_user_total(pool: &PgPool, user_id: &str) {
        let _ = sqlx::query("UPDATE usuarios SET tempo_total = 0 WHERE id_usuario = $1")
            .bind(user_id)
            .execute(pool).await;
    }
}
