use std::sync::Arc;

use jarvis_core::JarvisThread;
use jarvis_core::NewThread;
use jarvis_core::ThreadManager;
use jarvis_core::config::Config;
use jarvis_core::protocol::Event;
use jarvis_core::protocol::EventMsg;
use jarvis_core::protocol::Op;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

/// Spawn the agent bootstrapper and op forwarding loop, returning the
/// `UnboundedSender<Op>` used by the UI to submit operations.
pub(crate) fn spawn_agent(
    config: Config,
    app_event_tx: AppEventSender,
    server: Arc<ThreadManager>,
) -> UnboundedSender<Op> {
    let (jarvis_op_tx, mut jarvis_op_rx) = unbounded_channel::<Op>();

    let app_event_tx_clone = app_event_tx;
    tokio::spawn(async move {
        let NewThread {
            thread,
            session_configured,
            ..
        } = match server.start_thread(config.clone()).await {
            Ok(v) => v,
            Err(err) => {
                let message = format!("Failed to initialize Jarvis: {err}");
                tracing::error!("{message}");
                app_event_tx_clone.send(AppEvent::CodexEvent(Event {
                    id: "".to_string(),
                    msg: EventMsg::Error(err.to_error_event(None)),
                }));
                app_event_tx_clone.send(AppEvent::FatalExitRequest(message));
                tracing::error!("failed to initialize Jarvis: {err}");
                return;
            }
        };

        // Inicializar servidores de mensageria se configurado
        if config.messaging.enabled {
            let thread_for_messaging = thread.clone();
            let config_for_messaging = config.clone();
            tokio::spawn(async move {
                // Aguardar um pouco para garantir que a thread está totalmente inicializada
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                // Inicializar os servidores de mensageria usando o thread
                if let Err(e) = jarvis_core::messaging::initialize_messaging_servers_from_thread(
                    &config_for_messaging,
                    thread_for_messaging,
                ).await {
                    tracing::error!("Failed to initialize messaging servers: {}", e);
                } else {
                    tracing::info!("Messaging servers initialized successfully");
                }
            });
        }

        // Forward the captured `SessionConfigured` event so it can be rendered in the UI.
        let ev = jarvis_core::protocol::Event {
            // The `id` does not matter for rendering, so we can use a fake value.
            id: "".to_string(),
            msg: jarvis_core::protocol::EventMsg::SessionConfigured(session_configured),
        };
        app_event_tx_clone.send(AppEvent::CodexEvent(ev));

        let thread_clone = thread.clone();
        tokio::spawn(async move {
            while let Some(op) = jarvis_op_rx.recv().await {
                let id = thread_clone.submit(op).await;
                if let Err(e) = id {
                    tracing::error!("failed to submit op: {e}");
                }
            }
        });

        while let Ok(event) = thread.next_event().await {
            app_event_tx_clone.send(AppEvent::CodexEvent(event));
        }
    });

    jarvis_op_tx
}

/// Spawn agent loops for an existing thread (e.g., a forked thread).
/// Sends the provided `SessionConfiguredEvent` immediately, then forwards subsequent
/// events and accepts Ops for submission.
pub(crate) fn spawn_agent_from_existing(
    thread: std::sync::Arc<JarvisThread>,
    session_configured: jarvis_core::protocol::SessionConfiguredEvent,
    app_event_tx: AppEventSender,
) -> UnboundedSender<Op> {
    let (jarvis_op_tx, mut jarvis_op_rx) = unbounded_channel::<Op>();

    let app_event_tx_clone = app_event_tx;
    tokio::spawn(async move {
        // Forward the captured `SessionConfigured` event so it can be rendered in the UI.
        let ev = jarvis_core::protocol::Event {
            id: "".to_string(),
            msg: jarvis_core::protocol::EventMsg::SessionConfigured(session_configured),
        };
        app_event_tx_clone.send(AppEvent::CodexEvent(ev));

        let thread_clone = thread.clone();
        tokio::spawn(async move {
            while let Some(op) = jarvis_op_rx.recv().await {
                let id = thread_clone.submit(op).await;
                if let Err(e) = id {
                    tracing::error!("failed to submit op: {e}");
                }
            }
        });

        while let Ok(event) = thread.next_event().await {
            app_event_tx_clone.send(AppEvent::CodexEvent(event));
        }
    });

    jarvis_op_tx
}

/// Spawn an op-forwarding loop for an existing thread without subscribing to events.
pub(crate) fn spawn_op_forwarder(thread: std::sync::Arc<JarvisThread>) -> UnboundedSender<Op> {
    let (jarvis_op_tx, mut jarvis_op_rx) = unbounded_channel::<Op>();

    tokio::spawn(async move {
        while let Some(op) = jarvis_op_rx.recv().await {
            if let Err(e) = thread.submit(op).await {
                tracing::error!("failed to submit op: {e}");
            }
        }
    });

    jarvis_op_tx
}
