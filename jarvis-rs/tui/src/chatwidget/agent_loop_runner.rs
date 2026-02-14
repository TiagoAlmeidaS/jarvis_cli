//! Agent loop runner for the TUI.
//!
//! When the active model is detected as text-based (no native function calling),
//! or when the user explicitly configures `agent_loop.mode = "text_based"`,
//! this module runs the [`AgentLoop`] in place of the standard Responses API flow.
//!
//! It translates [`AgentEvent`]s into protocol [`Event`]/[`EventMsg`] variants
//! so the chatwidget can render tool calls, progress, and final responses
//! using its existing rendering pipeline.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use jarvis_core::agent_loop::bridge::{
    default_tool_specs, BridgeLlmConfig, BridgeToolConfig, BridgeToolExecutor,
};
use jarvis_core::agent_loop::events::AgentEvent;
use jarvis_core::agent_loop::{AgentLoop, AgentLoopConfig};
use jarvis_core::config::types::AgentLoopSettings;
use jarvis_core::protocol::{
    AgentMessageEvent, AskForApproval, ErrorEvent, Event, EventMsg, ExecCommandBeginEvent,
    ExecCommandEndEvent, ExecCommandSource, SandboxPolicy, SessionConfiguredEvent,
    TurnCompleteEvent, TurnStartedEvent, WarningEvent,
};
use jarvis_protocol::config_types::ModeKind;
use jarvis_protocol::ThreadId;
use tokio_util::sync::CancellationToken;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

/// A user message to be processed by the agent loop.
pub(crate) struct AgentLoopMessage {
    pub text: String,
    pub cwd: PathBuf,
}

/// Spawn the agent loop runner.
///
/// Returns a sender for submitting user messages and a cancellation token
/// that the TUI can use to interrupt the running loop.
pub(crate) fn spawn_agent_loop_runner(
    settings: AgentLoopSettings,
    system_prompt: String,
    app_event_tx: AppEventSender,
    cwd: PathBuf,
) -> (
    tokio::sync::mpsc::UnboundedSender<AgentLoopMessage>,
    CancellationToken,
) {
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<AgentLoopMessage>();
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        // Send a synthetic SessionConfigured so the chatwidget enters "ready" state.
        let model_name = settings
            .model
            .clone()
            .unwrap_or_else(|| "text-based-local".to_string());
        let session_event = Event {
            id: String::new(),
            msg: EventMsg::SessionConfigured(SessionConfiguredEvent {
                session_id: ThreadId::new(),
                forked_from_id: None,
                thread_name: None,
                model: model_name.clone(),
                model_provider_id: "agent-loop-bridge".to_string(),
                approval_policy: AskForApproval::Never,
                sandbox_policy: SandboxPolicy::DangerFullAccess,
                cwd: cwd.clone(),
                reasoning_effort: None,
                history_log_id: 0,
                history_entry_count: 0,
                initial_messages: None,
                rollout_path: None,
            }),
        };
        app_event_tx.send(AppEvent::CodexEvent(session_event));

        while let Some(message) = msg_rx.recv().await {
            let working_dir = message.cwd;

            // Build bridge components
            let llm_config = BridgeLlmConfig {
                base_url: settings.base_url.clone(),
                api_key: settings.api_key.clone(),
                model: settings
                    .model
                    .clone()
                    .unwrap_or_else(|| "mistral".to_string()),
                temperature: settings.temperature,
                max_tokens: settings.max_tokens,
                timeout_sec: settings.timeout_sec,
            };

            let tool_specs = default_tool_specs();
            let llm = match jarvis_core::agent_loop::bridge::BridgeLlmClient::new(
                llm_config,
                &tool_specs,
            ) {
                Ok(client) => Arc::new(client),
                Err(e) => {
                    let err_event = Event {
                        id: String::new(),
                        msg: EventMsg::Error(ErrorEvent {
                            message: format!("Failed to create LLM client: {e:#}"),
                            jarvis_error_info: None,
                        }),
                    };
                    app_event_tx.send(AppEvent::CodexEvent(err_event));
                    continue;
                }
            };

            let tool_config = BridgeToolConfig {
                working_dir: working_dir.clone(),
                shell_timeout: Duration::from_secs(30),
                require_approval: false,
            };
            let tools = Arc::new(BridgeToolExecutor::new(tool_config));

            let loop_config = AgentLoopConfig {
                max_iterations: settings.max_iterations,
                max_duration: Duration::from_secs(settings.timeout_sec),
                max_context_tokens: settings.max_context_tokens,
                ..Default::default()
            };

            let agent = AgentLoop::new(llm, tools, loop_config);

            // Emit TurnStarted
            let turn_started = Event {
                id: String::new(),
                msg: EventMsg::TurnStarted(TurnStartedEvent {
                    model_context_window: Some(settings.max_context_tokens as i64),
                    collaboration_mode_kind: ModeKind::default(),
                }),
            };
            app_event_tx.send(AppEvent::CodexEvent(turn_started));

            let tx_for_events = app_event_tx.clone();
            let cwd_for_events = working_dir.clone();
            let turn_cancel = cancel_clone.child_token();

            let result = agent
                .run(
                    &system_prompt,
                    &message.text,
                    move |event| {
                        emit_agent_event(&tx_for_events, &event, &cwd_for_events);
                    },
                    turn_cancel,
                )
                .await;

            match result {
                Ok(loop_result) => {
                    // Emit final message if not already emitted via FinalResponse event
                    if !loop_result.response.is_empty() {
                        let msg_event = Event {
                            id: String::new(),
                            msg: EventMsg::AgentMessage(AgentMessageEvent {
                                message: loop_result.response.clone(),
                            }),
                        };
                        app_event_tx.send(AppEvent::CodexEvent(msg_event));
                    }

                    // Emit TurnComplete
                    let turn_complete = Event {
                        id: String::new(),
                        msg: EventMsg::TurnComplete(TurnCompleteEvent {
                            last_agent_message: if loop_result.response.is_empty() {
                                None
                            } else {
                                Some(loop_result.response)
                            },
                        }),
                    };
                    app_event_tx.send(AppEvent::CodexEvent(turn_complete));
                }
                Err(e) => {
                    let err_event = Event {
                        id: String::new(),
                        msg: EventMsg::Error(ErrorEvent {
                            message: format!("Agent loop error: {e:#}"),
                            jarvis_error_info: None,
                        }),
                    };
                    app_event_tx.send(AppEvent::CodexEvent(err_event));

                    let turn_complete = Event {
                        id: String::new(),
                        msg: EventMsg::TurnComplete(TurnCompleteEvent {
                            last_agent_message: None,
                        }),
                    };
                    app_event_tx.send(AppEvent::CodexEvent(turn_complete));
                }
            }
        }
    });

    (msg_tx, cancel)
}

