use crate::messages::{Message, Command};
use crate::models::{ApiThread, ApiAgent, ApiThreadMessage};
use crate::models::ApiAgentDetails;
use crate::state::{APP_STATE, dispatch_global_message};
use crate::network::api_client::ApiClient;
use std::rc::Rc;
use std::cell::RefCell;

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
        Command::FetchAgents => {
            // Determine current dashboard scope before starting the async
            // task so we don’t hold a RefCell borrow across await points.
            let scope_str = crate::state::APP_STATE.with(|state_ref| {
                let state = state_ref.borrow();
                state.dashboard_scope.as_str().to_string()
            });

            web_sys::console::log_1(&format!("Executing FetchAgents command (scope={})", scope_str).into());
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_agents_scoped(&scope_str).await {
                    Ok(agents_json) => {
                        match serde_json::from_str::<Vec<ApiAgent>>(&agents_json) {
                            Ok(agents) => {
                                web_sys::console::log_1(&format!("Fetched {} agents from API", agents.len()).into());
                                dispatch_global_message(Message::AgentsRefreshed(agents))
                            },
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse agents: {:?}", e).into())
                        }
                    },
                    Err(e) => web_sys::console::error_1(&format!("Failed to fetch agents: {:?}", e).into())
                }
            });
        },

        // -----------------------------------------------------------
        // Workflow helpers (new)
        // -----------------------------------------------------------

        Command::FetchWorkflows => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_workflows().await {
                    Ok(json_str) => {
                        match serde_json::from_str::<Vec<crate::models::ApiWorkflow>>(&json_str) {
                            Ok(api_wfs) => {
                                let workflows: Vec<crate::models::Workflow> = api_wfs.into_iter().map(|w| w.into()).collect();
                                dispatch_global_message(Message::WorkflowsLoaded(workflows));
                            },
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse workflows: {:?}", e).into()),
                        }
                    }
                    Err(e) => web_sys::console::error_1(&format!("Failed to fetch workflows: {:?}", e).into()),
                }
            });
        },

        // ---------------- Workflow CRUD commands -------------------

        Command::CreateWorkflowApi { name } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::create_workflow(&name).await {
                    Ok(json_str) => {
                        match serde_json::from_str::<crate::models::ApiWorkflow>(&json_str) {
                            Ok(api_wf) => {
                                let wf: crate::models::Workflow = api_wf.into();
                                dispatch_global_message(Message::WorkflowCreated(wf));
                            }
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse created workflow: {:?}", e).into()),
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to create workflow: {:?}", e).into());
                        // Could dispatch error message / toast
                    }
                }
            });
        }
        Command::DeleteWorkflowApi { workflow_id } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::delete_workflow(workflow_id).await {
                    Ok(()) => dispatch_global_message(Message::WorkflowDeleted { workflow_id }),
                    Err(e) => web_sys::console::error_1(&format!("Failed to delete workflow: {:?}", e).into()),
                }
            });
        }
        Command::RenameWorkflowApi { workflow_id, name, description } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::rename_workflow(workflow_id, &name, &description).await {
                    Ok(json_str) => {
                        match serde_json::from_str::<crate::models::ApiWorkflow>(&json_str) {
                            Ok(api_wf) => {
                                let wf: crate::models::Workflow = api_wf.into();
                                dispatch_global_message(Message::WorkflowUpdated(wf));
                            }
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse renamed workflow: {:?}", e).into()),
                        }
                    }
                    Err(e) => web_sys::console::error_1(&format!("Failed to rename workflow: {:?}", e).into()),
                }
            });
        }

        // -----------------------------------------------------------
        // Start workflow execution – call backend and then subscribe
        // -----------------------------------------------------------
        Command::StartWorkflowExecutionApi { workflow_id } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::start_workflow_execution(workflow_id).await {
                    Ok(json_str) => {
                        // Expected response: { "execution_id": 123, "status": "running" }
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            if let Some(exec_id_val) = val.get("execution_id") {
                                if let Some(exec_id_u64) = exec_id_val.as_u64() {
                                    let exec_id = exec_id_u64 as u32;
                                    crate::state::dispatch_global_message(crate::messages::Message::SubscribeWorkflowExecution { execution_id: exec_id });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to start workflow execution: {:?}", e).into());
                        // Optionally: dispatch toast / error message here
                    }
                }
            });
        }

        // -----------------------------------------------------------
        // Workflow Scheduling commands
        // -----------------------------------------------------------
        Command::ScheduleWorkflowApi { workflow_id, cron_expression } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::schedule_workflow(workflow_id, &cron_expression).await {
                    Ok(json_str) => {
                        web_sys::console::log_1(&format!("Workflow {} scheduled successfully: {}", workflow_id, json_str).into());
                        // Could dispatch success message / toast
                        crate::toast::success(&format!("Workflow scheduled with cron: {}", cron_expression));
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to schedule workflow: {:?}", e).into());
                        crate::toast::error("Failed to schedule workflow");
                    }
                }
            });
        }

        Command::UnscheduleWorkflowApi { workflow_id } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::unschedule_workflow(workflow_id).await {
                    Ok(_) => {
                        web_sys::console::log_1(&format!("Workflow {} unscheduled successfully", workflow_id).into());
                        crate::toast::success("Workflow unscheduled successfully");
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to unschedule workflow: {:?}", e).into());
                        crate::toast::error("Failed to unschedule workflow");
                    }
                }
            });
        }

        Command::CheckWorkflowScheduleApi { workflow_id } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_workflow_schedule(workflow_id).await {
                    Ok(json_str) => {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            if let Some(scheduled) = val.get("scheduled").and_then(|v| v.as_bool()) {
                                if scheduled {
                                    if let Some(next_run) = val.get("next_run_time").and_then(|v| v.as_str()) {
                                        web_sys::console::log_1(&format!("Workflow {} is scheduled, next run: {}", workflow_id, next_run).into());
                                    }
                                } else {
                                    web_sys::console::log_1(&format!("Workflow {} is not scheduled", workflow_id).into());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to check workflow schedule: {:?}", e).into());
                    }
                }
            });
        }

        Command::FetchAgentDetails(agent_id) => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_agent_details(agent_id).await {
                    Ok(json_str) => {
                        match serde_json::from_str::<ApiAgentDetails>(&json_str) {
                            Ok(details) => dispatch_global_message(Message::ReceiveAgentDetails(details)),
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse agent details: {:?}", e).into()),
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to fetch agent details: {:?}", e).into());
                        // Could choose to dispatch HideAgentDebugModal or error state
                    }
                }
            });
        },
        Command::FetchAgentRuns(agent_id) => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_agent_runs(agent_id, 20).await {
                    Ok(json_str) => {
                        match serde_json::from_str::<Vec<crate::models::ApiAgentRun>>(&json_str) {
                            Ok(runs) => dispatch_global_message(Message::ReceiveAgentRuns { agent_id, runs }),
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse agent runs: {:?}", e).into()),
                        }
                    },
                    Err(e) => web_sys::console::error_1(&format!("Failed to fetch agent runs: {:?}", e).into()),
                }
            });
        },

        // -----------------------------------------------------------
        // Trigger helpers
        // -----------------------------------------------------------

        Command::FetchTriggers(agent_id) => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::get_triggers(agent_id).await {
                    Ok(json_str) => {
                        match serde_json::from_str::<Vec<crate::models::Trigger>>(&json_str) {
                            Ok(triggers) => dispatch_global_message(crate::messages::Message::TriggersLoaded { agent_id, triggers }),
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse triggers: {:?}", e).into()),
                        }
                    }
                    Err(e) => web_sys::console::error_1(&format!("Failed to fetch triggers: {:?}", e).into()),
                }
            });
        },

        Command::CreateTrigger { payload_json } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::create_trigger(&payload_json).await {
                    Ok(json_str) => {
                        match serde_json::from_str::<crate::models::Trigger>(&json_str) {
                            Ok(trigger) => {
                                let agent_id = trigger.agent_id;
                                dispatch_global_message(crate::messages::Message::TriggerCreated { agent_id, trigger });
                            }
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse new trigger: {:?}", e).into()),
                        }
                    }
                    Err(e) => web_sys::console::error_1(&format!("Failed to create trigger: {:?}", e).into()),
                }
            });
        },

        Command::DeleteTrigger(trigger_id) => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::delete_trigger(trigger_id).await {
                    Ok(()) => dispatch_global_message(crate::messages::Message::TriggerDeleted { agent_id: 0, trigger_id }),
                    Err(e) => web_sys::console::error_1(&format!("Failed to delete trigger: {:?}", e).into()),
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
            
            // Minimal required flow: (1) create message, (2) trigger processing
            wasm_bindgen_futures::spawn_local(async move {
                // Step 1 – create the user message
                if let Err(e) = ApiClient::create_thread_message(thread_id, &content).await {
                    web_sys::console::error_1(&format!("Executor: Failed to create message for thread {}: {:?}", thread_id, e).into());
                    dispatch_global_message(Message::ThreadMessageFailed(thread_id, client_id_str.clone()));
                    return;
                }

                // Step 2 – run the thread (process unprocessed messages)
                if let Err(e) = ApiClient::run_thread(thread_id).await {
                    web_sys::console::error_1(&format!("Executor: Failed to run thread {}: {:?}", thread_id, e).into());
                    dispatch_global_message(Message::ThreadMessageFailed(thread_id, client_id_str));
                } else {
                    web_sys::console::log_1(&format!("Executor: Processing started for thread {}", thread_id).into());
                }
            });
        },
        Command::RunThread(thread_id) => {
            // This command might now be redundant if SendThreadMessage handles everything.
            // If kept, it should ideally not be called right after SendThreadMessage.
            // Consider removing or repurposing this command.
            web_sys::console::warn_1(&format!("Executor: Handling potentially redundant Command::RunThread for thread {}", thread_id).into());
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::run_thread(thread_id).await {
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
                // Ensure endpoint is absolute
                let endpoint_abs = if endpoint.starts_with("http") {
                    endpoint
                } else {
                    match crate::network::get_api_base_url() {
                        Ok(base) => format!("{}{}", base, endpoint),
                        Err(e) => {
                            web_sys::console::error_1(&format!("API base URL error: {}", e).into());
                            dispatch_global_message(*on_error);
                            return;
                        }
                    }
                };
                let result = match method.as_str() {
                    "GET" => ApiClient::fetch_json(&endpoint_abs, "GET", None).await,
                    "POST" => {
                        if let Some(data) = body {
                            ApiClient::fetch_json(&endpoint_abs, "POST", Some(&data)).await
                        } else {
                            Err("POST request requires body data".into())
                        }
                    },
                    "PUT" => {
                        if let Some(data) = body {
                            ApiClient::fetch_json(&endpoint_abs, "PUT", Some(&data)).await
                        } else {
                            Err("PUT request requires body data".into())
                        }
                    },
                    "DELETE" => ApiClient::fetch_json(&endpoint_abs, "DELETE", None).await,
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

                        // 1. Notify the state layer that the update succeeded so it
                        //    can refresh the agent list (or optimistic cache).
                        dispatch_global_message(*on_success);

                        // 2. Close the configuration modal – this ensures the UI
                        //    only disappears **after** the server responded, fixing
                        //    race-conditions seen in Playwright tests and giving
                        //    users clear confirmation the save completed.
                        dispatch_global_message(Message::CloseAgentModal);
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to update agent {}: {:?}", agent_id, e).into());
                        dispatch_global_message(*on_error);

                        // Re-enable the Save button so the user can retry.
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Some(btn) = doc.get_element_by_id("save-agent") {
                                btn.set_inner_html("Save");
                                let _ = btn.remove_attribute("disabled");
                            }
                        }
                    }
                }
            });
        },
        Command::DeleteAgentApi { agent_id } => {
            wasm_bindgen_futures::spawn_local(async move {
                match ApiClient::delete_agent(agent_id).await {
                    Ok(_) => dispatch_global_message(Message::AgentDeletionSuccess { agent_id }),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Delete agent API call failed: {:?}", e).into());
                        dispatch_global_message(Message::AgentDeletionFailure { 
                            agent_id, 
                            error: format!("Failed to delete agent: {:?}", e) 
                        })
                    }
                }
            });
        },
        _ => web_sys::console::warn_1(&"Unexpected command type in execute_network_command".into())
    }
}

// ---------------------------------------------------------------------------
// Save-state executor – called by debounced SaveState command
// ---------------------------------------------------------------------------

pub fn execute_save_command() {
    // Borrow AppState immutably to serialise current state.  Persistence
    // helpers only need read-access.
    crate::state::APP_STATE.with(|state_ref| {
        let state = state_ref.borrow();
        // Persist viewport & node positions (localStorage for now)
        crate::storage::save_state_to_api(&state);
    });
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
