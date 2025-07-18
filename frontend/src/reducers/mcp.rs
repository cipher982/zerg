//! MCP/Integration domain reducer: handles MCP server/tool management, integration UI, allowed tools, connection testing, etc.

use crate::messages::{Command, Message};
use crate::state::AppState;

/// Handles MCP/integration-related messages. Returns true if the message was handled.
pub fn update(state: &mut AppState, msg: &Message, commands: &mut Vec<Command>) -> bool {
    match msg {
        Message::LoadMcpTools(agent_id) => {
            web_sys::console::log_1(&format!("LoadMcpTools for agent {}", agent_id).into());
            let agent_id_clone = *agent_id;
            commands.push(Command::UpdateUI(Box::new(move || {
                wasm_bindgen_futures::spawn_local(async move {
                    match crate::network::api_client::ApiClient::get_mcp_available_tools(
                        agent_id_clone,
                    )
                    .await
                    {
                        Ok(response) => {
                            match serde_json::from_str::<serde_json::Value>(&response) {
                                Ok(json) => {
                                    let builtin_tools = json["builtin"]
                                        .as_array()
                                        .unwrap_or(&Vec::new())
                                        .iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect::<Vec<String>>();

                                    let mut mcp_tools: std::collections::HashMap<
                                        String,
                                        Vec<crate::state::McpToolInfo>,
                                    > = std::collections::HashMap::new();

                                    if let Some(mcp_obj) = json["mcp"].as_object() {
                                        for (server_name, tools_array) in mcp_obj {
                                            let tools: Vec<crate::state::McpToolInfo> = tools_array
                                                .as_array()
                                                .unwrap_or(&Vec::new())
                                                .iter()
                                                .filter_map(|tool| {
                                                    tool.as_str().map(|name| {
                                                        crate::state::McpToolInfo {
                                                            name: name.to_string(),
                                                            server_name: server_name.clone(),
                                                            description: None,
                                                        }
                                                    })
                                                })
                                                .collect();

                                            if !tools.is_empty() {
                                                mcp_tools.insert(server_name.clone(), tools);
                                            }
                                        }
                                    }

                                    crate::state::dispatch_global_message(
                                        Message::McpToolsLoaded {
                                            agent_id: agent_id_clone,
                                            builtin_tools,
                                            mcp_tools,
                                        },
                                    );
                                }
                                Err(e) => {
                                    web_sys::console::error_1(
                                        &format!("Failed to parse MCP tools response: {:?}", e)
                                            .into(),
                                    );
                                    crate::state::dispatch_global_message(Message::McpError {
                                        agent_id: agent_id_clone,
                                        error: format!("Failed to parse tools: {}", e),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to load MCP tools: {:?}", e).into(),
                            );
                            crate::state::dispatch_global_message(Message::McpError {
                                agent_id: agent_id_clone,
                                error: format!("Failed to load tools: {:?}", e),
                            });
                        }
                    }
                });
            })));
            true
        }
        Message::McpToolsLoaded {
            agent_id,
            builtin_tools,
            mcp_tools,
        } => {
            state
                .available_mcp_tools
                .insert(*agent_id, mcp_tools.values().flatten().cloned().collect());
            web_sys::console::log_1(
                &format!(
                    "McpToolsLoaded for agent {}: {:?} built-in, {:?} mcp",
                    agent_id, builtin_tools, mcp_tools
                )
                .into(),
            );
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(_doc) = win.document() {
                        web_sys::console::log_1(&"Render MCP tools UI after loading".into());
                    }
                }
            })));
            true
        }
        Message::AddMcpServer {
            agent_id,
            server_config,
        } => {
            web_sys::console::log_1(
                &format!("AddMcpServer for agent {}: {:?}", agent_id, server_config).into(),
            );
            let payload = serde_json::to_string(server_config).unwrap_or_else(|_| "{}".to_string());
            commands.push(Command::NetworkCall {
                endpoint: format!("/api/agents/{}/mcp-servers", agent_id),
                method: "POST".to_string(),
                body: Some(payload),
                on_success: Box::new(Message::McpServerAdded {
                    agent_id: *agent_id,
                    server_name: server_config.name.clone(),
                }),
                on_error: Box::new(Message::McpError {
                    agent_id: *agent_id,
                    error: format!("Failed to add server: {}", server_config.name),
                }),
            });
            true
        }
        Message::RemoveMcpServer {
            agent_id,
            server_name,
        } => {
            web_sys::console::log_1(
                &format!("RemoveMcpServer for agent {}: {}", agent_id, server_name).into(),
            );
            commands.push(Command::NetworkCall {
                endpoint: format!("/api/agents/{}/mcp-servers/{}", agent_id, server_name),
                method: "DELETE".to_string(),
                body: None,
                on_success: Box::new(Message::McpServerRemoved {
                    agent_id: *agent_id,
                    server_name: server_name.clone(),
                }),
                on_error: Box::new(Message::McpError {
                    agent_id: *agent_id,
                    error: format!("Failed to remove server: {}", server_name),
                }),
            });
            true
        }
        Message::TestMcpConnection {
            agent_id,
            server_config,
        } => {
            web_sys::console::log_1(
                &format!(
                    "TestMcpConnection for agent {}: {:?}",
                    agent_id, server_config
                )
                .into(),
            );
            let payload = serde_json::to_string(server_config).unwrap_or_else(|_| "{}".to_string());
            commands.push(Command::NetworkCall {
                endpoint: format!("/api/agents/{}/mcp-servers/test", agent_id),
                method: "POST".to_string(),
                body: Some(payload),
                on_success: Box::new(Message::McpConnectionTested {
                    agent_id: *agent_id,
                    server_name: server_config.name.clone(),
                    status: crate::state::ConnectionStatus::Healthy,
                }),
                on_error: Box::new(Message::McpConnectionTested {
                    agent_id: *agent_id,
                    server_name: server_config.name.clone(),
                    status: crate::state::ConnectionStatus::Failed("Connection failed".to_string()),
                }),
            });
            true
        }
        Message::McpConnectionTested {
            agent_id,
            server_name,
            status,
        } => {
            let key = format!("{}:{}", agent_id, server_name);
            state.mcp_connection_status.insert(key, status.clone());
            web_sys::console::log_1(
                &format!(
                    "McpConnectionTested for agent {}: {} status {:?}",
                    agent_id, server_name, status
                )
                .into(),
            );
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(_doc) = win.document() {
                        web_sys::console::log_1(
                            &"Render MCP tools UI after connection test".into(),
                        );
                    }
                }
            })));
            true
        }
        Message::UpdateAllowedTools {
            agent_id,
            allowed_tools,
        } => {
            if let Some(config) = state.agent_mcp_configs.get_mut(agent_id) {
                config.allowed_tools = allowed_tools.clone();
            } else {
                state.agent_mcp_configs.insert(
                    *agent_id,
                    crate::state::AgentMcpConfig {
                        servers: Vec::new(),
                        allowed_tools: allowed_tools.clone(),
                    },
                );
            }
            web_sys::console::log_1(
                &format!(
                    "UpdateAllowedTools for agent {}: {:?}",
                    agent_id,
                    state
                        .agent_mcp_configs
                        .get(agent_id)
                        .map(|c| &c.allowed_tools)
                )
                .into(),
            );
            true
        }
        Message::McpServerAdded {
            agent_id,
            server_name,
        } => {
            commands.push(Command::SendMessage(Message::LoadMcpTools(*agent_id)));
            web_sys::console::log_1(
                &format!("McpServerAdded for agent {}: {}", agent_id, server_name).into(),
            );
            true
        }
        Message::McpServerRemoved {
            agent_id,
            server_name,
        } => {
            commands.push(Command::SendMessage(Message::LoadMcpTools(*agent_id)));
            web_sys::console::log_1(
                &format!("McpServerRemoved for agent {}: {}", agent_id, server_name).into(),
            );
            true
        }
        Message::McpError { agent_id, error } => {
            web_sys::console::error_1(
                &format!("MCP Error for agent {}: {}", agent_id, error).into(),
            );
            true
        }
        Message::SetMCPTab { agent_id, tab } => {
            web_sys::console::log_1(&format!("SetMCPTab for agent {}: {:?}", agent_id, tab).into());
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(_doc) = win.document() {
                        web_sys::console::log_1(&"MCP tab switch UI update".into());
                    }
                }
            })));
            true
        }
        Message::ConnectMCPPreset {
            agent_id,
            preset_id,
        } => {
            web_sys::console::log_1(
                &format!("ConnectMCPPreset for agent {}: {}", agent_id, preset_id).into(),
            );
            let preset_id_cloned = preset_id.clone();
            commands.push(Command::UpdateUI(Box::new(move || {
                if web_sys::window().is_some() {
                    crate::toast::info(&format!(
                        "Connect to {} preset (auth flow coming soon)",
                        preset_id_cloned
                    ));
                }
            })));
            true
        }
        Message::AddMCPServer {
            agent_id,
            url,
            name,
            preset,
            auth_token,
        } => {
            web_sys::console::log_1(
                &format!(
                    "AddMCPServer for agent {}: {} ({})",
                    agent_id,
                    name,
                    url.as_deref().unwrap_or("preset")
                )
                .into(),
            );
            let server_config = crate::state::McpServerConfig {
                name: name.clone(),
                url: url.clone(),
                preset: preset.clone(),
                auth_token: Some(auth_token.clone()),
            };
            commands.push(Command::SendMessage(Message::AddMcpServer {
                agent_id: *agent_id,
                server_config,
            }));
            true
        }
        Message::RemoveMCPServer {
            agent_id,
            server_name,
        } => {
            web_sys::console::log_1(
                &format!("RemoveMCPServer for agent {}: {}", agent_id, server_name).into(),
            );
            commands.push(Command::SendMessage(Message::RemoveMcpServer {
                agent_id: *agent_id,
                server_name: server_name.clone(),
            }));
            true
        }
        Message::TestMCPConnection {
            agent_id,
            url,
            name,
            auth_token,
        } => {
            web_sys::console::log_1(
                &format!(
                    "TestMCPConnection for agent {}: {} at {}",
                    agent_id, name, url
                )
                .into(),
            );
            let server_config = crate::state::McpServerConfig {
                name: name.clone(),
                url: Some(url.clone()),
                preset: None,
                auth_token: Some(auth_token.clone()),
            };
            commands.push(Command::SendMessage(Message::TestMcpConnection {
                agent_id: *agent_id,
                server_config,
            }));
            true
        }
        _ => false,
    }
}
