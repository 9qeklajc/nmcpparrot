use crate::process_management;
use nostr_sdk::prelude::*;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Runs a shell command each time it receives a direct message
pub async fn run_command_on_message(
    client: &Client,
    our_pubkey: &PublicKey,
    sender_pubkey: &PublicKey,
    shell_command: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Shared state for the current child process
    let process_handle: process_management::ChildHandle = Arc::new(Mutex::new(None));
    let cmd = shell_command.to_string();

    // Build a callback that owns a clone of our shared state + command string
    let callback = {
        let handle_cloned = process_handle.clone();
        move |decrypted_message: String| {
            let handle = handle_cloned.clone();
            let cmd = cmd.clone();
            async move {
                handle_message(&handle, &cmd, decrypted_message).await;
                false // Never returns
            }
        }
    };

    // We wrap the callback in a Mutex
    let callback_arc = Arc::new(Mutex::new(callback));

    // Hand off to the listener
    listen_for_messages(client, our_pubkey, sender_pubkey, callback_arc).await?;
    Ok(())
}

/// This small message handler performs the “kill old, spawn new, store new” logic in one place.
async fn handle_message(handle: &process_management::ChildHandle, cmd: &str, msg: String) {
    let mut guard = handle.lock().await;
    process_management::kill_existing(&mut guard).await;

    let bytes = msg.into_bytes();
    match process_management::spawn_and_pipe(cmd, bytes) {
        Ok(child) => *guard = Some(child),
        Err(e) => {
            log::error!("Error spawning '{}': {}", cmd, e);
            *guard = None;
        }
    }
}

/// Listens for Nostr messages (NIP-17 DMs) from a specific sender and calls a callback
/// with the decrypted message content.
pub async fn listen_for_messages<F, Fut>(
    client: &Client,
    our_pubkey: &PublicKey,
    sender_pubkey: &PublicKey,
    callback: Arc<Mutex<F>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    // Callback takes a String, returns a Future resolving to (), and is Send + Sync + 'static
    F: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = bool> + Send + 'static,
{
    let subscription = Filter::new()
        .kind(Kind::GiftWrap)
        .pubkey(*our_pubkey)
        .limit(0);

    log::info!("Subscribing to GiftWrap events for pubkey: {}", our_pubkey);
    log::info!("Expected sender pubkey: {}", sender_pubkey);
    client.subscribe(subscription, None).await?;

    let callback_clone = callback.clone();
    client
        .handle_notifications(move |notification| {
            let callback_clone = callback_clone.clone();
            let sender_pubkey = *sender_pubkey;
            async move {
                let event = match notification {
                    RelayPoolNotification::Event { event, .. } => {
                        log::debug!("Received event kind {} from {}", event.kind, event.pubkey);
                        event
                    }
                    _ => {
                        log::debug!("Non-event notification");
                        return Ok(false);
                    }
                };

                if event.kind != Kind::GiftWrap {
                    return Ok(false);
                }

                log::debug!("Processing GiftWrap event");
                match client.unwrap_gift_wrap(&event).await {
                    Ok(UnwrappedGift { rumor, sender }) => {
                        log::debug!("Unwrapped gift from {} with kind {}", sender, rumor.kind);

                        if sender == sender_pubkey && rumor.kind == Kind::PrivateDirectMessage {
                            log::info!("Received DM from target sender: {}", rumor.content);
                            let guard = callback_clone.lock().await;
                            return Ok(guard(rumor.content).await);
                        } else {
                            log::debug!(
                                "Ignoring message from {} (expected {})",
                                sender,
                                sender_pubkey
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to unwrap gift wrap: {}", e);
                    }
                }

                Ok(false)
            }
        })
        .await?;

    Ok(())
}

/// Waits for a message from a specific user to our pubkey, and returns one once received
pub async fn wait_for_message(
    client: &Client,
    our_pubkey: &PublicKey,
    from_user: &PublicKey,
) -> Result<String, Box<dyn std::error::Error>> {
    let message_mutex = Arc::new(Mutex::new(None));

    let message_callback = {
        let message_mutex = Arc::clone(&message_mutex);
        move |message: String| {
            let message_mutex = Arc::clone(&message_mutex);
            async move {
                let mut message_guard = message_mutex.lock().await;
                *message_guard = Some(message);
                true // Returns as soon as we receive the first message
            }
        }
    };

    listen_for_messages(
        client,
        our_pubkey,
        from_user,
        Arc::new(Mutex::new(message_callback)),
    )
    .await?;

    let result = message_mutex
        .lock()
        .await
        .take()
        .ok_or_else(|| std::io::Error::other("No message found"))?;
    Ok(result)
}