/// Translate a single `AgentEvent` into protocol events for the TUI.
fn emit_agent_event(tx: &AppEventSender, event: &AgentEvent, cwd: &PathBuf) {
    match event {
        AgentEvent::Thinking {
            iteration,
            max_iterations,
        } => {
            tracing::debug!("AgentLoop: thinking (iteration {iteration}/{max_iterations})");
            // The TUI's spinner is already driven by the TurnStarted event,
            // so we emit a background event for additional iterations.
            if *iteration > 0 {
                let bg_event = Event {
                    id: String::new(),
                    msg: EventMsg::BackgroundEvent(jarvis_core::protocol::BackgroundEventEvent {
                        message: format!("Thinking... (step {}/{max_iterations})", iteration + 1,),
                    }),
                };
                tx.send(AppEvent::CodexEvent(bg_event));
            }
        }
        AgentEvent::ExecutingTool {
            name,
            arguments,
            iteration,
        } => {
            let call_id = format!("agentloop_{iteration}_{name}");
            let begin_event = Event {
                id: String::new(),
                msg: EventMsg::ExecCommandBegin(ExecCommandBeginEvent {
                    call_id,
                    process_id: None,
                    turn_id: "agent_loop_turn".to_string(),
                    command: vec![name.clone(), arguments.clone()],
                    cwd: cwd.clone(),
                    parsed_cmd: vec![],
                    source: ExecCommandSource::Agent,
                    interaction_input: None,
                }),
            };
            tx.send(AppEvent::CodexEvent(begin_event));
        }
        AgentEvent::ToolResult {
            name,
            output_preview,
            is_error,
            iteration,
        } => {
            let call_id = format!("agentloop_{iteration}_{name}");
            let end_event = Event {
                id: String::new(),
                msg: EventMsg::ExecCommandEnd(ExecCommandEndEvent {
                    call_id,
                    process_id: None,
                    turn_id: "agent_loop_turn".to_string(),
                    command: vec![name.clone()],
                    cwd: cwd.clone(),
                    parsed_cmd: vec![],
                    source: ExecCommandSource::Agent,
                    interaction_input: None,
                    stdout: output_preview.clone(),
                    stderr: String::new(),
                    aggregated_output: output_preview.clone(),
                    exit_code: if *is_error { 1 } else { 0 },
                    duration: Duration::from_millis(0),
                    formatted_output: output_preview.clone(),
                }),
            };
            tx.send(AppEvent::CodexEvent(end_event));
        }
        AgentEvent::FinalResponse { content, .. } => {
            // Will be emitted as AgentMessage after the loop completes.
            tracing::debug!("AgentLoop: final response ({} chars)", content.len());
        }
        AgentEvent::CompactingContext {
            estimated_tokens,
            max_tokens,
        } => {
            let bg_event = Event {
                id: String::new(),
                msg: EventMsg::BackgroundEvent(jarvis_core::protocol::BackgroundEventEvent {
                    message: format!(
                        "Compacting context ({estimated_tokens} tokens > {max_tokens} max)"
                    ),
                }),
            };
            tx.send(AppEvent::CodexEvent(bg_event));
        }
        AgentEvent::MaxIterationsReached { iterations } => {
            let warn_event = Event {
                id: String::new(),
                msg: EventMsg::Warning(WarningEvent {
                    message: format!("Agent loop reached maximum iterations ({iterations})"),
                }),
            };
            tx.send(AppEvent::CodexEvent(warn_event));
        }
        AgentEvent::Timeout { elapsed } => {
            let warn_event = Event {
                id: String::new(),
                msg: EventMsg::Warning(WarningEvent {
                    message: format!("Agent loop timed out after {:.1}s", elapsed.as_secs_f64()),
                }),
            };
            tx.send(AppEvent::CodexEvent(warn_event));
        }
        AgentEvent::Cancelled => {
            tracing::debug!("AgentLoop: cancelled");
        }
        AgentEvent::Error { message } => {
            let err_event = Event {
                id: String::new(),
                msg: EventMsg::Error(ErrorEvent {
                    message: message.clone(),
                    jarvis_error_info: None,
                }),
            };
            tx.send(AppEvent::CodexEvent(err_event));
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use jarvis_core::config::types::{AgentLoopMode, AgentLoopSettings};
    use pretty_assertions::assert_eq;

    #[test]
    fn agent_loop_message_construction() {
        let msg = AgentLoopMessage {
            text: "Hello world".to_string(),
            cwd: PathBuf::from("/tmp"),
        };
        assert_eq!(msg.text, "Hello world");
        assert_eq!(msg.cwd, PathBuf::from("/tmp"));
    }

    #[test]
    fn settings_defaults_are_sane() {
        let settings = AgentLoopSettings::default();
        assert_eq!(settings.mode, AgentLoopMode::Auto);
        assert_eq!(settings.base_url, "http://localhost:11434/v1");
        assert_eq!(settings.max_iterations, 25);
        assert_eq!(settings.timeout_sec, 300);
    }

    #[test]
    fn effective_mode_auto_detects_text_based() {
        let settings = AgentLoopSettings::default();
        assert_eq!(
            settings.effective_mode("mistral-nemo"),
            AgentLoopMode::TextBased
        );
        assert_eq!(settings.effective_mode("gpt-4o"), AgentLoopMode::Native);
        assert_eq!(
            settings.effective_mode("phi-3-mini"),
            AgentLoopMode::TextBased
        );
    }

    #[test]
    fn effective_mode_explicit_override() {
        let mut settings = AgentLoopSettings::default();
        settings.mode = AgentLoopMode::TextBased;
        // Even for a model that would be Native, explicit override wins.
        assert_eq!(settings.effective_mode("gpt-4o"), AgentLoopMode::TextBased);
    }

    #[test]
    fn emit_agent_event_does_not_panic() {
        // Ensure emit_agent_event handles all variants without panicking.
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sender = AppEventSender::new(tx);
        let cwd = PathBuf::from(".");

        let events = vec![
            AgentEvent::Thinking {
                iteration: 0,
                max_iterations: 10,
            },
            AgentEvent::Thinking {
                iteration: 1,
                max_iterations: 10,
            },
            AgentEvent::ExecutingTool {
                name: "shell".to_string(),
                arguments: r#"{"command":"ls"}"#.to_string(),
                iteration: 0,
            },
            AgentEvent::ToolResult {
                name: "shell".to_string(),
                output_preview: "file1.txt\nfile2.txt".to_string(),
                is_error: false,
                iteration: 0,
            },
            AgentEvent::FinalResponse {
                content: "Done.".to_string(),
                iteration: 1,
            },
            AgentEvent::CompactingContext {
                estimated_tokens: 40000,
                max_tokens: 32000,
            },
            AgentEvent::MaxIterationsReached { iterations: 25 },
            AgentEvent::Timeout {
                elapsed: Duration::from_secs(300),
            },
            AgentEvent::Cancelled,
            AgentEvent::Error {
                message: "test error".to_string(),
            },
        ];

        for event in &events {
            emit_agent_event(&sender, event, &cwd);
        }
    }
}
