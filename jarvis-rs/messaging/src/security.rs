//! Validação de segurança para webhooks

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Valida assinatura HMAC-SHA256 para Telegram
pub fn validate_telegram_signature(
    secret: &str,
    payload: &[u8],
    signature: &str,
) -> anyhow::Result<bool> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("Invalid secret: {}", e))?;
    
    mac.update(payload);
    
    let expected_signature = hex::encode(mac.finalize().into_bytes());
    Ok(expected_signature == signature)
}

/// Valida verify_token para WhatsApp
pub fn validate_whatsapp_token(received: &str, expected: &str) -> bool {
    received == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whatsapp_token_validation() {
        assert!(validate_whatsapp_token("test_token", "test_token"));
        assert!(!validate_whatsapp_token("wrong", "test_token"));
    }

    #[test]
    fn test_telegram_signature_validation() {
        let secret = "test_secret";
        let payload = b"test_payload";
        
        // Criar assinatura válida
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let valid_signature = hex::encode(mac.finalize().into_bytes());
        
        assert!(validate_telegram_signature(secret, payload, &valid_signature).unwrap());
        assert!(!validate_telegram_signature(secret, payload, "invalid").unwrap());
    }
}
