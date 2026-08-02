use sqlx::PgPool;
use tracing::error;

pub struct PaymentDb;

impl PaymentDb {
    pub async fn init(pool: &PgPool) {
        let q = "CREATE TABLE IF NOT EXISTS payments (
            payment_id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            package_id TEXT NOT NULL,
            created_at BIGINT NOT NULL
        )";

        if let Err(e) = sqlx::query(q).execute(pool).await {
            error!("Erro ao criar tabela payments: {}", e);
        }

        let idx_q = "CREATE INDEX IF NOT EXISTS idx_payments_created_at ON payments (created_at DESC)";
        if let Err(e) = sqlx::query(idx_q).execute(pool).await {
            error!("Erro ao criar índice idx_payments_created_at: {}", e);
        }
    }

    pub async fn add_payment(pool: &PgPool, payment_id: &str, user_id: &str, package_id: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        let q = "INSERT INTO payments (payment_id, user_id, package_id, created_at) VALUES ($1, $2, $3, $4)";
        sqlx::query(q)
            .bind(payment_id)
            .bind(user_id)
            .bind(package_id)
            .bind(now)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_pending_payments(pool: &PgPool) -> Result<Vec<(String, String, String)>, sqlx::Error> {
        let min_created_at = chrono::Utc::now().timestamp() - 86400; // últimas 24h
        let q = "SELECT payment_id, user_id, package_id FROM payments WHERE created_at >= $1 ORDER BY created_at DESC LIMIT 100";
        let rows = sqlx::query_as::<_, (String, String, String)>(q)
            .bind(min_created_at)
            .fetch_all(pool)
            .await?;
        Ok(rows)
    }

    pub async fn remove_payment(pool: &PgPool, payment_id: &str) -> Result<(), sqlx::Error> {
        let q = "DELETE FROM payments WHERE payment_id = $1";
        sqlx::query(q).bind(payment_id).execute(pool).await?;
        Ok(())
    }
}
