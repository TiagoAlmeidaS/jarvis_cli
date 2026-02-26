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

use jarvis_core::agent::AgentSessionManager;
use jarvis_core::agent::session_persistent::PersistentAgentSessionManager;
use jarvis_core::agent_loop::AgentLoop;
use jarvis_core::agent_loop::AgentLoopConfig;
use jarvis_core::agent_loop::SafeToolExecutor;
use jarvis_core::agent_loop::bridge::BridgeLlmConfig;
use jarvis_core::agent_loop::bridge::BridgeToolConfig;
use jarvis_core::agent_loop::bridge::BridgeToolExecutor;
use jarvis_core::agent_loop::bridge::default_tool_specs;
use jarvis_core::agent_loop::events::AgentEvent;
use jarvis_core::config::types::AgentLoopSettings;
use jarvis_core::config::types::KnowledgeConfig;
use jarvis_core::intent::RuleBasedIntentDetector;
use jarvis_core::knowledge::InMemoryKnowledgeBase;
use jarvis_core::knowledge::PersistentKnowledgeBase;
use jarvis_core::knowledge::RuleBasedLearningSystem;
use jarvis_core::protocol::AgentMessageEvent;
use jarvis_core::protocol::AskForApproval;
use jarvis_core::protocol::ErrorEvent;
use jarvis_core::protocol::Event;
use jarvis_core::protocol::EventMsg;
use jarvis_core::protocol::ExecCommandBeginEvent;
use jarvis_core::protocol::ExecCommandEndEvent;
use jarvis_core::protocol::ExecCommandSource;
use jarvis_core::protocol::SandboxPolicy;
use jarvis_core::protocol::SessionConfiguredEvent;
use jarvis_core::protocol::TurnCompleteEvent;
use jarvis_core::protocol::TurnStartedEvent;
use jarvis_core::protocol::WarningEvent;
use jarvis_core::safety::RuleBasedSafetyClassifier;
use jarvis_protocol::ThreadId;
use jarvis_protocol::config_types::ModeKind;
use tokio_util::sync::CancellationToken;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

/// A user message to be processed by the agent loop.
pub(crate) struct AgentLoopMessage {
    pub text: String,
    pub cwd: PathBuf,
}

/// Build the system prompt for the agent loop.
///
/// Combines the user-provided instructions with a strong agentic preamble
/// that directs the model to use tools autonomously instead of asking the user.
/// If there is context from a previous session, it is included so the agent
/// can pick up where it left off.
fn build_agent_system_prompt(
    user_instructions: &str,
    cwd: &std::path::Path,
    previous_context: Option<&str>,
) -> String {
    let cwd_display = cwd.display();
    let mut prompt = format!(
        "You are Jarvis, an autonomous AI coding assistant with access to local tools.\n\
         \n\
         ## Core Directives\n\
         - You MUST act autonomously. NEVER ask the user for information you can \
           discover yourself using your tools.\n\
         - When asked to analyze, read, explore, or search a project, IMMEDIATELY \
           use the appropriate tools (list_directory, read_file, grep_search, shell).\n\
         - The current working directory is: {cwd_display}\n\
         - All relative paths should be resolved from this directory.\n\
         - If you need to find files, start by listing the directory structure.\n\
         - If you need file contents, read them directly.\n\
         - If you need to search for patterns, use grep_search.\n\
         - Think step by step, use tools to gather information, then provide \
           a comprehensive answer.\n"
    );

    if let Some(ctx) = previous_context {
        prompt.push_str(&format!(
            "\n## Previous Session Context\n\
             The following is a summary of recent interactions. Use this as background \
             knowledge but do NOT repeat it unless asked.\n\
             {ctx}\n"
        ));
    }

    prompt.push_str(&format!(
        "\n## User Instructions\n\
         {user_instructions}"
    ));

    prompt
}

/// Directory for storing agent session files.
fn sessions_dir(jarvis_home: &std::path::Path) -> PathBuf {
    jarvis_home.join("sessions")
}

