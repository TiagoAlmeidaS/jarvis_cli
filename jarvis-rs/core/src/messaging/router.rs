//! Router para executar tools através do sistema de mensageria

use std::sync::Arc;
use crate::tools::router::ToolRouter;
use crate::tools::context::ToolInvocation;
use crate::function_tool::FunctionCallError;
use jarvis_protocol::models::ResponseInputItem;

/// Router que executa tools através do sistema de mensageria
pub struct MessagingRouter {
    tool_router: Arc<ToolRouter>,
}

impl MessagingRouter {
    pub fn new(tool_router: Arc<ToolRouter>) -> Self {
        Self { tool_router }
    }

    /// Executa uma tool invocation e retorna o resultado formatado como string
    pub async fn execute_tool(
        &self,
        invocation: ToolInvocation,
    ) -> Result<String, FunctionCallError> {
        // Usa o registry para executar a tool
        let registry = &self.tool_router.registry;
        let result = registry.dispatch(invocation).await?;

        // Converte o ResponseInputItem em string
        Ok(Self::format_response_item(&result))
    }

    /// Formata um ResponseInputItem como string para envio via mensageria
    fn format_response_item(item: &ResponseInputItem) -> String {
        match item {
            ResponseInputItem::FunctionCallOutput { output, .. } => {
                output.content.clone()
            }
            ResponseInputItem::CustomToolCallOutput { output, .. } => {
                output.clone()
            }
            ResponseInputItem::McpToolCallOutput { result, .. } => {
                match result {
                    Ok(call_result) => format!("MCP Result: {:?}", call_result),
                    Err(e) => format!("MCP Error: {}", e),
                }
            }
            _ => {
                format!("Resultado: {:?}", item)
            }
        }
    }
}
