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
        let q = "SELECT payment_id, user_id, package_id FROM payments";
        let rows = sqlx::query_as::<_, (String, String, String)>(q)
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
