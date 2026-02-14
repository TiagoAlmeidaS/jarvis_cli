# Agentic Loop — Think, Execute, Observe, Repeat

**Status**: Implementado (core module)
**Prioridade**: ALTA
**Gap**: G5
**Roadmap**: Fase 2, Step 2.2
**Ultima atualizacao**: 2026-02-13

---

## 1. Problema

O TUI atual delega **todo o raciocinio** ao modelo LLM. Quando o usuario pede
"analise o projeto", o modelo precisa sozinho decidir o que ler, ler, processar
e responder — tudo em uma unica chamada (ou com tool calling nativo do modelo).

Modelos baratos nao conseguem fazer isso. E mesmo modelos premium as vezes
perdem o fio da meada em tarefas longas.

### Como Claude Code / Cursor fazem

Eles implementam um **agentic loop client-side**:

```
1. THINK:   O modelo recebe contexto e decide a proxima acao
2. EXECUTE: O client executa a acao (tool call)
3. OBSERVE: O client captura o resultado e adiciona ao contexto
4. DECIDE:  O modelo ve o resultado e decide: continuar ou responder
5. REPEAT:  Volta ao passo 1 se houver mais acoes necessarias
```

O **client** gerencia o loop, nao o modelo. O modelo so precisa decidir
"qual tool usar" e "quando parar".

---

## 2. Solucao: AgentLoop no Jarvis

### Arquitetura

```
┌─────────────────────────────────────────────────────────────┐
│                       AGENT LOOP                             │
│                                                              │
│  ┌────────────┐     ┌──────────────┐     ┌───────────────┐  │
│  │   THINK    │────▶│   EXECUTE    │────▶│   OBSERVE     │  │
│  │            │     │              │     │               │  │
│  │ LLM decide │     │ Tool         │     │ Captura       │  │
│  │ proxima    │     │ Dispatcher   │     │ resultado +   │  │
│  │ acao       │     │ executa      │     │ adiciona ao   │  │
│  │            │     │ a tool       │     │ contexto      │  │
│  └─────▲──────┘     └──────────────┘     └───────┬───────┘  │
│        │                                          │          │
│        │              ┌──────────┐                │          │
│        └──────────────│  DECIDE  │◄───────────────┘          │
│                       │          │                           │
│                       │ Continuar│                           │
│                       │ ou parar?│                           │
│                       └──────────┘                           │
│                                                              │
│  Condicoes de parada:                                        │
│  - Modelo respondeu sem tool_calls                           │
│  - Max iterations atingido                                   │
│  - Timeout                                                   │
│  - Usuario cancelou (Ctrl+C / Esc)                          │
│  - Erro critico                                              │
└─────────────────────────────────────────────────────────────┘
```

### Componente principal