/// Build a compact summary of a session's recent history for context injection.
fn build_session_summary(session: &jarvis_core::agent::AgentSession) -> String {
    let mut summary = String::new();

    // Include knowledge base.
    if !session.knowledge_base.is_empty() {
        summary.push_str("### Learned Facts\n");
        for (key, value) in &session.knowledge_base {
            summary.push_str(&format!("- {key}: {value}\n"));
        }
        summary.push('\n');
    }

    // Include last N messages as context (compact form).
    let recent: Vec<_> = session.history.iter().rev().take(20).collect();
    if !recent.is_empty() {
        summary.push_str("### Recent Conversation (newest first)\n");
        for msg in &recent {
            let role = &msg.role;
            // Truncate long messages.
            let content = if msg.content.len() > 300 {
                format!("{}...", &msg.content[..300])
            } else {
                msg.content.clone()
            };
            summary.push_str(&format!("**{role}**: {content}\n"));
        }
    }

    // Include files that were read.
    if !session.files_read.is_empty() {
        summary.push_str("\n### Files Previously Read\n");
        for f in session.files_read.iter().take(30) {
            summary.push_str(&format!("- {}\n", f.display()));
        }
    }

    summary
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
    jarvis_home: PathBuf,
    knowledge_config: KnowledgeConfig,
) -> (
    tokio::sync::mpsc::UnboundedSender<AgentLoopMessage>,
    CancellationToken,
) {
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<AgentLoopMessage>();
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        // Initialize session persistence.
        let session_dir = sessions_dir(&jarvis_home);
        let session_mgr = PersistentAgentSessionManager::new(session_dir.clone());

        // Try to load context from the most recent session.
        let previous_context = match load_latest_session_context(&session_mgr).await {
            Ok(Some(ctx)) => {
                tracing::info!("Loaded previous session context ({} chars)", ctx.len());
                Some(ctx)
            }
            Ok(None) => {
                tracing::debug!("No previous session context found");
                None
            }
            Err(e) => {
                tracing::warn!("Failed to load previous session: {e}");
                None
            }
        };

        // Create a new session for this run.
        let current_session_id = match session_mgr.create_session("agent_loop").await {
            Ok(session) => {
                tracing::info!("Created session: {}", session.session_id);
                Some(session.session_id)
            }
            Err(e) => {
                tracing::warn!("Failed to create session: {e}");
                None
            }
        };

        // Build the full agentic system prompt with CWD, autonomy directives, and context.
        let effective_system_prompt =
            build_agent_system_prompt(&system_prompt, &cwd, previous_context.as_deref());

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

        // ---- KNOWLEDGE SYSTEM ----
        let learning_system: Option<Arc<dyn jarvis_core::knowledge::LearningSystem>> =
            if knowledge_config.enabled {
                let kb: Box<dyn jarvis_core::knowledge::KnowledgeBase> =
                    if knowledge_config.backend == "memory" {
                        Box::new(InMemoryKnowledgeBase::new())
                    } else {
                        let kb_dir = jarvis_home.join(&knowledge_config.storage_dir);
                        Box::new(PersistentKnowledgeBase::new(kb_dir))
                    };
                tracing::info!(
                    "Knowledge system enabled (backend: {}, dir: {})",
                    knowledge_config.backend,
                    knowledge_config.storage_dir
                );
                Some(Arc::new(RuleBasedLearningSystem::new(kb)))
            } else {
                tracing::debug!("Knowledge system disabled");
                None
            };

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
            let raw_tools = Arc::new(BridgeToolExecutor::new(tool_config));

            // Wrap tools with the safety gate so destructive operations are
            // blocked automatically.
            let classifier = Arc::new(RuleBasedSafetyClassifier::default());
            let safe_tools = Arc::new(SafeToolExecutor::new(raw_tools, classifier));

            let loop_config = AgentLoopConfig {
                max_iterations: settings.max_iterations,
                max_duration: Duration::from_secs(settings.timeout_sec),
                max_context_tokens: settings.max_context_tokens,
                ..Default::default()
            };

            let mut agent = AgentLoop::new(llm, safe_tools, loop_config);

            // Attach intent detector so user messages are classified.
            let intent_detector = Arc::new(RuleBasedIntentDetector::default());
            agent = agent.with_intent_detector(intent_detector);

            // Attach learning system if enabled.
            if let Some(ref ls) = learning_system {
                agent = agent.with_learning_system(Arc::clone(ls));
            }

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
                    &effective_system_prompt,
                    &message.text,
                    move |event| {
                        emit_agent_event(&tx_for_events, &event, &cwd_for_events);
                    },
                    turn_cancel,
                )
                .await;

            match result {
                Ok(loop_result) => {
                    // Persist conversation to session.
                    if let Some(ref sid) = current_session_id {
                        let _ = session_mgr.add_message(sid, "user", &message.text).await;
                        if !loop_result.response.is_empty() {
                            let _ = session_mgr
                                .add_message(sid, "assistant", &loop_result.response)
                                .await;
                        }
                        for tool in &loop_result.tools_used {
                            let _ = session_mgr.record_tool_usage(sid, tool).await;
                        }
                    }

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
        AgentEvent::IntentDetected {
            intent_type,
            confidence,
        } => {
            tracing::debug!("AgentLoop: intent detected: {intent_type} (confidence: {confidence})");
            let bg_event = Event {
                id: String::new(),
                msg: EventMsg::BackgroundEvent(jarvis_core::protocol::BackgroundEventEvent {
                    message: format!("Intent: {intent_type} (confidence: {confidence:.0}%)"),
                }),
            };
            tx.send(AppEvent::CodexEvent(bg_event));
        }
        AgentEvent::SafetyWarning {
            tool_name,
            risk_level,
            reasoning,
        } => {
            tracing::warn!("AgentLoop: safety warning for {tool_name}: {reasoning}");
            let warn_event = Event {
                id: String::new(),
                msg: EventMsg::Warning(WarningEvent {
                    message: format!("Safety warning ({risk_level}): {tool_name} — {reasoning}"),
                }),
            };
            tx.send(AppEvent::CodexEvent(warn_event));
        }
        AgentEvent::SafetyBlocked {
            tool_name,
            risk_level,
            reasoning,
        } => {
            tracing::error!("AgentLoop: safety blocked {tool_name}: {reasoning}");
            let err_event = Event {
                id: String::new(),
                msg: EventMsg::Error(ErrorEvent {
                    message: format!("Safety blocked ({risk_level}): {tool_name} — {reasoning}"),
                    jarvis_error_info: None,
                }),
            };
            tx.send(AppEvent::CodexEvent(err_event));
        }
        AgentEvent::KnowledgeLearned {
            items_count,
            summary,
        } => {
            tracing::info!("AgentLoop: learned {items_count} knowledge items");
            let bg_event = Event {
                id: String::new(),
                msg: EventMsg::BackgroundEvent(jarvis_core::protocol::BackgroundEventEvent {
                    message: format!("Learned {items_count} item(s): {summary}"),
                }),
            };
            tx.send(AppEvent::CodexEvent(bg_event));
        }
        AgentEvent::KnowledgeRetrieved { items_count, query } => {
            tracing::debug!("AgentLoop: retrieved {items_count} knowledge items for '{query}'");
            let bg_event = Event {
                id: String::new(),
                msg: EventMsg::BackgroundEvent(jarvis_core::protocol::BackgroundEventEvent {
                    message: format!("Retrieved {items_count} relevant knowledge item(s)"),
                }),
            };
            tx.send(AppEvent::CodexEvent(bg_event));
        }
    }
}

