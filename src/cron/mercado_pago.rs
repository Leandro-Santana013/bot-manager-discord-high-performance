use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::error;
use std::env;

#[derive(Serialize)]
struct PayerIdentification {
    #[serde(rename = "type")]
    id_type: String,
    number: String,
}

#[derive(Serialize)]
struct Payer {
    email: String,
    first_name: String,
    last_name: String,
    identification: PayerIdentification,
}

#[derive(Serialize)]
struct PaymentRequest {
    transaction_amount: f64,
    description: String,
    payment_method_id: String,
    payer: Payer,
}

#[derive(Deserialize, Debug)]
pub struct PaymentResponse {
    pub id: u64,
    pub point_of_interaction: Option<PointOfInteraction>,
    pub status: String,
}

#[derive(Deserialize, Debug)]
pub struct PointOfInteraction {
    pub transaction_data: TransactionData,
}

#[derive(Deserialize, Debug)]
pub struct TransactionData {
    pub qr_code: String,
    pub qr_code_base64: String,

}

pub struct MercadoPagoClient {
    client: Client,
    access_token: String,
}

impl MercadoPagoClient {
    pub fn new() -> Self {
        let token = env::var("MP_ACCESS_TOKEN").unwrap_or_else(|_| "APP_USR-test".to_string());
        Self {
            client: Client::new(),
            access_token: token,
        }
    }

    pub async fn create_pix_payment(
        &self,
        valor: f64,
        email: String,
        nome: String,
        cpf: String,
        description: String,
    ) -> Result<PaymentResponse, Box<dyn std::error::Error + Send + Sync>> {
        let idempotency_key = format!("{}-{}", chrono::Utc::now().timestamp_millis(), uuid::Uuid::new_v4());
        
        let cpf_clean: String = cpf.chars().filter(|c| c.is_digit(10)).collect();
        let parts: Vec<&str> = nome.split_whitespace().collect();
        let first_name = parts.get(0).unwrap_or(&"User").to_string();
        let last_name = if parts.len() > 1 {
            parts[1..].join(" ")
        } else {
            "User".to_string()
        };

        let payload = PaymentRequest {
            transaction_amount: valor,
            description,
            payment_method_id: "pix".to_string(),
            payer: Payer {
                email,
                first_name,
                last_name,
                identification: PayerIdentification {
                    id_type: "CPF".to_string(),
                    number: cpf_clean,
                },
            },
        };

        let res = self.client.post("https://api.mercadopago.com/v1/payments")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("X-Idempotency-Key", idempotency_key)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await?;
            error!("Erro na API MercadoPago (Criar PIX): {}", error_text);
            return Err("Erro na API MercadoPago".into());
        }

        let payment_res = res.json::<PaymentResponse>().await?;
        Ok(payment_res)
    }

    pub async fn get_payment_status(&self, payment_id: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("https://api.mercadopago.com/v1/payments/{}", payment_id);
        
        let res = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await?;
            error!("Erro na API MercadoPago (Check Status): {}", error_text);
            return Err("Erro na API MercadoPago".into());
        }

        let payment_res = res.json::<PaymentResponse>().await?;
        Ok(payment_res.status)
    }
}