```rust
pub struct AgentLoop {
    llm_client: Arc<dyn LlmClient>,
    tool_registry: Arc<ToolRegistry>,
    tool_dispatcher: ToolDispatcher,
    config: AgentLoopConfig,
}

pub struct AgentLoopConfig {
    /// Maximum number of think-execute-observe iterations.
    pub max_iterations: usize,         // default: 25
    /// Maximum total time for the loop.
    pub max_duration: Duration,        // default: 5 minutes
    /// Maximum context window tokens before summarization.
    pub max_context_tokens: usize,     // default: 32000
    /// Whether to stream partial responses.
    pub stream_responses: bool,        // default: true
    /// Approval policy for tool execution.
    pub approval_policy: AskForApproval,
    /// Sandbox policy.
    pub sandbox_policy: SandboxPolicy,
}

impl AgentLoop {
    /// Run the agentic loop for a user message.
    pub async fn run(
        &self,
        user_message: &str,
        history: &mut Vec<Message>,
        on_event: impl Fn(AgentEvent),
        cancel: CancellationToken,
    ) -> Result<AgentResult> {

        // Add user message to history.
        history.push(Message::user(user_message));

        let mut iteration = 0;
        let start = Instant::now();

        loop {
            // Check stop conditions.
            if iteration >= self.config.max_iterations {
                on_event(AgentEvent::MaxIterationsReached);
                break;
            }
            if start.elapsed() > self.config.max_duration {
                on_event(AgentEvent::Timeout);
                break;
            }
            if cancel.is_cancelled() {
                on_event(AgentEvent::Cancelled);
                break;
            }

            // THINK: Send context to LLM, get response.
            on_event(AgentEvent::Thinking { iteration });

            let response = self.llm_client.chat(
                history,
                &self.tool_registry.to_api_tools(),
            ).await?;

            // Check if model wants to use tools.
            if response.tool_calls.is_empty() {
                // Model responded without tools — we're done.
                history.push(Message::assistant(&response.content));
                on_event(AgentEvent::FinalResponse {
                    content: response.content.clone(),
                });
                return Ok(AgentResult {
                    response: response.content,
                    iterations: iteration + 1,
                    tools_used: vec![],
                });
            }

            // EXECUTE: Run each tool call.
            history.push(Message::assistant_with_tool_calls(&response.tool_calls));

            for tool_call in &response.tool_calls {
                on_event(AgentEvent::ExecutingTool {
                    name: tool_call.function.name.clone(),
                    args: tool_call.function.arguments.clone(),
                });

                // Check approval.
                let tool = self.tool_registry.get(&tool_call.function.name);
                if let Some(tool) = &tool {
                    if tool.requires_approval()
                        && self.config.approval_policy != AskForApproval::Never
                    {
                        on_event(AgentEvent::WaitingForApproval {
                            tool_name: tool_call.function.name.clone(),
                            args: tool_call.function.arguments.clone(),
                        });
                        // Wait for user approval via channel...
                    }
                }

                // Execute the tool.
                let result = self.tool_dispatcher.dispatch(
                    &tool_call.function.name,
                    &tool_call.function.arguments,
                ).await;

                // OBSERVE: Add result to context.
                let output = match result {
                    Ok(r) => r.output,
                    Err(e) => format!("Error: {e}"),
                };

                on_event(AgentEvent::ToolResult {
                    name: tool_call.function.name.clone(),
                    output: output.clone(),
                });

                history.push(Message::tool_result(
                    &tool_call.id,
                    &output,
                ));
            }

            // Context management: summarize if too long.
            if self.estimate_tokens(history) > self.config.max_context_tokens {
                on_event(AgentEvent::SummarizingContext);
                self.summarize_context(history).await?;
            }

            iteration += 1;
        }

        Ok(AgentResult {
            response: String::new(),
            iterations: iteration,
            tools_used: vec![],
        })
    }
}
```

---

## 3. Eventos do Loop (UX no TUI)

O TUI precisa mostrar ao usuario o que o agente esta fazendo em tempo real:

```rust
pub enum AgentEvent {
    /// Agent is thinking (calling LLM).
    Thinking { iteration: usize },
    /// Agent is executing a tool.
    ExecutingTool { name: String, args: String },
    /// Tool execution completed.
    ToolResult { name: String, output: String },
    /// Agent is waiting for user approval.
    WaitingForApproval { tool_name: String, args: String },
    /// Agent is summarizing context (context too long).
    SummarizingContext,
    /// Agent produced final response.
    FinalResponse { content: String },
    /// Agent hit max iterations.
    MaxIterationsReached,
    /// Agent timed out.
    Timeout,
    /// Agent was cancelled by user.
    Cancelled,
    /// Streaming partial text from LLM.
    StreamChunk { text: String },
}
```

### Visualizacao no TUI

```
> Analise o projeto jarvis_cli e sugira melhorias

  [1/25] Thinking...
  → list_directory(".")
    jarvis-rs/  docs/  scripts/  .env.example  config.toml.example  ...

  [2/25] Thinking...
  → read_file("docs/architecture/autonomy-roadmap.md")
    (624 lines read)

  [3/25] Thinking...
  → grep_search("TODO|FIXME|HACK", "jarvis-rs/")
    Found 12 matches in 8 files

  [4/25] Thinking...
  → read_file("jarvis-rs/daemon/src/runner.rs")
    (153 lines read)

  [5/25] Final response:

  ## Analise do Projeto jarvis_cli

  O projeto esta bem estruturado com 3 camadas principais...
  [restante da resposta]
```

---

## 4. Context Management

### 4.1 Problema do context window

Em um loop agentico, o contexto cresce rapidamente:
- Cada tool call adiciona ~100-500 tokens
- Cada tool result pode adicionar milhares de tokens (conteudo de arquivo)
- Apos 10 iteracoes, o contexto pode ter 50k+ tokens

### 4.2 Estrategias de compactacao

```rust
pub enum ContextStrategy {
    /// Truncar resultados longos de tools.
    TruncateToolResults {
        max_chars_per_result: usize,  // default: 4000
    },
    /// Resumir historico antigo via LLM.
    SummarizeOlderMessages {
        keep_last_n: usize,  // default: 5 messages
    },
    /// Sliding window: descartar mensagens mais antigas.
    SlidingWindow {
        max_messages: usize,  // default: 20
    },
    /// Combinacao de todas as estrategias.
    Hybrid,
}
```

