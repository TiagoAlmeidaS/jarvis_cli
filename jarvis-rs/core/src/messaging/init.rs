//! Inicialização dos servidores de mensageria
//!
//! Este módulo inicializa os servidores de webhook para WhatsApp e Telegram
//! quando a configuração estiver habilitada.

use crate::JarvisThread;
use crate::Session;
use crate::TurnContext;
use crate::config::Config;
use crate::messaging::handler::MessageToJarvisHandler;
use crate::tools::router::ToolRouter;
use std::sync::Arc;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Inicializa os servidores de mensageria baseado na configuração usando um JarvisThread.
/// Esta é a versão recomendada que cria os componentes necessários a partir do thread.
pub async fn initialize_messaging_servers_from_thread(
    config: &Config,
    thread: Arc<JarvisThread>,
) -> anyhow::Result<()> {
    if !config.messaging.enabled {
        info!("Messaging integrations are disabled in config");
        return Ok(());
    }

    // Obter Session do thread
    let session = thread.session();

    // Criar um TurnContext temporário para inicialização
    // Usamos new_default_turn que cria um contexto válido baseado na configuração da sessão
    let turn_context = session.new_default_turn().await;

    // Criar ToolRouter a partir do TurnContext
    let tool_router = Arc::new(ToolRouter::from_config(
        &turn_context.tools_config,
        None, // MCP tools serão carregados quando necessário
        turn_context.dynamic_tools.as_slice(),
    ));

    initialize_messaging_servers(config, session, turn_context, tool_router).await
}

/// Inicializa os servidores de mensageria baseado na configuração
pub async fn initialize_messaging_servers(
    config: &Config,
    session: Arc<Session>,
    turn_context: Arc<TurnContext>,
    tool_router: Arc<ToolRouter>,
) -> anyhow::Result<()> {
    if !config.messaging.enabled {
        info!("Messaging integrations are disabled in config");
        return Ok(());
    }

    let mut handles = Vec::new();

    // Inicializar WhatsApp se configurado
    if let Some(whatsapp_config) = &config.messaging.whatsapp {
        if whatsapp_config.enabled {
            info!(
                "Initializing WhatsApp webhook server on port {}",
                whatsapp_config.webhook_port
            );
            match initialize_whatsapp_server(
                whatsapp_config.clone(),
                session.clone(),
                turn_context.clone(),
                tool_router.clone(),
            )
            .await
            {
                Ok(handle) => {
                    handles.push(handle);
                    info!("WhatsApp webhook server started successfully");
                }
                Err(e) => {
                    error!("Failed to start WhatsApp webhook server: {}", e);
                    warn!("WhatsApp integration will not be available");
                }
            }
        }
    }

    // Inicializar Telegram se configurado
    if let Some(telegram_config) = &config.messaging.telegram {
        if telegram_config.enabled {
            info!(
                "Initializing Telegram webhook server on port {}",
                telegram_config.webhook_port
            );
            match initialize_telegram_server(
                telegram_config.clone(),
                session.clone(),
                turn_context.clone(),
                tool_router.clone(),
            )
            .await
            {
                Ok(handle) => {
                    handles.push(handle);
                    info!("Telegram webhook server started successfully");
                }
                Err(e) => {
                    error!("Failed to start Telegram webhook server: {}", e);
                    warn!("Telegram integration will not be available");
                }
            }
        }
    }

    if handles.is_empty() {
        warn!("No messaging servers were started. Check your configuration.");
    } else {
        info!("Started {} messaging server(s)", handles.len());
    }

    // Aguardar indefinidamente (os servidores rodam em background)
    // Em uma implementação real, você pode querer retornar os handles
    // para que o chamador possa gerenciá-los
    Ok(())
}

async fn initialize_whatsapp_server(
    config: crate::config::types::WhatsAppConfig,
    session: Arc<Session>,
    turn_context: Arc<TurnContext>,
    tool_router: Arc<ToolRouter>,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    use jarvis_whatsapp::client::WhatsAppClient;
    use jarvis_whatsapp::config::WhatsAppConfig as WhatsAppCrateConfig;
    use jarvis_whatsapp::platform::WhatsAppPlatform;

    // Converter configuração do core para configuração do crate
    let whatsapp_crate_config = WhatsAppCrateConfig {
        api_url: config.api_url,
        access_token: config.access_token,
        verify_token: config.verify_token,
        phone_number_id: config.phone_number_id,
        webhook_port: config.webhook_port,
    };

    let client = WhatsAppClient::new(whatsapp_crate_config.clone());
    let platform = WhatsAppPlatform::new(client, whatsapp_crate_config.clone());

    // Criar handler
    let handler = MessageToJarvisHandler::new(
        session,
        turn_context,
        tool_router,
        Arc::new(platform) as Arc<dyn jarvis_messaging::platform::MessagingPlatform>,
    );

    // Criar servidor webhook
    use jarvis_whatsapp::webhook::WhatsAppWebhookServer;
    let server = WhatsAppWebhookServer::new(
        Box::new(handler) as Box<dyn jarvis_messaging::handler::MessageHandler>,
        whatsapp_crate_config,
    );

    // Iniciar servidor em background
    let handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            error!("WhatsApp webhook server error: {}", e);
        }
    });

    Ok(handle)
}

async fn initialize_telegram_server(
    config: crate::config::types::TelegramConfig,
    session: Arc<Session>,
    turn_context: Arc<TurnContext>,
    tool_router: Arc<ToolRouter>,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    use jarvis_telegram::client::TelegramClient;
    use jarvis_telegram::config::TelegramConfig as TelegramCrateConfig;
    use jarvis_telegram::platform::TelegramPlatform;

    // Converter configuração do core para configuração do crate
    let telegram_crate_config = TelegramCrateConfig {
        bot_token: config.bot_token,
        webhook_url: config.webhook_url,
        webhook_port: config.webhook_port,
        webhook_secret: config.webhook_secret,
    };

    let client = TelegramClient::new(telegram_crate_config.clone());
    let platform = TelegramPlatform::new(client, telegram_crate_config.clone());

    // Criar handler
    let handler = MessageToJarvisHandler::new(
        session,
        turn_context,
        tool_router,
        Arc::new(platform) as Arc<dyn jarvis_messaging::platform::MessagingPlatform>,
    );

    // Criar servidor webhook
    use jarvis_telegram::webhook::TelegramWebhookServer;
    let server = TelegramWebhookServer::new(
        Box::new(handler) as Box<dyn jarvis_messaging::handler::MessageHandler>,
        telegram_crate_config,
    );

    // Iniciar servidor em background
    let handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            error!("Telegram webhook server error: {}", e);
        }
    });

    Ok(handle)
}
