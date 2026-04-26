use crate::AppState;
use anyhow::Result;
use chrono::{Duration, Utc};

pub struct InvoiceResponse {
    pub invoice: String,
    pub payment_hash: String,
    pub amount_sats: i64,
}

pub async fn create_escrow_invoice(state: &AppState, amount_sats: i64, task_id: &str) -> Result<InvoiceResponse> {
    tracing::info!("Creating escrow invoice for {} sats, task={}", amount_sats, task_id);
    
    let description = format!("OAN Task Escrow: {}", task_id);
    let mdk_invoice = state.wallet.receive(amount_sats, Some(&description)).await?;
    
    Ok(InvoiceResponse {
        invoice: mdk_invoice.invoice,
        payment_hash: mdk_invoice.payment_hash,
        amount_sats,
    })
}

pub async fn create_variable_invoice(state: &AppState, description: Option<&str>) -> Result<InvoiceResponse> {
    tracing::info!("Creating variable amount invoice");
    
    let mdk_invoice = state.wallet.receive_variable(description).await?;
    
    Ok(InvoiceResponse {
        invoice: mdk_invoice.invoice,
        payment_hash: mdk_invoice.payment_hash,
        amount_sats: 0,
    })
}

pub async fn pay_to_lightning_address(state: &AppState, ln_address: &str, amount_sats: i64) -> Result<String> {
    tracing::info!("Paying {} sats to Lightning address: {}", amount_sats, ln_address);
    
    let result = state.wallet.send(ln_address, Some(amount_sats)).await?;
    Ok(result.payment_hash)
}

pub async fn get_balance(state: &AppState) -> Result<i64> {
    state.wallet.balance().await
}

pub mod l402 {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use sha2::Sha256;
    use hmac::{Hmac, Mac};

    type HmacSha256 = Hmac<Sha256>;

    const L402_SECRET: &[u8] = b"oan-l402-secret-key-change-in-production";

    pub struct Macaroon {
        pub resource: String,
        pub amount_sats: u64,
        pub expires_at: i64,
        pub signature: String,
    }

    impl Macaroon {
        pub fn new(resource: &str, amount_sats: u64, expires_at: i64) -> Self {
            let mut mac = HmacSha256::new_from_slice(L402_SECRET).expect("HMAC init failed");
            mac.update(resource.as_bytes());
            mac.update(&amount_sats.to_le_bytes());
            mac.update(&expires_at.to_le_bytes());
            let result = mac.finalize();
            let signature = hex::encode(result.into_bytes());

            Self {
                resource: resource.to_string(),
                amount_sats,
                expires_at,
                signature,
            }
        }

        pub fn encode(&self) -> String {
            let payload = format!("{}:{}:{}:{}", self.resource, self.amount_sats, self.expires_at, self.signature);
            BASE64.encode(payload)
        }

        pub fn decode(encoded: &str) -> Result<Self> {
            let decoded = BASE64.decode(encoded.as_bytes())?;
            let payload = String::from_utf8(decoded)?;
            let parts: Vec<&str> = payload.split(':').collect();
            
            if parts.len() != 4 {
                anyhow::bail!("Invalid macaroon format");
            }

            let macaroon = Macaroon {
                resource: parts[0].to_string(),
                amount_sats: parts[1].parse()?,
                expires_at: parts[2].parse()?,
                signature: parts[3].to_string(),
            };

            let expected = Macaroon::new(&macaroon.resource, macaroon.amount_sats, macaroon.expires_at);
            if macaroon.signature != expected.signature {
                anyhow::bail!("Invalid macaroon signature");
            }

            Ok(macaroon)
        }
    }

    pub fn verify_preimage(preimage: &str, _payment_hash: &str) -> Result<bool> {
        if preimage.len() >= 32 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn create_challenge(state: &AppState, resource: &str, amount_sats: u64) -> Result<crate::models::L402Challenge> {
        let expires_at = Utc::now() + Duration::minutes(15);
        let macaroon = Macaroon::new(resource, amount_sats, expires_at.timestamp());
        
        let invoice = create_escrow_invoice(state, amount_sats as i64, "l402").await?;

        Ok(crate::models::L402Challenge {
            macaroon: macaroon.encode(),
            invoice: invoice.invoice,
            payment_hash: invoice.payment_hash,
            amount_sats,
            expires_at: expires_at.to_rfc3339(),
        })
    }

    pub async fn verify_macaroon(state: &AppState, encoded_macaroon: &str, _preimage: &str) -> Result<bool> {
        let macaroon = Macaroon::decode(encoded_macaroon)?;
        
        let now = Utc::now().timestamp();
        if now > macaroon.expires_at {
            anyhow::bail!("Macaroon expired");
        }

        let _payment = crate::db::payments::find_by_hash(&state.db, &macaroon.resource).await?;
        
        Ok(true)
    }
}
