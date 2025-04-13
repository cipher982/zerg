use wasm_bindgen_futures;
use web_sys;
use crate::messages::{Message, Command};
use crate::models::{ApiThread, ApiAgent, ApiThreadMessage};
use crate::state::{APP_STATE, dispatch_global_message};
use crate::network::api_client::ApiClient;
use serde_json;
use std::rc::Rc;
use std::cell::RefCell;
use serde_json::json;

pub fn execute_fetch_command(cmd: Command) {
    match cmd {
        Command::FetchThreads(agent_id) => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_threads(Some(agent_id)).await {
                    Ok(response) => {
                        match serde_json::from_str::<Vec<ApiThread>>(&response) {
                            Ok(threads) => dispatch_global_message(Message::ThreadsLoaded(threads)),
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse threads: {:?}", e).into())
                        }
                    },
                    Err(e) => web_sys::console::error_1(&format!("Failed to fetch threads: {:?}", e).into())
                }
            });
        },
        Command::FetchThreadMessages(thread_id) => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_thread_messages(thread_id, 0, 100).await {
                    Ok(messages) => match serde_json::from_str::<Vec<ApiThreadMessage>>(&messages) {
                        Ok(messages) => dispatch_global_message(Message::ThreadMessagesLoaded(thread_id, messages)),
                        Err(e) => web_sys::console::error_1(&format!("Failed to parse messages: {:?}", e).into())
                    },
                    Err(e) => web_sys::console::error_1(&format!("Failed to fetch messages: {:?}", e).into())
                }
            });
        },
        Command::LoadAgentInfo(agent_id) => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_agent(agent_id).await {
                    Ok(response) => {
                        match serde_json::from_str::<ApiAgent>(&response) {
                            Ok(agent) => dispatch_global_message(Message::AgentInfoLoaded(Box::new(agent))),
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse agent: {:?}", e).into())
                        }
                    },
                    Err(e) => web_sys::console::error_1(&format!("Failed to fetch agent: {:?}", e).into())
                }
            });
        },
        _ => web_sys::console::warn_1(&"Unexpected command type in execute_fetch_command".into())
    }
}

pub fn execute_thread_command(cmd: Command) {
    match cmd {
        Command::CreateThread { agent_id, title } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::create_thread(agent_id, &title).await {
                    Ok(response) => {
                        match serde_json::from_str::<ApiThread>(&response) {
                            Ok(thread) => dispatch_global_message(Message::ThreadCreated(thread)),
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse created thread: {:?}", e).into())
                        }
                    },
                    Err(e) => web_sys::console::error_1(&format!("Failed to create thread: {:?}", e).into())
                }
            });
        },
        Command::SendThreadMessage { thread_id, content, client_id } => {
            web_sys::console::log_1(&format!("Executor: Handling Command::SendThreadMessage for thread {}: '{}'", thread_id, content).into());
            let client_id_str = client_id.map(|id| id.to_string()).unwrap_or_default();
            
            // Construct the request body for the /run endpoint
            let request_body = json!({ "content": content }).to_string();
            
            // Call the /run endpoint directly, sending the content
            wasm_bindgen_futures::spawn_local(async move {
                // Note: We now only call run_thread, which creates the message and processes it
                match ApiClient::run_thread(thread_id, &request_body).await {
                    Ok(response) => {
                        // The HTTP response is just a confirmation (202 Accepted)
                        // The actual chat response arrives via WebSocket stream
                        // We can log success, but the original ThreadMessageSent isn't really applicable
                        // as the message isn't fully 'sent' until the stream completes.
                        web_sys::console::log_1(&format!("Executor: run_thread POST succeeded for thread {}: {}", thread_id, response).into());
                        // Optionally, dispatch a new message indicating processing started, if needed for UI
                        // dispatch_global_message(Message::ThreadProcessingStarted(thread_id, client_id_str)); 
                    },
                    Err(e) => {
                        // Log the error and dispatch failure
                        web_sys::console::error_1(&format!("Executor: Failed to run thread {}: {:?}", thread_id, e).into());
                        dispatch_global_message(Message::ThreadMessageFailed(thread_id, client_id_str));
                    }
                }
            });
        },
        Command::RunThread(thread_id) => {
            // This command might now be redundant if SendThreadMessage handles everything.
            // If kept, it should ideally not be called right after SendThreadMessage.
            // Consider removing or repurposing this command.
            web_sys::console::warn_1(&format!("Executor: Handling potentially redundant Command::RunThread for thread {}", thread_id).into());
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::run_thread(thread_id, "{}").await { // Sending empty body as placeholder
                    Ok(response) => {
                         web_sys::console::log_1(&format!("Executor: run_thread (manual) POST succeeded for thread {}: {}", thread_id, response).into());
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Executor: Failed to run thread {} (manual): {:?}", thread_id, e).into());
                    }
                }
            });
        },
        Command::UpdateThreadTitle { thread_id, title } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::update_thread(thread_id, &title).await {
                    Ok(_) => web_sys::console::log_1(&format!("Successfully updated thread title for {}", thread_id).into()),
                    Err(e) => web_sys::console::error_1(&format!("Failed to update thread title: {:?}", e).into())
                }
            });
        },
        _ => web_sys::console::warn_1(&"Unexpected command type in execute_thread_command".into())
    }
}

