pub mod crm;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRMConfig {
    pub provider: CRMProvider,
    pub api_key: String,
    pub instance_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CRMProvider {
    Salesforce,
    HubSpot,
    None,
}

impl Default for CRMConfig {
    fn default() -> Self {
        Self {
            provider: CRMProvider::None,
            api_key: String::new(),
            instance_url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRMContact {
    pub id: Option<String>,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub company: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRMNote {
    pub contact_id: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRMSyncResult {
    pub success: bool,
    pub message: String,
    pub record_id: Option<String>,
}
