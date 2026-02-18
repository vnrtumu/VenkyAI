use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

use super::manager::Session;

pub struct Storage {
    conn: Connection,
}

type StorageState = Arc<Mutex<Storage>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub template: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub summary: Option<String>,
}

impl Storage {
    pub fn new(db_path: &Path) -> Result<Self, String> {
        let conn =
            Connection::open(db_path).map_err(|e| format!("Failed to open database: {}", e))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT,
                summary TEXT,
                transcript_json TEXT
            );

            CREATE TABLE IF NOT EXISTS prompt_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                template TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'general'
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| format!("Failed to create tables: {}", e))?;

        // Insert default prompt templates if none exist
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompt_templates", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        if count == 0 {
            let defaults = vec![
                ("Sales Call", "You're helping during a sales call. Focus on: identifying pain points, suggesting product features that match needs, and crafting compelling value propositions.", "sales"),
                ("Job Interview", "You're helping during a job interview. Focus on: structured answers using STAR method, highlighting relevant experience, asking insightful questions.", "interview"),
                ("Technical Discussion", "You're helping during a technical discussion. Focus on: clear explanations, suggesting best practices, identifying potential issues, and providing code snippets when helpful.", "technical"),
                ("Presentation", "You're helping during a presentation. Focus on: keeping on track, suggesting transition phrases, anticipating audience questions, and highlighting key data points.", "presentation"),
            ];

            for (name, template, category) in defaults {
                conn.execute(
                    "INSERT INTO prompt_templates (id, name, template, category) VALUES (?1, ?2, ?3, ?4)",
                    params![uuid::Uuid::new_v4().to_string(), name, template, category],
                ).ok();
            }
        }

        Ok(Self { conn })
    }

    pub fn save_session(&self, session: &Session) -> Result<(), String> {
        let transcript_json =
            serde_json::to_string(&session.transcript).unwrap_or_else(|_| "[]".to_string());

        self.conn
            .execute(
                "INSERT OR REPLACE INTO sessions (id, title, start_time, end_time, summary, transcript_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    session.id,
                    session.title,
                    session.start_time,
                    session.end_time,
                    session.summary,
                    transcript_json,
                ],
            )
            .map_err(|e| format!("Failed to save session: {}", e))?;

        Ok(())
    }
}

#[tauri::command]
pub fn get_all_sessions(storage: tauri::State<'_, StorageState>) -> Result<Vec<SessionSummary>, String> {
    let s = storage.lock();
    let mut stmt = s
        .conn
        .prepare("SELECT id, title, start_time, end_time, summary FROM sessions ORDER BY start_time DESC")
        .map_err(|e| format!("Query error: {}", e))?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(SessionSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                summary: row.get(4)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

#[tauri::command]
pub fn get_prompt_templates(
    storage: tauri::State<'_, StorageState>,
) -> Result<Vec<PromptTemplate>, String> {
    let s = storage.lock();
    let mut stmt = s
        .conn
        .prepare("SELECT id, name, template, category FROM prompt_templates ORDER BY name")
        .map_err(|e| format!("Query error: {}", e))?;

    let templates = stmt
        .query_map([], |row| {
            Ok(PromptTemplate {
                id: row.get(0)?,
                name: row.get(1)?,
                template: row.get(2)?,
                category: row.get(3)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(templates)
}

#[tauri::command]
pub fn save_prompt_template(
    storage: tauri::State<'_, StorageState>,
    name: String,
    template: String,
    category: String,
) -> Result<PromptTemplate, String> {
    let s = storage.lock();
    let id = uuid::Uuid::new_v4().to_string();

    s.conn
        .execute(
            "INSERT INTO prompt_templates (id, name, template, category) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, template, category],
        )
        .map_err(|e| format!("Failed to save template: {}", e))?;

    Ok(PromptTemplate {
        id,
        name,
        template,
        category,
    })
}

#[tauri::command]
pub fn delete_prompt_template(
    storage: tauri::State<'_, StorageState>,
    id: String,
) -> Result<(), String> {
    let s = storage.lock();
    s.conn
        .execute("DELETE FROM prompt_templates WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete template: {}", e))?;
    Ok(())
}