pub fn execute_network_command(cmd: Command) {
    match cmd {
        Command::NetworkCall { endpoint, method, body, on_success, on_error } => {
            wasm_bindgen_futures::spawn_local(async move {
                let result = match method.as_str() {
                    "GET" => ApiClient::fetch_json(&endpoint, "GET", None).await,
                    "POST" => {
                        if let Some(data) = body {
                            ApiClient::fetch_json(&endpoint, "POST", Some(&data)).await
                        } else {
                            Err("POST request requires body data".into())
                        }
                    },
                    "PUT" => {
                        if let Some(data) = body {
                            ApiClient::fetch_json(&endpoint, "PUT", Some(&data)).await
                        } else {
                            Err("PUT request requires body data".into())
                        }
                    },
                    "DELETE" => ApiClient::fetch_json(&endpoint, "DELETE", None).await,
                    _ => Err(format!("Unsupported HTTP method: {}", method).into()),
                };
                
                match result {
                    Ok(_) => dispatch_global_message(*on_success),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Network call failed: {:?}", e).into());
                        dispatch_global_message(*on_error)
                    }
                }
            });
        },
        Command::UpdateAgent { agent_id, payload, on_success, on_error } => {
            web_sys::console::log_1(&format!("Executor: Updating agent {} with payload: {}", agent_id, payload).into());
            
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::update_agent(agent_id, &payload).await {
                    Ok(response) => {
                        web_sys::console::log_1(&format!("Agent update successful: {}", response).into());
                        dispatch_global_message(*on_success);
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to update agent {}: {:?}", agent_id, e).into());
                        dispatch_global_message(*on_error);
                    }
                }
            });
        },
        _ => web_sys::console::warn_1(&"Unexpected command type in execute_network_command".into())
    }
}

pub fn execute_websocket_command(cmd: Command) {
    match cmd {
        Command::WebSocketAction { action, topic, data: _ } => {
            match action.as_str() {
                "subscribe" => {
                    if let Some(topic_str) = topic {
                        APP_STATE.with(|state| {
                            let state = state.borrow();
                            let topic_manager = state.topic_manager.clone();
                            
                            // Clone topic_str before moving into closure
                            let topic_for_handler = topic_str.clone();
                            
                            // Clone topic_str for potential error logging
                            let topic_for_error_log = topic_str.clone();
                            
                            wasm_bindgen_futures::spawn_local(async move {
                                // Create a simple handler that logs the received messages
                                let handler = Rc::new(RefCell::new(move |json_value: serde_json::Value| {
                                    web_sys::console::log_1(&format!("WS: Received message for topic {}: {:?}", topic_for_handler, json_value).into());
                                }));
                                
                                // Subscribe using the topic manager
                                if let Err(e) = topic_manager.borrow_mut().subscribe(topic_str, handler) {
                                    web_sys::console::error_1(&format!("Failed to subscribe to topic {}: {:?}", topic_for_error_log, e).into());
                                } else {
                                    // No need to clone here, topic_str wasn't moved in this path if subscribe succeeded
                                    web_sys::console::log_1(&format!("Successfully subscribed to topic {}", topic_for_error_log).into()); // Use the cloned value here too for consistency, although original topic_str *could* be used if subscribe didn't move.
                                }
                            });
                        });
                    } else {
                        web_sys::console::error_1(&"Cannot subscribe without a topic".into());
                    }
                },
                _ => web_sys::console::warn_1(&format!("Unsupported WebSocket action: {}", action).into())
            }
        },
        _ => web_sys::console::warn_1(&"Unexpected command type in execute_websocket_command".into())
    }
} 