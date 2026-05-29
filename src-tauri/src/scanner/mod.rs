pub mod ping;
pub mod probe;
pub mod ranges;
pub mod bedrock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub ip: String,
    pub port: u16,
    pub motd: String,
    pub protocol: i32,
    pub version: String,
    pub online_players: i32,
    pub max_players: i32,
    pub player_sample: Vec<PlayerSample>,
    pub ping_ms: i64,
    pub modded: bool,
    pub mod_list: Vec<String>,
    pub whitelisted: Option<bool>,
    pub category: ServerCategory,
    pub tags: Vec<String>,
    pub last_seen: String,
    pub first_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSample {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerCategory {
    VanillaSurvival,
    Modded,
    PluginHeavy,
    Creative,
    Minigame,
    Anarchy,
    PrivateGroup,
    Idle,
    Unknown,
}

impl ServerCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServerCategory::VanillaSurvival => "vanilla_survival",
            ServerCategory::Modded => "modded",
            ServerCategory::PluginHeavy => "plugin_heavy",
            ServerCategory::Creative => "creative",
            ServerCategory::Minigame => "minigame",
            ServerCategory::Anarchy => "anarchy",
            ServerCategory::PrivateGroup => "private_group",
            ServerCategory::Idle => "idle",
            ServerCategory::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "vanilla_survival" => ServerCategory::VanillaSurvival,
            "modded" => ServerCategory::Modded,
            "plugin_heavy" => ServerCategory::PluginHeavy,
            "creative" => ServerCategory::Creative,
            "minigame" => ServerCategory::Minigame,
            "anarchy" => ServerCategory::Anarchy,
            "private_group" => ServerCategory::PrivateGroup,
            "idle" => ServerCategory::Idle,
            _ => ServerCategory::Unknown,
        }
    }
}

impl Serialize for ServerCategory {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ServerCategory {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(ServerCategory::from_str(&s))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanTarget {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub scanned: u64,
    pub total: u64,
    pub found: u64,
    pub current_ip: String,
}
