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
use std::str;
#[allow(unused)]
pub fn encrypt(content: &str, password: &str) -> String {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let derive_key = derive_key(password, &salt);
    let key = derive_key.as_bytes();

    let mut iv = [0u8; 12];
    OsRng.fill_bytes(&mut iv);

    let key: &Key<Aes256Gcm> = key.into();
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&iv);
    let mut bytes = cipher.encrypt(nonce, content.as_bytes()).unwrap();

    let mut combined = vec![];
    combined.append(&mut salt.to_vec());
    combined.append(&mut iv.to_vec());
    combined.append(&mut bytes);
    general_purpose::STANDARD.encode(combined.as_slice())
}
#[allow(unused)]
pub fn decrypt(encrypted: &str, password: &str) -> String {
    let buffer = decode(encrypted);
    let buffer_slice = buffer.as_slice();
    let salt = &buffer_slice[0..16];
    let iv = &buffer_slice[16..28];
    let data = &buffer_slice[28..];

    let derive_key = derive_key(password, salt);
    let key = derive_key.as_bytes();

    // let key = Key::<Aes256Gcm>::from_slice(key);
    let key: &Key<Aes256Gcm> = key.into();
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&iv);
    let decrypted = cipher.decrypt(nonce, data).unwrap();
    String::from_utf8(decrypted).unwrap()
}

#[allow(unused)]
fn derive_key(password: &str, salt: &[u8]) -> Output {
    let params = Params {
        rounds: 12345,
        output_length: 32,
    };

    let salt_string = SaltString::encode_b64(salt).unwrap();
    let salt = Salt::from(&salt_string);
    let password = password.as_bytes();
    let key = Pbkdf2
        .hash_password_customized(password, None, None, params, salt)
        .unwrap();
    key.hash.unwrap()
}
#[allow(unused)]
fn decode(s: &str) -> Vec<u8> {
    general_purpose::STANDARD.decode(s).unwrap()
}

#[cfg(test)]
mod tests {
    use crate::crypto::{decrypt, encrypt};

    #[test]
    fn test_crypto() {
        let password = "password";
        let content = "中文测试 😍 언문.";
        let encrypted = encrypt(content, password);
        println!("{}", encrypted);

        let decrypted = decrypt(&encrypted, password);
        println!("{}", decrypted);
        assert_eq!(content, decrypted)
    }
}
