//! AES-256-GCM 密码加密模块
//!
//! 用于加密存储用户密码，以便在 Subsonic token 认证时解密获取原始密码计算 MD5。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use application::auth::PasswordEncryptor;
use application::error::AppError;
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::Rng;
use std::sync::Arc;

/// AES-256-GCM 密码加密器
///
/// 加密格式: base64(nonce + ciphertext)
/// - nonce: 12 bytes (96 bits)
/// - ciphertext: 加密后的数据 + 16 bytes auth tag
#[derive(Clone)]
pub struct Aes256GcmEncryptor {
    cipher: Arc<Aes256Gcm>,
}

impl Aes256GcmEncryptor {
    /// 创建新的加密器
    ///
    /// # Arguments
    /// * `key` - 32 字节的密钥（256 bits），如果不足 32 字节会用 SHA-256 派生
    pub fn new(key: &str) -> Result<Self, AppError> {
        let key_bytes = Self::derive_key(key);
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| AppError::AuthError(format!("Invalid key: {}", e)))?;
        Ok(Self {
            cipher: Arc::new(cipher),
        })
    }

    /// 从任意长度的密钥派生 32 字节密钥
    fn derive_key(key: &str) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&result);
        key_bytes
    }
}

impl PasswordEncryptor for Aes256GcmEncryptor {
    fn encrypt(&self, plain_password: &str) -> Result<String, AppError> {
        // 生成随机 nonce (12 bytes)
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密
        let ciphertext = self
            .cipher
            .encrypt(nonce, plain_password.as_bytes())
            .map_err(|e| AppError::AuthError(format!("Encryption failed: {}", e)))?;

        // 组合 nonce + ciphertext 并 base64 编码
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(STANDARD.encode(&combined))
    }

    fn decrypt(&self, encrypted_password: &str) -> Result<String, AppError> {
        // Base64 解码
        let combined = STANDARD
            .decode(encrypted_password)
            .map_err(|e| AppError::AuthError(format!("Base64 decode failed: {}", e)))?;

        // 检查最小长度 (12 bytes nonce + 16 bytes auth tag)
        if combined.len() < 28 {
            return Err(AppError::AuthError(
                "Encrypted data too short".to_string(),
            ));
        }

        // 分离 nonce 和 ciphertext
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // 解密
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::AuthError(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::AuthError(format!("Invalid UTF-8: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let encryptor = Aes256GcmEncryptor::new("test_secret_key").unwrap();
        let password = "my_secret_password";

        let encrypted = encryptor.encrypt(password).unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();

        assert_eq!(password, decrypted);
    }

    #[test]
    fn test_different_encryptions() {
        let encryptor = Aes256GcmEncryptor::new("test_secret_key").unwrap();
        let password = "my_secret_password";

        // 每次加密应该产生不同的结果（因为 nonce 不同）
        let encrypted1 = encryptor.encrypt(password).unwrap();
        let encrypted2 = encryptor.encrypt(password).unwrap();

        assert_ne!(encrypted1, encrypted2);

        // 但都应该能正确解密
        assert_eq!(password, encryptor.decrypt(&encrypted1).unwrap());
        assert_eq!(password, encryptor.decrypt(&encrypted2).unwrap());
    }

    #[test]
    fn test_invalid_data() {
        let encryptor = Aes256GcmEncryptor::new("test_secret_key").unwrap();

        // 无效的 base64
        assert!(encryptor.decrypt("not_valid_base64!!!").is_err());

        // 太短的数据
        assert!(encryptor.decrypt("YWJj").is_err()); // "abc" in base64

        // 使用不同密钥加密的数据
        let other_encryptor = Aes256GcmEncryptor::new("other_key").unwrap();
        let encrypted = other_encryptor.encrypt("password").unwrap();
        assert!(encryptor.decrypt(&encrypted).is_err());
    }
}