### 4.3 Truncamento de tool results

```rust
fn truncate_tool_output(output: &str, max_chars: usize) -> String {
    if output.len() <= max_chars {
        return output.to_string();
    }
    let half = max_chars / 2;
    format!(
        "{}\n\n... ({} characters truncated) ...\n\n{}",
        &output[..half],
        output.len() - max_chars,
        &output[output.len() - half..],
    )
}
```

---

## 5. Integracao com o TUI existente

O `AgentLoop` deve substituir (ou complementar) o fluxo atual do `chatwidget.rs`
onde o modelo e chamado uma unica vez por turno.

### Ponto de integracao

```rust
// Em chatwidget.rs, no handler de submit:
// ANTES:
//   self.send_to_model(user_input).await;
//
// DEPOIS:
//   let agent = AgentLoop::new(llm_client, tool_registry, config);
//   agent.run(user_input, &mut history, |event| {
//       self.handle_agent_event(event);
//   }, cancel_token).await;
```

### Coexistencia com modo legado

Para modelos que ja suportam tool calling nativo (via Responses API),
manter o comportamento atual. O `AgentLoop` client-side e ativado apenas
quando:
1. O modelo nao suporta tools nativamente, OU
2. O usuario opta por usar o loop client-side (via config)

```toml
# config.toml
[tui]
agentic_mode = "auto"  # "auto" | "client" | "model"
# auto: detecta baseado no modelo
# client: sempre usa AgentLoop client-side
# model: sempre delega ao modelo (comportamento atual)
```

---

## 6. Diferenca entre Daemon Loop e TUI Loop

| Aspecto | Daemon Loop | TUI Agent Loop |
|---------|-------------|----------------|
| Trigger | Scheduler (cron) | Input do usuario |
| Tools | Pipelines (scrape, LLM, publish) | Shell, files, git, web |
| Approval | Auto-approve low-risk | Depende da policy |
| Context | Dados do banco (metrics, goals) | Historico de chat + tool results |
| Feedback | Revenue + metrics | Resposta ao usuario |
| Max duration | Minutos-horas | Segundos-minutos |
| Persistence | SQLite (jobs, content) | Session rollout (JSONL) |

O daemon ja tem seu proprio loop agentico (scheduler → pipeline → feedback).
O TUI `AgentLoop` e complementar — foca na interacao humano-agente.

---

## 7. Testes

| Teste | Tipo | Descricao |
|-------|------|-----------|
| `test_loop_single_tool_call` | Unitario | Modelo chama 1 tool, recebe resultado, responde |
| `test_loop_multi_tool_calls` | Unitario | Modelo chama 3 tools em sequencia |
| `test_loop_max_iterations` | Unitario | Para apos max_iterations |
| `test_loop_timeout` | Unitario | Para apos max_duration |
| `test_loop_cancel` | Unitario | Usuario cancela via CancellationToken |
| `test_loop_no_tools_needed` | Unitario | Modelo responde direto sem tools |
| `test_context_truncation` | Unitario | Tool results truncados quando longos |
| `test_context_summarization` | Unitario | Contexto resumido quando ultrapassa limite |
| `test_approval_flow` | Integracao | Tool high-risk pausa e espera aprovacao |
| `test_text_based_fallback` | Unitario | Modelo sem function calling usa text-based |

---

## 8. Arquivos a criar/modificar

| Arquivo | Acao | Descricao |
|---------|------|-----------|
| `core/src/agent_loop/mod.rs` | **Criar** | AgentLoop struct + config |
| `core/src/agent_loop/context.rs` | **Criar** | Context management (truncate, summarize) |
| `core/src/agent_loop/events.rs` | **Criar** | AgentEvent enum |
| `tui/src/chatwidget.rs` | Modificar | Integrar AgentLoop no submit handler |
| `tui/src/agent_status.rs` | **Criar** | Widget de status do loop (iteration, tool) |
| `core/src/config/mod.rs` | Modificar | Adicionar `agentic_mode` config |

---

## 9. Estimativa

- **Complexidade**: Alta
- **Tempo estimado**: 3-5 dias
- **Risco**: Medio
- **Prerequisito**: Tool Calling Nativo (G4) deve estar implementado
- **Dependente dele**: Nenhum (mas melhora significativamente a experiencia)
