fn get_env_or_panic(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("环境变量 {} 未设置", key))
}

pub fn todoist_client_id() -> String {
    get_env_or_panic("TODOIST_CLIENT_ID")
}

pub fn todoist_client_secret() -> String {
    get_env_or_panic("TODOIST_CLIENT_SECRET")
}

pub const TODOIST_SCOPE: &str = "data:read_write,data:delete,project:delete";
pub const BACKUP_VERSION: &str = "2.0";
pub const UPDATE_TIMEOUT: i32 = 1500;
pub const DESTROY_TIMEOUT: i32 = 750;
pub const SYNC_TIMEOUT: i32 = 2500;
pub const SHORT_NAME_SIZE: usize = 20;
pub const PRIORITY_1: i32 = 4;
pub const PRIORITY_2: i32 = 3;
pub const PRIORITY_3: i32 = 2;
pub const PRIORITY_4: i32 = 1;
pub const SCROLL_STEPS: i32 = 6;
pub const TWITTER_URL: &str = "https://twitter.com/useplanify";
pub const CONTACT_US: &str = "linruohan@126.com";
pub const TELEGRAM_GROUP: &str = "https://t.me/+cArNTCbdT3xmOTcx";
pub const PATREON_URL: &str = "https://www.patreon.com/join/alainm23";
pub const PAYPAL_ME_URL: &str = "https://www.paypal.com/paypalme/alainm23";
pub const LIBERAPAY_URL: &str = "https://liberapay.com/Alain/";
pub const KOFI_URL: &str = "https://ko-fi.com/alainm23";
pub const MATRIX_URL: &str = "https://matrix.to/#/#useplanify:matrix.org";
pub const MASTODON_URL: &str = "https://mastodon.social/@planifyapp";
pub const SHOW_WHATSNEW: bool = false;
pub const BLOCK_PAST_DAYS: bool = false;
pub const INBOX_PROJECT_ID: &str = "";
