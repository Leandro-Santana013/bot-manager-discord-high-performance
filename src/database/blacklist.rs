use sqlx::PgPool;
use sqlx::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BlacklistPanel {
    pub msg_id: String,
    pub channel_id: String,
    pub guild_id: String,
    pub role_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlacklistUser {
    pub guild_id: String,
    pub user_id: String,
    pub role_id: String,
    pub expires_at: i64,
    pub panel_msg_id: String,
    pub panel_channel_id: String,
}

pub struct BlacklistDb;

impl BlacklistDb {
    pub async fn init(pool: &PgPool) {
        let query1 = "
            CREATE TABLE IF NOT EXISTS blacklist_panels (
                msg_id TEXT PRIMARY KEY,
                channel_id TEXT NOT NULL,
                guild_id TEXT NOT NULL,
                role_id TEXT NOT NULL
            )
        ";
        if let Err(e) = sqlx::query(query1).execute(pool).await {
            tracing::error!("Failed to create blacklist_panels table: {}", e);
        }

        let query2 = "
            CREATE TABLE IF NOT EXISTS blacklist_users (
                guild_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                expires_at BIGINT NOT NULL,
                panel_msg_id TEXT NOT NULL,
                panel_channel_id TEXT NOT NULL,
                PRIMARY KEY (guild_id, user_id)
            )
        ";
        if let Err(e) = sqlx::query(query2).execute(pool).await {
            tracing::error!("Failed to create blacklist_users table: {}", e);
        }
    }

    pub async fn add_panel(pool: &PgPool, msg_id: &str, channel_id: &str, guild_id: &str, role_id: &str) -> Result<(), sqlx::Error> {
        let query = "INSERT INTO blacklist_panels (msg_id, channel_id, guild_id, role_id) VALUES ($1, $2, $3, $4) ON CONFLICT (msg_id) DO UPDATE SET channel_id = EXCLUDED.channel_id, guild_id = EXCLUDED.guild_id, role_id = EXCLUDED.role_id";
        sqlx::query(query)
            .bind(msg_id)
            .bind(channel_id)
            .bind(guild_id)
            .bind(role_id)
            .execute(pool).await?;
        Ok(())
    }

    pub async fn get_panel(pool: &PgPool, msg_id: &str) -> Option<BlacklistPanel> {
        let query = "SELECT * FROM blacklist_panels WHERE msg_id = $1";
        let row = sqlx::query(query).bind(msg_id).fetch_optional(pool).await.ok()??;
        Some(BlacklistPanel {
            msg_id: row.get("msg_id"),
            channel_id: row.get("channel_id"),
            guild_id: row.get("guild_id"),
            role_id: row.get("role_id"),
        })
    }

    pub async fn add_user(pool: &PgPool, guild_id: &str, user_id: &str, role_id: &str, expires_at: i64, panel_msg_id: &str, panel_channel_id: &str) -> Result<(), sqlx::Error> {
        let query = "INSERT INTO blacklist_users (guild_id, user_id, role_id, expires_at, panel_msg_id, panel_channel_id) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (guild_id, user_id) DO UPDATE SET role_id = EXCLUDED.role_id, expires_at = EXCLUDED.expires_at, panel_msg_id = EXCLUDED.panel_msg_id, panel_channel_id = EXCLUDED.panel_channel_id";
        sqlx::query(query)
            .bind(guild_id)
            .bind(user_id)
            .bind(role_id)
            .bind(expires_at)
            .bind(panel_msg_id)
            .bind(panel_channel_id)
            .execute(pool).await?;
        Ok(())
    }

    pub async fn remove_user(pool: &PgPool, guild_id: &str, user_id: &str) -> Result<(), sqlx::Error> {
        let query = "DELETE FROM blacklist_users WHERE guild_id = $1 AND user_id = $2";
        sqlx::query(query)
            .bind(guild_id)
            .bind(user_id)
            .execute(pool).await?;
        Ok(())
    }

    pub async fn get_users_for_panel(pool: &PgPool, panel_msg_id: &str) -> Vec<BlacklistUser> {
        let query = "SELECT * FROM blacklist_users WHERE panel_msg_id = $1";
        let rows = sqlx::query(query).bind(panel_msg_id).fetch_all(pool).await.unwrap_or_else(|_| vec![]);
        rows.into_iter().map(|row| BlacklistUser {
            guild_id: row.get("guild_id"),
            user_id: row.get("user_id"),
            role_id: row.get("role_id"),
            expires_at: row.get("expires_at"),
            panel_msg_id: row.get("panel_msg_id"),
            panel_channel_id: row.get("panel_channel_id"),
        }).collect()
    }

    pub async fn get_expired_users(pool: &PgPool, current_time: i64) -> Vec<BlacklistUser> {
        let query = "SELECT * FROM blacklist_users WHERE expires_at <= $1";
        let rows = sqlx::query(query).bind(current_time).fetch_all(pool).await.unwrap_or_else(|_| vec![]);
        rows.into_iter().map(|row| BlacklistUser {
            guild_id: row.get("guild_id"),
            user_id: row.get("user_id"),
            role_id: row.get("role_id"),
            expires_at: row.get("expires_at"),
            panel_msg_id: row.get("panel_msg_id"),
            panel_channel_id: row.get("panel_channel_id"),
        }).collect()
    }

    pub async fn get_next_expiration(pool: &PgPool) -> Option<i64> {
        let query = "SELECT MIN(expires_at) as next_exp FROM blacklist_users";
        let row = sqlx::query(query).fetch_optional(pool).await.ok()??;
        row.try_get::<i64, _>("next_exp").ok()
    }
}
