use sqlx::PgPool;
use tracing::error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VipBlock {
    pub id: String,
    pub title: String,
    pub desc: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VipProduct {
    pub id: String,
    pub label: String,
    pub price: String,
    pub role_id: String,
}

pub struct VipDb;

impl VipDb {
    pub async fn init(pool: &PgPool) {
        let q = "CREATE TABLE IF NOT EXISTS vip_config (
            chave TEXT PRIMARY KEY,
            valor TEXT
        )";

        if let Err(e) = sqlx::query(q).execute(pool).await {
            error!("Erro ao criar tabela vip_config: {}", e);
        }
    }

    pub async fn get_config(pool: &PgPool, key: &str, default_value: &str) -> String {
        let q = "SELECT valor FROM vip_config WHERE chave = $1";
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
        let q = "INSERT INTO vip_config (chave, valor) VALUES ($1, $2) ON CONFLICT(chave) DO UPDATE SET valor = EXCLUDED.valor";
        sqlx::query(q)
            .bind(key)
            .bind(value)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_main_text(pool: &PgPool) -> String {
        let default_desc = "**@Amethyst** por apenas **R$ 10**\nVantagens:\n\n💎 *Permissão de foto*\n💎 *Permissão de entrar nas call's da categoria:* `REAL`\n\n(Selecione abaixo o seu pacote VIP ou adicione saldo)";
        Self::get_config(pool, "main_desc", default_desc).await
    }

    pub async fn get_main_image(pool: &PgPool) -> String {
        Self::get_config(pool, "main_image", "").await
    }

    pub async fn set_main_text(pool: &PgPool, desc: &str, image: &str) {
        let _ = Self::set_config(pool, "main_desc", desc).await;
        let _ = Self::set_config(pool, "main_image", image).await;
    }

    pub async fn get_extra_blocks(pool: &PgPool) -> Vec<VipBlock> {
        let raw = Self::get_config(pool, "extra_blocks", "[]").await;
        serde_json::from_str(&raw).unwrap_or_else(|_| vec![])
    }

    pub async fn save_extra_block(pool: &PgPool, id: String, title: String, desc: String, color: String) {
        let mut blocks = Self::get_extra_blocks(pool).await;
        let data = VipBlock { id: id.clone(), title, desc, color };
        
        if let Some(pos) = blocks.iter().position(|b| b.id == id) {
            blocks[pos] = data;
        } else {
            blocks.push(data);
        }
        
        let json = serde_json::to_string(&blocks).unwrap_or_else(|_| "[]".to_string());
        let _ = Self::set_config(pool, "extra_blocks", &json).await;
    }

    pub async fn delete_extra_block(pool: &PgPool, id: &str) {
        let mut blocks = Self::get_extra_blocks(pool).await;
        blocks.retain(|b| b.id != id);
        let json = serde_json::to_string(&blocks).unwrap_or_else(|_| "[]".to_string());
        let _ = Self::set_config(pool, "extra_blocks", &json).await;
    }

    pub async fn get_products(pool: &PgPool) -> Vec<VipProduct> {
        let raw = Self::get_config(pool, "vip_products", "").await;
        if raw.is_empty() {
            let defaults = vec![
                VipProduct { id: "vip_partner".into(), label: "Partner - 200R$".into(), price: "200".into(), role_id: "".into() },
                VipProduct { id: "vip_black".into(), label: "Black - 100R$".into(), price: "100".into(), role_id: "".into() },
                VipProduct { id: "vip_diamond".into(), label: "Diamond - 50R$".into(), price: "50".into(), role_id: "".into() },
                VipProduct { id: "vip_amethyst".into(), label: "Amethyst - 10R$".into(), price: "10".into(), role_id: "".into() },
                VipProduct { id: "vip_teste".into(), label: "Teste - 8R$".into(), price: "8".into(), role_id: "".into() },
            ];
            let json = serde_json::to_string(&defaults).unwrap_or_else(|_| "[]".to_string());
            let _ = Self::set_config(pool, "vip_products", &json).await;
            return defaults;
        }
        serde_json::from_str(&raw).unwrap_or_else(|_| vec![])
    }

    pub async fn save_product(pool: &PgPool, id: String, label: String, price: String, role_id: String) {
        let mut prods = Self::get_products(pool).await;
        let data = VipProduct { id: id.clone(), label, price, role_id };
        
        if let Some(pos) = prods.iter().position(|p| p.id == id) {
            prods[pos] = data;
        } else {
            prods.push(data);
        }
        
        let json = serde_json::to_string(&prods).unwrap_or_else(|_| "[]".to_string());
        let _ = Self::set_config(pool, "vip_products", &json).await;
    }

    pub async fn delete_product(pool: &PgPool, id: &str) {
        let mut prods = Self::get_products(pool).await;
        prods.retain(|p| p.id != id);
        let json = serde_json::to_string(&prods).unwrap_or_else(|_| "[]".to_string());
        let _ = Self::set_config(pool, "vip_products", &json).await;
    }

    pub async fn get_panel_message(pool: &PgPool) -> (String, String) {
        let ch = Self::get_config(pool, "panel_channel_id", "").await;
        let msg = Self::get_config(pool, "panel_message_id", "").await;
        (ch, msg)
    }

    pub async fn set_panel_message(pool: &PgPool, channel_id: &str, message_id: &str) {
        let _ = Self::set_config(pool, "panel_channel_id", channel_id).await;
        let _ = Self::set_config(pool, "panel_message_id", message_id).await;
    }
}
