//! Handler que conecta mensagens de mensageria ao sistema de tools do Jarvis

use crate::Session;
use crate::TurnContext;
use crate::tools::context::{SharedTurnDiffTracker, ToolInvocation, ToolPayload};
use crate::tools::router::ToolRouter;
use crate::turn_diff_tracker::TurnDiffTracker;
use jarvis_messaging::handler::MessageHandler;
use jarvis_messaging::message::{IncomingMessage, MessageType, OutgoingMessage};
use jarvis_messaging::platform::MessagingPlatform;
use std::sync::Arc;

use super::command_parser::{CommandParser, ParsedCommand};
use super::router::MessagingRouter;

/// Handler que processa mensagens recebidas e as converte em comandos do Jarvis
pub struct MessageToJarvisHandler {
    session: Arc<Session>,
    turn_context: Arc<TurnContext>,
    tool_router: Arc<ToolRouter>,
    messaging_platform: Arc<dyn MessagingPlatform>,
    tracker: SharedTurnDiffTracker,
}

impl MessageToJarvisHandler {
    pub fn new(
        session: Arc<Session>,
        turn_context: Arc<TurnContext>,
        tool_router: Arc<ToolRouter>,
        messaging_platform: Arc<dyn MessagingPlatform>,
    ) -> Self {
        let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
        Self {
            session,
            turn_context,
            tool_router,
            messaging_platform,
            tracker,
        }
    }

    /// Processa uma mensagem recebida e executa o comando correspondente
    async fn process_message(&self, message: IncomingMessage) -> anyhow::Result<String> {
        // Parseia o comando da mensagem
        let command = CommandParser::parse(&message.message_type);

        match command {
            ParsedCommand::Help => Ok(CommandParser::help_message()),
            ParsedCommand::Exec { command, args } => self.handle_exec_command(command, args).await,
            ParsedCommand::Read { path } => self.handle_read_command(path).await,
            ParsedCommand::List { path } => self.handle_list_command(path).await,
            ParsedCommand::Search { query } => self.handle_search_command(query).await,
            ParsedCommand::Unknown { text } => Ok(format!(
                "Comando desconhecido ou inválido. Use /help para ver os comandos disponíveis.\n\nMensagem recebida: {}",
                text
            )),
        }
    }

    async fn handle_exec_command(
        &self,
        command: String,
        args: Vec<String>,
    ) -> anyhow::Result<String> {
        // Cria um payload de shell command
        let full_command = if args.is_empty() {
            command.clone()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        let payload = ToolPayload::Function {
            arguments: serde_json::json!({
                "command": command,
                "args": args
            })
            .to_string(),
        };

        let invocation = ToolInvocation {
            session: Arc::clone(&self.session),
            turn: Arc::clone(&self.turn_context),
            tracker: Arc::clone(&self.tracker),
            call_id: format!("msg-{}", uuid::Uuid::new_v4()),
            tool_name: "shell".to_string(),
            payload,
        };

        // Executa através do router
        let router = MessagingRouter::new(Arc::clone(&self.tool_router));
        match router.execute_tool(invocation).await {
            Ok(output) => Ok(format!(
                "Comando executado:\n```\n{}\n```\n{}",
                full_command, output
            )),
            Err(e) => Ok(format!("Erro ao executar comando: {}", e)),
        }
    }

    async fn handle_read_command(&self, path: String) -> anyhow::Result<String> {
        let payload = ToolPayload::Function {
            arguments: serde_json::json!({
                "file_path": path
            })
            .to_string(),
        };

        let invocation = ToolInvocation {
            session: Arc::clone(&self.session),
            turn: Arc::clone(&self.turn_context),
            tracker: Arc::clone(&self.tracker),
            call_id: format!("msg-{}", uuid::Uuid::new_v4()),
            tool_name: "read_file".to_string(),
            payload,
        };

        let router = MessagingRouter::new(Arc::clone(&self.tool_router));
        match router.execute_tool(invocation).await {
            Ok(output) => Ok(format!(
                "Conteúdo do arquivo `{}`:\n```\n{}\n```",
                path, output
            )),
            Err(e) => Ok(format!("Erro ao ler arquivo: {}", e)),
        }
    }

    async fn handle_list_command(&self, path: String) -> anyhow::Result<String> {
        let payload = ToolPayload::Function {
            arguments: serde_json::json!({
                "path": path
            })
            .to_string(),
        };

        let invocation = ToolInvocation {
            session: Arc::clone(&self.session),
            turn: Arc::clone(&self.turn_context),
            tracker: Arc::clone(&self.tracker),
            call_id: format!("msg-{}", uuid::Uuid::new_v4()),
            tool_name: "list_dir".to_string(),
            payload,
        };

        let router = MessagingRouter::new(Arc::clone(&self.tool_router));
        match router.execute_tool(invocation).await {
            Ok(output) => Ok(format!(
                "Conteúdo do diretório `{}`:\n```\n{}\n```",
                path, output
            )),
            Err(e) => Ok(format!("Erro ao listar diretório: {}", e)),
        }
    }

    async fn handle_search_command(&self, query: String) -> anyhow::Result<String> {
        let payload = ToolPayload::Function {
            arguments: serde_json::json!({
                "query": query
            })
            .to_string(),
        };

        let invocation = ToolInvocation {
            session: Arc::clone(&self.session),
            turn: Arc::clone(&self.turn_context),
            tracker: Arc::clone(&self.tracker),
            call_id: format!("msg-{}", uuid::Uuid::new_v4()),
            tool_name: "grep_files".to_string(),
            payload,
        };

        let router = MessagingRouter::new(Arc::clone(&self.tool_router));
        match router.execute_tool(invocation).await {
            Ok(output) => Ok(format!(
                "Resultados da busca por `{}`:\n```\n{}\n```",
                query, output
            )),
            Err(e) => Ok(format!("Erro ao buscar: {}", e)),
        }
    }

    /// Envia uma resposta de volta para a plataforma
    async fn send_response(&self, chat_id: String, response: String) -> anyhow::Result<()> {
        let message = OutgoingMessage {
            chat_id,
            message_type: MessageType::Text(response),
            reply_to: None,
        };

        self.messaging_platform.send_message(message).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl MessageHandler for MessageToJarvisHandler {
    async fn handle_message(&self, message: IncomingMessage) -> anyhow::Result<()> {
        // Processa a mensagem e obtém a resposta
        let response = self.process_message(message.clone()).await?;

        // Envia a resposta de volta
        self.send_response(message.chat_id, response).await?;

        Ok(())
    }
}
