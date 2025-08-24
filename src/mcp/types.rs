use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use super::chat::{ProgressMessageRequest, SendMessageRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub event_type: String,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddNoteRequest {
    #[schemars(description = "The content of the note")]
    pub content: String,
    #[schemars(description = "Optional tags for categorizing the note")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Optional metadata key-value pairs")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddEventRequest {
    #[schemars(description = "The title of the event")]
    pub title: String,
    #[schemars(description = "Optional description of the event")]
    pub description: Option<String>,
    #[schemars(description = "Type of event (meeting, task, reminder, etc.)")]
    pub event_type: String,
    #[schemars(description = "Optional tags for categorizing the event")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Optional start time in ISO 8601 format")]
    pub start_time: Option<String>,
    #[schemars(description = "Optional end time in ISO 8601 format")]
    pub end_time: Option<String>,
    #[schemars(description = "Optional metadata key-value pairs")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListNotesRequest {
    #[schemars(description = "Optional tag filter - only show notes with this tag")]
    pub tag: Option<String>,
    #[schemars(description = "Optional limit on number of notes to return")]
    pub limit: Option<u32>,
    #[schemars(description = "Sort order: 'newest', 'oldest', or 'updated'")]
    pub sort: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListEventsRequest {
    #[schemars(description = "Optional event type filter")]
    pub event_type: Option<String>,
    #[schemars(description = "Optional tag filter - only show events with this tag")]
    pub tag: Option<String>,
    #[schemars(description = "Optional limit on number of events to return")]
    pub limit: Option<u32>,
    #[schemars(description = "Sort order: 'newest', 'oldest', or 'start_time'")]
    pub sort: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchNotesRequest {
    #[schemars(description = "Search query - searches in note content")]
    pub query: String,
    #[schemars(description = "Optional tag filter")]
    pub tag: Option<String>,
    #[schemars(description = "Optional limit on number of results")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchEventsRequest {
    #[schemars(description = "Search query - searches in title and description")]
    pub query: String,
    #[schemars(description = "Optional event type filter")]
    pub event_type: Option<String>,
    #[schemars(description = "Optional tag filter")]
    pub tag: Option<String>,
    #[schemars(description = "Optional limit on number of results")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteNoteRequest {
    #[schemars(description = "The ID of the note to delete")]
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteEventRequest {
    #[schemars(description = "The ID of the event to delete")]
    pub id: String,
}
