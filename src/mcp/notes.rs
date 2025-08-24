use super::types::*;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct NotesManager {
    notes: RwLock<HashMap<String, Note>>,
    storage_path: String,
}

impl NotesManager {
    pub fn new(storage_path: String) -> Self {
        let mut manager = Self {
            notes: RwLock::new(HashMap::new()),
            storage_path,
        };
        let _ = manager.load_from_disk();
        manager
    }

    pub async fn add_note(&self, request: AddNoteRequest) -> Result<Note, String> {
        let now = chrono::Utc::now();
        let note = Note {
            id: Uuid::new_v4().to_string(),
            content: request.content,
            tags: request.tags.unwrap_or_default(),
            created_at: now,
            updated_at: now,
            metadata: request.metadata.unwrap_or_default(),
        };

        {
            let mut notes = self.notes.write().await;
            notes.insert(note.id.clone(), note.clone());
        }

        self.save_to_disk().await?;
        Ok(note)
    }

    pub async fn list_notes(&self, request: ListNotesRequest) -> Result<Vec<Note>, String> {
        let notes = self.notes.read().await;
        let mut filtered_notes: Vec<Note> = notes
            .values()
            .filter(|note| {
                if let Some(tag) = &request.tag {
                    note.tags.contains(tag)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        let sort_order = request.sort.as_deref().unwrap_or("newest");
        match sort_order {
            "oldest" => filtered_notes.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
            "updated" => filtered_notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            _ => filtered_notes.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        }

        if let Some(limit) = request.limit {
            filtered_notes.truncate(limit as usize);
        }

        Ok(filtered_notes)
    }

    pub async fn search_notes(&self, request: SearchNotesRequest) -> Result<Vec<Note>, String> {
        let notes = self.notes.read().await;
        let query_lower = request.query.to_lowercase();

        let mut matching_notes: Vec<Note> = notes
            .values()
            .filter(|note| {
                let content_match = note.content.to_lowercase().contains(&query_lower);
                let tag_match = if let Some(tag) = &request.tag {
                    note.tags.contains(tag)
                } else {
                    true
                };
                content_match && tag_match
            })
            .cloned()
            .collect();

        matching_notes.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = request.limit {
            matching_notes.truncate(limit as usize);
        }

        Ok(matching_notes)
    }

    pub async fn delete_note(&self, request: DeleteNoteRequest) -> Result<bool, String> {
        let mut notes = self.notes.write().await;
        let existed = notes.remove(&request.id).is_some();
        drop(notes);

        if existed {
            self.save_to_disk().await?;
        }

        Ok(existed)
    }

    fn load_from_disk(&mut self) -> Result<(), String> {
        if !Path::new(&self.storage_path).exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.storage_path)
            .map_err(|e| format!("Failed to read notes file: {}", e))?;

        if content.trim().is_empty() {
            return Ok(());
        }

        let notes: HashMap<String, Note> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse notes file: {}", e))?;

        *self.notes.get_mut() = notes;
        Ok(())
    }

    async fn save_to_disk(&self) -> Result<(), String> {
        let notes = self.notes.read().await;
        let content = serde_json::to_string_pretty(&*notes)
            .map_err(|e| format!("Failed to serialize notes: {}", e))?;

        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create storage directory: {}", e))?;
        }

        fs::write(&self.storage_path, content)
            .map_err(|e| format!("Failed to write notes file: {}", e))?;

        Ok(())
    }
}
