use super::chat::Chat;
use super::events::EventsManager;
use super::notes::NotesManager;
use super::progress_enforcer::ProgressTracker;
use super::types::*;
use super::validation::{extract_error_context, sanitize_json_parameters};
use nostr_sdk::prelude::*;
use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, Error as RmcpError, ServerHandler,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct EnhancedMcpServer {
    chat: Chat,
    notes: Arc<NotesManager>,
    events: Arc<EventsManager>,
    progress_tracker: Arc<ProgressTracker>,
}

#[tool(tool_box)]
impl EnhancedMcpServer {
    pub fn new(
        client: Client,
        progress_client: Option<Client>,
        our_pubkey: PublicKey,
        target_pubkey: PublicKey,
        data_dir: Option<String>,
    ) -> Self {
        let data_dir = data_dir.unwrap_or_else(|| "data".to_string());

        Self {
            chat: Chat::new(client, progress_client, our_pubkey, target_pubkey),
            notes: Arc::new(NotesManager::new(format!("{}/notes.json", data_dir))),
            events: Arc::new(EventsManager::new(format!("{}/events.json", data_dir))),
            progress_tracker: Arc::new(ProgressTracker::new()),
        }
    }

    /// Helper function to safely parse JSON parameters with error recovery
    #[allow(dead_code)] // Future use for JSON parameter recovery
    fn safe_parse_params<T>(&self, params_str: &str) -> Result<T, RmcpError>
    where
        T: serde::de::DeserializeOwned,
    {
        // First try direct parsing
        match serde_json::from_str::<T>(params_str) {
            Ok(parsed) => Ok(parsed),
            Err(original_error) => {
                // If that fails, try to sanitize the JSON
                match sanitize_json_parameters(params_str) {
                    Ok(sanitized) => match serde_json::from_str::<T>(&sanitized) {
                        Ok(parsed) => {
                            log::warn!("Successfully recovered from malformed JSON parameters");
                            Ok(parsed)
                        }
                        Err(sanitize_error) => {
                            let context = extract_error_context(&sanitize_error.to_string());
                            Err(RmcpError::internal_error(
                                format!(
                                    "Parameter parsing failed: {}. Original error: {}",
                                    context, original_error
                                ),
                                None,
                            ))
                        }
                    },
                    Err(sanitize_error) => Err(RmcpError::internal_error(
                        format!(
                            "Could not interpret tool use parameters: {}. {}",
                            original_error, sanitize_error
                        ),
                        None,
                    )),
                }
            }
        }
    }

    #[tool(description = "Send a message to the user")]
    async fn send(
        &self,
        #[tool(aggr)] request: SendMessageRequest,
    ) -> Result<CallToolResult, RmcpError> {
        self.chat.send(request).await
    }

    #[tool(description = "Send a progress/debug message to the user via the progress identity")]
    async fn progress(
        &self,
        #[tool(aggr)] request: ProgressMessageRequest,
    ) -> Result<CallToolResult, RmcpError> {
        self.chat.progress(request).await
    }

    #[tool(description = "Listen and wait for the user's next message")]
    async fn wait(&self) -> Result<CallToolResult, RmcpError> {
        self.chat.wait().await
    }

