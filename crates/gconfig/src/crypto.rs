use std::str;

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose};
use password_hash::{
    Output, PasswordHasher, Salt, SaltString,
    rand_core::{OsRng, RngCore},
};
use pbkdf2::{Params, Pbkdf2};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("utf8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("password hashing error")]
    PasswordHashError,
    #[error("encryption/decryption error")]
    AeadError,
    #[error("invalid encrypted format")]
    InvalidFormat,
}

pub fn encrypt(content: &str, password: &str) -> Result<String, CryptoError> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let derive_key = derive_key(password, &salt)?;
    let key = derive_key.as_bytes();

    let mut iv = [0u8; 12];
    OsRng.fill_bytes(&mut iv);

    let key: &Key<Aes256Gcm> = key.into();
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from(iv); // Use from() instead of from_slice()
    let mut bytes =
        cipher.encrypt(&nonce, content.as_bytes()).map_err(|_| CryptoError::AeadError)?;

    let mut combined = vec![];
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&iv);
    combined.append(&mut bytes);
    Ok(general_purpose::STANDARD.encode(combined.as_slice()))
}

pub fn decrypt(encrypted: &str, password: &str) -> Result<String, CryptoError> {
    let buffer = decode(encrypted)?;
    if buffer.len() < 28 {
        return Err(CryptoError::InvalidFormat);
    }
    let buffer_slice = buffer.as_slice();
    let salt = &buffer_slice[0..16];
    let iv = &buffer_slice[16..28];
    let data = &buffer_slice[28..];

    // 将 iv 切片转换为固定大小数组
    let iv: [u8; 12] = iv.try_into().map_err(|_| CryptoError::InvalidFormat)?;

    let derive_key = derive_key(password, salt)?;
    let key = derive_key.as_bytes();
    let key: &Key<Aes256Gcm> = key.into();
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from(iv);
    let decrypted = cipher.decrypt(&nonce, data).map_err(|_| CryptoError::AeadError)?;
    Ok(String::from_utf8(decrypted)?)
}

fn derive_key(password: &str, salt: &[u8]) -> Result<Output, CryptoError> {
    // 使用 OWASP 推荐的迭代次数（至少 100,000 次）
    // 参考: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
    let params = Params { rounds: 600_000, output_length: 32 };

    let salt_string = SaltString::encode_b64(salt).map_err(|_| CryptoError::PasswordHashError)?;
    let salt = Salt::from(&salt_string);
    let password = password.as_bytes();
    let key = Pbkdf2
        .hash_password_customized(password, None, None, params, salt)
        .map_err(|_| CryptoError::PasswordHashError)?;
    key.hash.ok_or(CryptoError::PasswordHashError)
}
fn decode(s: &str) -> Result<Vec<u8>, CryptoError> {
    Ok(general_purpose::STANDARD.decode(s)?)
}

#[cfg(test)]
mod tests {
    use crate::crypto::{decrypt, encrypt};

    #[test]
    fn test_crypto() {
        let password = "password";
        let content = "中文测试 😍 언문.";
        let encrypted = encrypt(content, password).expect("encrypt failed");
        println!("{}", encrypted);

        let decrypted = decrypt(&encrypted, password).expect("decrypt failed");
        println!("{}", decrypted);
        assert_eq!(content, decrypted)
    }
}
