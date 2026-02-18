use reqwest::Client;
use serde::Deserialize;
use parking_lot::Mutex;
use std::sync::Arc;

use super::{CRMConfig, CRMContact, CRMNote, CRMProvider, CRMSyncResult};

type CRMState = Arc<Mutex<CRMConfig>>;

// ─── Salesforce ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SalesforceCreateResponse {
    id: Option<String>,
    success: Option<bool>,
}

async fn salesforce_create_contact(
    config: &CRMConfig,
    contact: &CRMContact,
) -> Result<CRMSyncResult, String> {
    let client = Client::new();

    let body = serde_json::json!({
        "FirstName": contact.first_name,
        "LastName": contact.last_name,
        "Email": contact.email,
        "Phone": contact.phone,
        "Company": contact.company,
    });

    let url = format!("{}/services/data/v59.0/sobjects/Contact/", config.instance_url);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Salesforce request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Ok(CRMSyncResult {
            success: false,
            message: format!("Salesforce error ({}): {}", status, body),
            record_id: None,
        });
    }

    let result: SalesforceCreateResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(CRMSyncResult {
        success: result.success.unwrap_or(false),
        message: "Contact created in Salesforce".to_string(),
        record_id: result.id,
    })
}

async fn salesforce_add_note(
    config: &CRMConfig,
    note: &CRMNote,
) -> Result<CRMSyncResult, String> {
    let client = Client::new();

    let body = serde_json::json!({
        "ParentId": note.contact_id,
        "Title": format!("Meeting Notes - {}", note.timestamp),
        "Body": note.content,
    });

    let url = format!("{}/services/data/v59.0/sobjects/Note/", config.instance_url);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Salesforce request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Ok(CRMSyncResult {
            success: false,
            message: format!("Salesforce error ({}): {}", status, body),
            record_id: None,
        });
    }

    let result: SalesforceCreateResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(CRMSyncResult {
        success: result.success.unwrap_or(false),
        message: "Note added to Salesforce".to_string(),
        record_id: result.id,
    })
}

// ─── HubSpot ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct HubSpotCreateResponse {
    id: Option<String>,
}

async fn hubspot_create_contact(
    config: &CRMConfig,
    contact: &CRMContact,
) -> Result<CRMSyncResult, String> {
    let client = Client::new();

    let body = serde_json::json!({
        "properties": {
            "firstname": contact.first_name,
            "lastname": contact.last_name,
            "email": contact.email,
            "phone": contact.phone.as_deref().unwrap_or(""),
            "company": contact.company.as_deref().unwrap_or(""),
        }
    });

    let response = client
        .post("https://api.hubapi.com/crm/v3/objects/contacts")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HubSpot request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Ok(CRMSyncResult {
            success: false,
            message: format!("HubSpot error ({}): {}", status, body),
            record_id: None,
        });
    }

    let result: HubSpotCreateResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(CRMSyncResult {
        success: true,
        message: "Contact created in HubSpot".to_string(),
        record_id: result.id,
    })
}

async fn hubspot_add_note(
    config: &CRMConfig,
    note: &CRMNote,
) -> Result<CRMSyncResult, String> {
    let client = Client::new();

    // Create a note (engagement) in HubSpot
    let body = serde_json::json!({
        "properties": {
            "hs_timestamp": note.timestamp,
            "hs_note_body": note.content,
        },
        "associations": [{
            "to": { "id": note.contact_id },
            "types": [{
                "associationCategory": "HUBSPOT_DEFINED",
                "associationTypeId": 202
            }]
        }]
    });

    let response = client
        .post("https://api.hubapi.com/crm/v3/objects/notes")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HubSpot request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Ok(CRMSyncResult {
            success: false,
            message: format!("HubSpot error ({}): {}", status, body),
            record_id: None,
        });
    }

    let result: HubSpotCreateResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(CRMSyncResult {
        success: true,
        message: "Note added to HubSpot".to_string(),
        record_id: result.id,
    })
}

// ─── Tauri Commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_crm_config(crm: tauri::State<'_, CRMState>) -> CRMConfig {
    crm.lock().clone()
}

#[tauri::command]
pub fn update_crm_config(
    crm: tauri::State<'_, CRMState>,
    config: CRMConfig,
) -> Result<(), String> {
    *crm.lock() = config;
    Ok(())
}

#[tauri::command]
pub async fn crm_sync_contact(
    crm: tauri::State<'_, CRMState>,
    contact: CRMContact,
) -> Result<CRMSyncResult, String> {
    let config = crm.lock().clone();

    match config.provider {
        CRMProvider::Salesforce => salesforce_create_contact(&config, &contact).await,
        CRMProvider::HubSpot => hubspot_create_contact(&config, &contact).await,
        CRMProvider::None => Ok(CRMSyncResult {
            success: false,
            message: "No CRM provider configured".to_string(),
            record_id: None,
        }),
    }
}

#[tauri::command]
pub async fn crm_sync_notes(
    crm: tauri::State<'_, CRMState>,
    note: CRMNote,
) -> Result<CRMSyncResult, String> {
    let config = crm.lock().clone();

    match config.provider {
        CRMProvider::Salesforce => salesforce_add_note(&config, &note).await,
        CRMProvider::HubSpot => hubspot_add_note(&config, &note).await,
        CRMProvider::None => Ok(CRMSyncResult {
            success: false,
            message: "No CRM provider configured".to_string(),
            record_id: None,
        }),
    }
}

#[tauri::command]
pub fn get_crm_providers() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "name": "Salesforce",
            "id": "Salesforce",
            "description": "Sync contacts and meeting notes to Salesforce CRM",
            "requiresInstanceUrl": true
        }),
        serde_json::json!({
            "name": "HubSpot",
            "id": "HubSpot",
            "description": "Sync contacts and meeting notes to HubSpot CRM",
            "requiresInstanceUrl": false
        }),
    ]
}
