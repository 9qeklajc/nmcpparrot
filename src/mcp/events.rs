use super::types::*;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct EventsManager {
    events: RwLock<HashMap<String, Event>>,
    storage_path: String,
}

impl EventsManager {
    pub fn new(storage_path: String) -> Self {
        let mut manager = Self {
            events: RwLock::new(HashMap::new()),
            storage_path,
        };
        let _ = manager.load_from_disk();
        manager
    }

    pub async fn add_event(&self, request: AddEventRequest) -> Result<Event, String> {
        let now = chrono::Utc::now();

        let start_time = if let Some(start_str) = request.start_time {
            Some(
                chrono::DateTime::parse_from_rfc3339(&start_str)
                    .map_err(|e| format!("Invalid start_time format: {}", e))?
                    .with_timezone(&chrono::Utc),
            )
        } else {
            None
        };

        let end_time = if let Some(end_str) = request.end_time {
            Some(
                chrono::DateTime::parse_from_rfc3339(&end_str)
                    .map_err(|e| format!("Invalid end_time format: {}", e))?
                    .with_timezone(&chrono::Utc),
            )
        } else {
            None
        };

        let event = Event {
            id: Uuid::new_v4().to_string(),
            title: request.title,
            description: request.description,
            event_type: request.event_type,
            tags: request.tags.unwrap_or_default(),
            created_at: now,
            start_time,
            end_time,
            metadata: request.metadata.unwrap_or_default(),
        };

        {
            let mut events = self.events.write().await;
            events.insert(event.id.clone(), event.clone());
        }

        self.save_to_disk().await?;
        Ok(event)
    }

    pub async fn list_events(&self, request: ListEventsRequest) -> Result<Vec<Event>, String> {
        let events = self.events.read().await;
        let mut filtered_events: Vec<Event> = events
            .values()
            .filter(|event| {
                let type_match = if let Some(event_type) = &request.event_type {
                    &event.event_type == event_type
                } else {
                    true
                };

                let tag_match = if let Some(tag) = &request.tag {
                    event.tags.contains(tag)
                } else {
                    true
                };

                type_match && tag_match
            })
            .cloned()
            .collect();

        let sort_order = request.sort.as_deref().unwrap_or("newest");
        match sort_order {
            "oldest" => filtered_events.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
            "start_time" => filtered_events.sort_by(|a, b| match (a.start_time, b.start_time) {
                (Some(a_time), Some(b_time)) => a_time.cmp(&b_time),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.created_at.cmp(&b.created_at),
            }),
            _ => filtered_events.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        }

        if let Some(limit) = request.limit {
            filtered_events.truncate(limit as usize);
        }

        Ok(filtered_events)
    }

    pub async fn search_events(&self, request: SearchEventsRequest) -> Result<Vec<Event>, String> {
        let events = self.events.read().await;
        let query_lower = request.query.to_lowercase();

        let mut matching_events: Vec<Event> = events
            .values()
            .filter(|event| {
                let title_match = event.title.to_lowercase().contains(&query_lower);
                let desc_match = event
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query_lower))
                    .unwrap_or(false);

                let content_match = title_match || desc_match;

                let type_match = if let Some(event_type) = &request.event_type {
                    &event.event_type == event_type
                } else {
                    true
                };

                let tag_match = if let Some(tag) = &request.tag {
                    event.tags.contains(tag)
                } else {
                    true
                };

                content_match && type_match && tag_match
            })
            .cloned()
            .collect();

        matching_events.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = request.limit {
            matching_events.truncate(limit as usize);
        }

        Ok(matching_events)
    }

    pub async fn delete_event(&self, request: DeleteEventRequest) -> Result<bool, String> {
        let mut events = self.events.write().await;
        let existed = events.remove(&request.id).is_some();
        drop(events);

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
            .map_err(|e| format!("Failed to read events file: {}", e))?;

        if content.trim().is_empty() {
            return Ok(());
        }

        let events: HashMap<String, Event> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse events file: {}", e))?;

        *self.events.get_mut() = events;
        Ok(())
    }

    async fn save_to_disk(&self) -> Result<(), String> {
        let events = self.events.read().await;
        let content = serde_json::to_string_pretty(&*events)
            .map_err(|e| format!("Failed to serialize events: {}", e))?;

        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create storage directory: {}", e))?;
        }

        fs::write(&self.storage_path, content)
            .map_err(|e| format!("Failed to write events file: {}", e))?;

        Ok(())
    }
}
