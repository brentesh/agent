use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub gpt_api_key: String,
    pub ebms_url: String,
    pub ebms_username: String,
    pub ebms_password: String,
    pub employee_id: String,
}

impl AppConfig {
    pub fn empty() -> Self {
        AppConfig {
            gpt_api_key: String::new(),
            ebms_url: String::new(),
            ebms_username: String::new(),
            ebms_password: String::new(),
            employee_id: String::new(),
        }
    }
}

const EBMS_API_AGENT: &str = "ebms_api_agent";

pub fn load_config() -> AppConfig {
    confy::load(EBMS_API_AGENT, None).unwrap_or(AppConfig::empty())
}

pub fn save_config(config: &AppConfig) {
    if let Err(e) = confy::store(EBMS_API_AGENT, None, config) {
        eprintln!("Failed to save config: {}", e);
    }
}