    #[tool(description = "Add a new note with content, optional tags, and metadata")]
    async fn addnote(
        &self,
        #[tool(aggr)] request: AddNoteRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Adding new note...".to_string(),
            })
            .await;

        match self.notes.add_note(request).await {
            Ok(note) => {
                let message = format!(
                    "Note added successfully!\n\nID: {}\nContent: {}\nTags: {}\nCreated: {}",
                    note.id,
                    note.content,
                    note.tags.join(", "),
                    note.created_at.format("%Y-%m-%d %H:%M UTC")
                );

                let _ = self.chat.send(SendMessageRequest { message }).await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Note added with ID: {}",
                    note.id
                ))]))
            }
            Err(e) => {
                let error_msg = format!("Failed to add note: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_msg.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }

    #[tool(description = "List notes with optional filtering by tag, limit, and sort order")]
    async fn listnotes(
        &self,
        #[tool(aggr)] request: ListNotesRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Retrieving notes...".to_string(),
            })
            .await;

        match self.notes.list_notes(request).await {
            Ok(notes) => {
                let message = if notes.is_empty() {
                    "📝 No notes found.".to_string()
                } else {
                    let notes_text = notes
                        .iter()
                        .map(|note| {
                            format!(
                                "• **{}** ({})\n  Tags: {}\n  Created: {}\n",
                                &note.id[..8],
                                note.content.chars().take(50).collect::<String>()
                                    + if note.content.len() > 50 { "..." } else { "" },
                                if note.tags.is_empty() {
                                    "none".to_string()
                                } else {
                                    note.tags.join(", ")
                                },
                                note.created_at.format("%Y-%m-%d %H:%M UTC")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    format!("📝 Found {} note(s):\n\n{}", notes.len(), notes_text)
                };

                let _ = self.chat.send(SendMessageRequest { message }).await;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Listed {} notes",
                    notes.len()
                ))]))
            }
            Err(e) => {
                let error_msg = format!("❌ Failed to list notes: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_msg.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }

    #[tool(description = "Search notes by content with optional tag filtering and result limit")]
    async fn searchnotes(
        &self,
        #[tool(aggr)] request: SearchNotesRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Searching notes for: '{}'...", request.query),
            })
            .await;

        match self.notes.search_notes(request).await {
            Ok(notes) => {
                let message = if notes.is_empty() {
                    "🔍 No matching notes found.".to_string()
                } else {
                    let notes_text = notes
                        .iter()
                        .map(|note| {
                            format!(
                                "• **{}**\n  {}\n  Tags: {}\n  Created: {}\n",
                                &note.id[..8],
                                note.content,
                                if note.tags.is_empty() {
                                    "none".to_string()
                                } else {
                                    note.tags.join(", ")
                                },
                                note.created_at.format("%Y-%m-%d %H:%M UTC")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    format!(
                        "🔍 Found {} matching note(s):\n\n{}",
                        notes.len(),
                        notes_text
                    )
                };

                let _ = self.chat.send(SendMessageRequest { message }).await;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Found {} matching notes",
                    notes.len()
                ))]))
            }
            Err(e) => {
                let error_msg = format!("❌ Failed to search notes: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_msg.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }

    #[tool(description = "Delete a note by its ID")]
    async fn deletenote(
        &self,
        #[tool(aggr)] request: DeleteNoteRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Deleting note {}...", request.id),
            })
            .await;

        match self.notes.delete_note(request).await {
            Ok(existed) => {
                let message = if existed {
                    "🗑️ Note deleted successfully!".to_string()
                } else {
                    "❌ Note not found.".to_string()
                };

                let _ = self.chat.send(SendMessageRequest { message }).await;
                Ok(CallToolResult::success(vec![Content::text(
                    if existed {
                        "Note deleted"
                    } else {
                        "Note not found"
                    }
                    .to_string(),
                )]))
            }
            Err(e) => {
                let error_msg = format!("❌ Failed to delete note: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_msg.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }

    #[tool(
        description = "Add a new event with title, description, type, optional times, tags, and metadata"
    )]
    async fn addevent(
        &self,
        #[tool(aggr)] request: AddEventRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Adding new event...".to_string(),
            })
            .await;

        match self.events.add_event(request).await {
            Ok(event) => {
                let time_info = match (event.start_time, event.end_time) {
                    (Some(start), Some(end)) => format!(
                        "\nStart: {}\nEnd: {}",
                        start.format("%Y-%m-%d %H:%M UTC"),
                        end.format("%Y-%m-%d %H:%M UTC")
                    ),
                    (Some(start), None) => {
                        format!("\nStart: {}", start.format("%Y-%m-%d %H:%M UTC"))
                    }
                    (None, Some(end)) => format!("\nEnd: {}", end.format("%Y-%m-%d %H:%M UTC")),
                    (None, None) => "".to_string(),
                };

                let message = format!(
                    "📅 Event added successfully!\n\nID: {}\nTitle: {}\nType: {}\nTags: {}\nCreated: {}{}",
                    event.id,
                    event.title,
                    event.event_type,
                    if event.tags.is_empty() { "none".to_string() } else { event.tags.join(", ") },
                    event.created_at.format("%Y-%m-%d %H:%M UTC"),
                    time_info
                );

                let _ = self.chat.send(SendMessageRequest { message }).await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Event added with ID: {}",
                    event.id
                ))]))
            }
            Err(e) => {
                let error_msg = format!("❌ Failed to add event: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_msg.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }

    #[tool(description = "List events with optional filtering by type, tag, limit, and sort order")]
    async fn listevents(
        &self,
        #[tool(aggr)] request: ListEventsRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Retrieving events...".to_string(),
            })
            .await;

        match self.events.list_events(request).await {
            Ok(events) => {
                let message = if events.is_empty() {
                    "📅 No events found.".to_string()
                } else {
                    let events_text = events
                        .iter()
                        .map(|event| {
                            let time_info = match event.start_time {
                                Some(start) => format!(" | {}", start.format("%m/%d %H:%M")),
                                None => "".to_string(),
                            };

                            format!(
                                "• **{}** - {} ({}){}\n  Tags: {}\n",
                                &event.id[..8],
                                event.title,
                                event.event_type,
                                time_info,
                                if event.tags.is_empty() {
                                    "none".to_string()
                                } else {
                                    event.tags.join(", ")
                                }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    format!("📅 Found {} event(s):\n\n{}", events.len(), events_text)
                };

                let _ = self.chat.send(SendMessageRequest { message }).await;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Listed {} events",
                    events.len()
                ))]))
            }
            Err(e) => {
                let error_msg = format!("❌ Failed to list events: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_msg.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }

    #[tool(
        description = "Search events by title and description with optional type and tag filtering"
    )]
    async fn searchevents(
        &self,
        #[tool(aggr)] request: SearchEventsRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Searching events for: '{}'...", request.query),
            })
            .await;

        match self.events.search_events(request).await {
            Ok(events) => {
                let message = if events.is_empty() {
                    "🔍 No matching events found.".to_string()
                } else {
                    let events_text = events
                        .iter()
                        .map(|event| {
                            let time_info = match (event.start_time, event.end_time) {
                                (Some(start), Some(end)) => format!(
                                    "\n  Time: {} - {}",
                                    start.format("%Y-%m-%d %H:%M"),
                                    end.format("%Y-%m-%d %H:%M")
                                ),
                                (Some(start), None) => {
                                    format!("\n  Start: {}", start.format("%Y-%m-%d %H:%M"))
                                }
                                (None, Some(end)) => {
                                    format!("\n  End: {}", end.format("%Y-%m-%d %H:%M"))
                                }
                                (None, None) => "".to_string(),
                            };

                            format!(
                                "• **{}** - {} ({})\n  Tags: {}{}",
                                &event.id[..8],
                                event.title,
                                event.event_type,
                                if event.tags.is_empty() {
                                    "none".to_string()
                                } else {
                                    event.tags.join(", ")
                                },
                                time_info
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n");

                    format!(
                        "🔍 Found {} matching event(s):\n\n{}",
                        events.len(),
                        events_text
                    )
                };

                let _ = self.chat.send(SendMessageRequest { message }).await;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Found {} matching events",
                    events.len()
                ))]))
            }
            Err(e) => {
                let error_msg = format!("❌ Failed to search events: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_msg.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }

    #[tool(description = "Delete an event by its ID")]
    async fn deleteevent(
        &self,
        #[tool(aggr)] request: DeleteEventRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Deleting event {}...", request.id),
            })
            .await;

        match self.events.delete_event(request).await {
            Ok(existed) => {
                let message = if existed {
                    "🗑️ Event deleted successfully!".to_string()
                } else {
                    "❌ Event not found.".to_string()
                };

                let _ = self.chat.send(SendMessageRequest { message }).await;
                Ok(CallToolResult::success(vec![Content::text(
                    if existed {
                        "Event deleted"
                    } else {
                        "Event not found"
                    }
                    .to_string(),
                )]))
            }
            Err(e) => {
                let error_msg = format!("❌ Failed to delete event: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_msg.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for EnhancedMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(format!("This enhanced server provides comprehensive tools for Nostr chat, note management, and event tracking.\n\nABSOLUTELY MANDATORY FOR EVERY USER MESSAGE:\n\n1. IMMEDIATE PROGRESS RESPONSE: The INSTANT you receive a user message, you MUST send a progress update\n   Example: {{\"tool\": \"progress\", \"arguments\": {{\"message\": \"I'm processing your request...\"}}}}\n\n2. PERFORM OPERATIONS: Execute the requested note/event operations\n\n3. MANDATORY FINAL SEND: You MUST ALWAYS end with a 'send' tool call - NO EXCEPTIONS\n   Example: {{\"tool\": \"send\", \"arguments\": {{\"message\": \"Operation completed successfully\"}}}}\n\nCRITICAL: EVERY conversation turn MUST follow this pattern:\n   wait -> progress -> [note/event operations] -> send\n\nUSER VISIBILITY RULES:\n- Users can ONLY see messages sent via 'send' and 'progress' tools\n- Users CANNOT see your thinking, reasoning, or stdout output\n- If you don't use 'send', the user sees NOTHING\n- If you don't use 'progress', users think you're not working\n\nFORBIDDEN BEHAVIORS:\n- Never end a turn without 'send'\n- Never start work without 'progress'\n- Never perform note/event operations without progress updates\n- Never assume the user knows what you're doing\n- Never send follow-up messages asking if user needs help\n- Never ask \"Is there anything else I can help you with?\"\n- Never send unsolicited check-in messages after task completion\n\n{}\n\nCRITICAL PARAMETER RULES:\n1) ALL tool parameters MUST be valid JSON objects\n2) String values MUST be properly quoted\n3) Use double quotes, not single quotes\n4) Ensure proper escaping of special characters\n5) NO trailing commas or extra characters\n\nCOMMON PARAMETER ERRORS TO AVOID:\n- Unquoted strings: {{message: hello}} WRONG -> {{\"message\": \"hello\"}} CORRECT\n- Single quotes: {{'message': 'hello'}} WRONG -> {{\"message\": \"hello\"}} CORRECT\n- Trailing chars: {{\"message\": \"hello\"}}extra WRONG -> {{\"message\": \"hello\"}} CORRECT\n- Missing commas: {{\"a\": \"1\" \"b\": \"2\"}} WRONG -> {{\"a\": \"1\", \"b\": \"2\"}} CORRECT\n\nERROR RECOVERY: If you receive parameter errors, retry with simpler, properly formatted JSON.\n\nFAILURE TO FOLLOW THIS PATTERN WILL BREAK THE SYSTEM\n\nAvailable capabilities: Chat (send, progress, wait), Notes (addnote, listnotes, searchnotes, deletenote), Events (addevent, listevents, searchevents, deleteevent).", 
                self.progress_tracker.create_comprehensive_instructions())),
        }
    }
}