/// Load context from the most recent session file in the sessions directory.
///
/// Returns `Ok(Some(summary))` if a usable session was found, `Ok(None)` if
/// no sessions exist, or `Err` on I/O errors.
async fn load_latest_session_context(
    mgr: &PersistentAgentSessionManager,
) -> anyhow::Result<Option<String>> {
    // List sessions via the manager's internal directory scan.
    // We pick the one with the most recent `updated_at`.
    let session_ids = match mgr.resume_latest_session_ids().await {
        Ok(ids) => ids,
        Err(_) => return Ok(None),
    };

    // Try each session (sorted newest first by the filename convention).
    for sid in session_ids.iter().rev().take(3) {
        if let Ok(session) = mgr.resume_session(sid).await {
            if session.history.is_empty() {
                continue;
            }
            let summary = build_session_summary(&session);
            if !summary.is_empty() {
                return Ok(Some(summary));
            }
        }
    }

    Ok(None)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use jarvis_core::config::types::AgentLoopMode;
    use jarvis_core::config::types::AgentLoopSettings;
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
            AgentEvent::IntentDetected {
                intent_type: "Explore".to_string(),
                confidence: 0.8,
            },
            AgentEvent::SafetyWarning {
                tool_name: "shell".to_string(),
                risk_level: "medium".to_string(),
                reasoning: "Unknown command".to_string(),
            },
            AgentEvent::SafetyBlocked {
                tool_name: "shell".to_string(),
                risk_level: "Critical".to_string(),
                reasoning: "Destructive operation".to_string(),
            },
            AgentEvent::KnowledgeLearned {
                items_count: 2,
                summary: "Learned workflow pattern".to_string(),
            },
            AgentEvent::KnowledgeRetrieved {
                items_count: 3,
                query: "REST API".to_string(),
            },
        ];

        for event in &events {
            emit_agent_event(&sender, event, &cwd);
        }
    }
}
