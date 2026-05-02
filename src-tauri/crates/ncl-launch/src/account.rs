use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 启动游戏使用的账号信息。Sprint 1 仅离线账号。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub username: String,
    /// 用 dashes 形式的 UUID 字符串（Mojang 启动参数也接受去 dash 形式，但保留是更稳）
    pub uuid: String,
    /// 离线账号填 "0"；正版/第三方账号填实际 token
    pub access_token: String,
    /// `msa` / `mojang` / `legacy`；离线惯例填 `msa`（HMCL/PCL 通行）
    pub user_type: String,
}

impl AccountInfo {
    /// 构造离线账号。UUID 由用户名通过 `MD5("OfflinePlayer:<name>")` 派生（v3 风格），
    /// 这与 HMCL / PCL2 / 官方启动器在离线模式下的算法一致——保证同名玩家
    /// 在不同 NCL 实例间得到稳定 UUID。
    #[must_use]
    pub fn offline(username: impl Into<String>) -> Self {
        let username = username.into();
        let uuid = offline_uuid(&username);
        Self {
            username,
            uuid: uuid.hyphenated().to_string(),
            access_token: "0".to_string(),
            user_type: "msa".to_string(),
        }
    }
}

/// 由用户名派生离线 UUID（version 3 规则）。
///
/// 算法：`MD5(b"OfflinePlayer:<name>")` → 设置 v3 + variant bits → `Uuid`。
#[must_use]
pub fn offline_uuid(name: &str) -> Uuid {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{name}").as_bytes());
    let digest = hasher.finalize();
    let mut bytes: [u8; 16] = digest.into();
    // 设置 version = 3
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    // 设置 variant = RFC4122
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_is_stable_for_same_name() {
        let a = offline_uuid("Steve");
        let b = offline_uuid("Steve");
        assert_eq!(a, b);
    }

    #[test]
    fn offline_uuid_differs_per_name() {
        assert_ne!(offline_uuid("Steve"), offline_uuid("Alex"));
    }

    #[test]
    fn offline_uuid_is_version_3() {
        let u = offline_uuid("anyone");
        let bytes = u.as_bytes();
        // version nibble (high 4 bits of byte 6) must be 3
        assert_eq!(bytes[6] >> 4, 0x3);
        // variant: high 2 bits of byte 8 must be 0b10
        assert_eq!(bytes[8] >> 6, 0b10);
    }

    #[test]
    fn account_offline_populates_fields() {
        let acc = AccountInfo::offline("Player1");
        assert_eq!(acc.username, "Player1");
        assert_eq!(acc.access_token, "0");
        assert_eq!(acc.user_type, "msa");
        assert!(acc.uuid.contains('-'), "uuid should be hyphenated");
    }
}
