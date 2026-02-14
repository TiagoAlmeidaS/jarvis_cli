# Arquitetura de Selecao de Modelos por Agent/Role

> **Status**: Parcialmente implementado - infraestrutura pronta, integracao pendente
> **Prioridade**: Alta - essencial para operacao multi-agent eficiente
> **Ultima atualizacao**: 2026-02-13

---

## 1. Visao Geral

O Jarvis CLI possui uma arquitetura de **selecao de modelos por papel (role)**, onde cada
agente especializado (Planner, Developer, Reviewer, etc.) pode usar um modelo LLM diferente,
otimizado para sua funcao.

**Principio**: Usar o modelo certo para a tarefa certa.
- Planejamento requer raciocinio profundo → modelo mais capaz (ex: Claude Opus, GPT-4o)
- Implementacao requer velocidade e contexto grande → modelo equilibrado (ex: Llama 405B)
- Revisao requer analise critica → modelo capaz mas pode ser menor
- Chat rapido requer baixa latencia → modelo leve e rapido (ex: Gemini Flash, Haiku)
- Exploracao requer velocidade → modelo rapido com reasoning medio

---

## 2. Estado Atual da Implementacao

### 2.1 O que JA existe

#### AgentRole enum (`core/src/agent/role.rs`)
```rust
pub enum AgentRole {
    Default,        // Herda configuracao do pai
    Orchestrator,   // Coordenacao (prompts prontos, desabilitado)
    Worker,         // Execucao de tarefas
    Explorer,       // Exploracao do codebase (unico com model override)
    Planner,        // Planejamento estrategico
    Developer,      // Implementacao de codigo
    Reviewer,       // Revisao de qualidade
}
```

#### AgentProfile struct
```rust
pub struct AgentProfile {
    pub base_instructions: Option<&'static str>,  // Prompt do role
    pub model: Option<&'static str>,              // Override de modelo
    pub reasoning_effort: Option<ReasoningEffort>, // Esforco de raciocinio
    pub read_only: bool,                          // Sandbox read-only
    pub description: &'static str,                // Descricao para tool specs
}
```

#### ConfigProfile struct (`core/src/config/profile.rs`)
```rust
pub struct ConfigProfile {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub approval_policy: Option<AskForApproval>,
    pub sandbox_mode: Option<SandboxMode>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub model_reasoning_summary: Option<ReasoningSummary>,
    pub model_verbosity: Option<Verbosity>,
    pub personality: Option<Personality>,
    // ... outros campos
}
```

#### Templates de instrucoes por role
```
jarvis-rs/core/templates/agents/
  orchestrator.md   - Coordenacao e delegacao
  planner.md        - Analise e planejamento estrategico
  developer.md      - Implementacao seguindo planos
  reviewer.md       - Revisao de qualidade e corretude
```

#### Profiles no config.toml
```toml
[profiles.default]    model = "databricks-claude-opus-4-5"
[profiles.planner]    model = "databricks-claude-opus-4-5"
[profiles.developer]  model = "databricks-meta-llama-3-1-405b"
[profiles.reviewer]   model = "databricks-meta-llama-3-1-405b"
[profiles.fastchat]   model = "databricks-claude-haiku-4-5"
```

#### Variaveis de ambiente por role (`.env.example`)
```env
OPENAI_MODEL_PLANNER=gpt-4-turbo-preview
OPENAI_MODEL_DEVELOPER=gpt-4-turbo-preview
OPENAI_MODEL_REVIEWER=gpt-4
OPENAI_MODEL_FASTCHAT=gpt-3.5-turbo

DATABRICKS_MODEL_PLANNER=databricks-claude-opus-4-5
DATABRICKS_MODEL_DEVELOPER=databricks-meta-llama-3-1-405b
DATABRICKS_MODEL_REVIEWER=databricks-meta-llama-3-1-405b
DATABRICKS_MODEL_FASTCHAT=databricks-claude-haiku-4-5
```

#### Metodo apply_to_config (funcional)
```rust
pub fn apply_to_config(self, config: &mut Config) -> Result<(), String> {
    let profile = self.profile();
    if let Some(base_instructions) = profile.base_instructions {
        config.base_instructions = Some(base_instructions.to_string());
    }
    if let Some(model) = profile.model {
        config.model = Some(model.to_string());
    }
    if let Some(reasoning_effort) = profile.reasoning_effort {
        config.model_reasoning_effort = Some(reasoning_effort)
    }
    // ... sandbox policy
}
```

### 2.2 O que esta INCOMPLETO

| Componente | Estado | Detalhe |
|------------|--------|---------|
| `Explorer.model` | Hardcoded `gpt-5.2-Jarvis` | Funciona, mas modelo inexistente para nos |
| `Planner.model` | `None` | TODO no codigo, nao seleciona modelo |
| `Developer.model` | `None` | TODO no codigo, nao seleciona modelo |
| `Reviewer.model` | `None` | TODO no codigo, nao seleciona modelo |
| `Worker.model` | `None` | Instrucoes comentadas |
| `Orchestrator` | Desabilitado | Comentado em `ALL_ROLES` |
| Intent → Role mapping | Nao existe | Intent detection e role system sao separados |
| Profile auto-selection | Nao existe | Usuario deve usar `-p` manualmente |
| Env vars por role | Definidas mas nao lidas | `*_MODEL_PLANNER` etc. nao sao usadas no Rust |

### 2.3 O que NAO existe ainda

- **Selecao automatica de role** baseada no tipo de tarefa detectada
- **Fallback de modelo** quando o modelo preferido nao esta disponivel
- **Agent Registry** com matching automatico (design doc existe em `docs/features/agents-registry.md`)
- **Leitura das env vars** `*_MODEL_PLANNER`, `*_MODEL_DEVELOPER` etc. no codigo Rust

---

## 3. Arquitetura Proposta (Implementacao Futura)

### 3.1 Fluxo Completo

```
                         Entrada do Usuario
                               |
                               v
                    +---------------------+
                    | Intent Detection    |
                    | (detecta tipo da    |
                    |  tarefa)            |
                    +---------------------+
                               |
                    Tipos: plan | develop | review | explore | chat
                               |
                               v
                    +---------------------+
                    | Role Selection      |
                    | (mapeia intent para |
                    |  AgentRole)         |
                    +---------------------+
                               |
                               v
                    +---------------------+
                    | Model Resolution    |  Cascata de resolucao:
                    | (resolve modelo     |  1. AgentProfile.model (role override)
                    |  efetivo)           |  2. Env var (*_MODEL_PLANNER)
                    +---------------------+  3. ConfigProfile (config.toml)
                               |             4. Provider default
                               v
                    +---------------------+
                    | Config Application  |
                    | apply_to_config()   |
                    | - model             |
                    | - instructions      |
                    | - reasoning_effort  |
                    | - sandbox_policy    |
                    +---------------------+
                               |
                               v
                    +---------------------+
                    | LLM Execution       |
                    | (usa modelo e       |
                    |  provider corretos) |
                    +---------------------+
```

### 3.2 Mapeamento Intent → Role

```rust
// Proposta de implementacao
fn intent_to_role(intent: &Intent) -> AgentRole {
    match intent {
        Intent::Plan | Intent::Analyze    => AgentRole::Planner,
        Intent::Implement | Intent::Fix   => AgentRole::Developer,
        Intent::Review | Intent::Audit    => AgentRole::Reviewer,
        Intent::Explore | Intent::Search  => AgentRole::Explorer,
        Intent::Chat | Intent::Question   => AgentRole::Default,
        Intent::Complex                   => AgentRole::Orchestrator,
    }
}
```

### 3.3 Resolucao de Modelo com Cascata

```rust
// Proposta: cascata de resolucao de modelo por role
fn resolve_model_for_role(
    role: AgentRole,
    provider_id: &str,
    config: &Config,
) -> Option<String> {
    // 1. Override explicito no AgentProfile
    if let Some(model) = role.profile().model {
        return Some(model.to_string());
    }

    // 2. Variavel de ambiente por provider + role
    //    Ex: OPENROUTER_MODEL_PLANNER, DATABRICKS_MODEL_DEVELOPER
    let env_key = format!(
        "{}_MODEL_{}",
        provider_id.to_uppercase().replace('-', "_"),
        role_to_env_suffix(role)
    );
    if let Ok(model) = std::env::var(&env_key) {
        if !model.trim().is_empty() {
            return Some(model);
        }
    }

    // 3. Profile no config.toml
    let profile_name = role_to_profile_name(role);
    if let Some(profile) = config.profiles.get(profile_name) {
        if let Some(model) = &profile.model {
            return Some(model.clone());
        }
    }

    // 4. Modelo padrao do provider (nenhum override)
    None
}

fn role_to_env_suffix(role: AgentRole) -> &'static str {
    match role {
        AgentRole::Planner     => "PLANNER",
        AgentRole::Developer   => "DEVELOPER",
        AgentRole::Reviewer    => "REVIEWER",
        AgentRole::Explorer    => "EXPLORER",
        AgentRole::Default     => "DEFAULT",
        AgentRole::Worker      => "WORKER",
        AgentRole::Orchestrator => "ORCHESTRATOR",
    }
}

fn role_to_profile_name(role: AgentRole) -> &'static str {
    match role {
        AgentRole::Planner     => "planner",
        AgentRole::Developer   => "developer",
        AgentRole::Reviewer    => "reviewer",
        AgentRole::Explorer    => "explorer",
        AgentRole::Default     => "default",
        AgentRole::Worker      => "worker",
        AgentRole::Orchestrator => "orchestrator",
    }
}
```

### 3.4 Configuracao Proposta (config.toml)

```toml
# ============================================================================
# MAPEAMENTO DE MODELOS POR ROLE
# ============================================================================
# Cada role pode ter seu proprio provider e modelo.
# Se nao especificado, herda do profile "default".

[roles.planner]
model_provider = "openrouter"
model = "deepseek/deepseek-r1-0528:free"     # Raciocinio profundo
reasoning_effort = "high"

[roles.developer]
model_provider = "openrouter"
model = "google/gemma-3-27b-it:free"         # Bom equilibrio
reasoning_effort = "medium"

[roles.reviewer]
model_provider = "openrouter"
model = "deepseek/deepseek-r1-0528:free"     # Analise critica
reasoning_effort = "high"

[roles.explorer]
model_provider = "openrouter"
model = "nvidia/nemotron-nano-9b-v2:free"    # Rapido
reasoning_effort = "low"

[roles.fastchat]
model_provider = "google"
model = "gemini-2.5-flash-lite"              # Minimo de latencia
reasoning_effort = "low"
```

---

## 4. Sugestao de Modelos por Role (Free Tier)

### 4.1 OpenRouter (Gratuitos - Testados em 2026-02-13)

| Role | Modelo Sugerido | Justificativa |
|------|----------------|---------------|
| **Planner** | `deepseek/deepseek-r1-0528:free` | Melhor raciocinio, pensamento profundo |
| **Developer** | `google/gemma-3-27b-it:free` | Equilibrio entre qualidade e velocidade |
| **Reviewer** | `deepseek/deepseek-r1-0528:free` | Analise critica detalhada |
| **Explorer** | `nvidia/nemotron-nano-9b-v2:free` | Rapido, suficiente para busca |
| **FastChat** | `stepfun/step-3.5-flash:free` | Minima latencia |
| **Fallback** | `openrouter/free` | Auto-roteamento para modelo disponivel |

### 4.2 Google AI Studio (Free Tier)

| Role | Modelo | Nota |
|------|--------|------|
| **Planner** | `gemini-2.5-flash` | Mais capaz, com thinking |
| **Developer** | `gemini-2.5-flash-lite` | Rapido e funcional |
| **Reviewer** | `gemini-2.5-flash` | Analise de qualidade |
| **FastChat** | `gemini-2.5-flash-lite` | Minima latencia |

### 4.3 Pagos (Referencia de Qualidade)

| Role | Modelo | Provider |
|------|--------|----------|
| **Planner** | `claude-opus-4-5` / `o3` | Anthropic / OpenAI |
| **Developer** | `claude-sonnet-4-5` / `gpt-4o` | Anthropic / OpenAI |
| **Reviewer** | `claude-opus-4-5` / `o3-mini` | Anthropic / OpenAI |
| **Explorer** | `claude-haiku-4-5` / `gpt-4o-mini` | Anthropic / OpenAI |
| **FastChat** | `gemini-2.5-flash-lite` | Google |

---

## 5. Tarefas de Implementacao

### Fase 1: Conectar AgentProfile.model aos profiles do config.toml
**Arquivos**: `core/src/agent/role.rs`, `core/src/config/`

- [ ] Modificar `AgentRole::profile()` para aceitar `Config` como parametro
- [ ] Ler modelo do profile correspondente no config.toml (cascata)
- [ ] Ler variaveis de ambiente `{PROVIDER}_MODEL_{ROLE}`
- [ ] Remover hardcode do `EXPLORER_MODEL` (`gpt-5.2-Jarvis`)
- [ ] Adicionar testes para resolucao de modelo

### Fase 2: Adicionar secao `[roles.*]` no config.toml
**Arquivos**: `core/src/config/`, `config.toml.example`

- [ ] Criar struct `RoleConfig` com model, model_provider, reasoning_effort
- [ ] Parsear secao `[roles]` do config.toml
- [ ] Integrar com `apply_to_config()`
- [ ] Documentar no config.toml.example

### Fase 3: Mapear Intent Detection → AgentRole
**Arquivos**: `core/src/intent/`, `core/src/agent/role.rs`

- [ ] Criar funcao `intent_to_role()` mapeando intents para roles
- [ ] Integrar no fluxo principal (codex.rs ou jarvis_thread.rs)
- [ ] Adicionar selecao automatica de role quando nao especificado
- [ ] Manter override manual via `-p` flag

### Fase 4: Fallback e resiliencia
**Arquivos**: `core/src/model_provider_info.rs`, `core/src/agent/role.rs`

- [ ] Implementar fallback de modelo quando o preferido falha (429, etc.)
- [ ] Cascata: role model → profile model → default model
- [ ] Retry com modelo alternativo em caso de rate limit
- [ ] Log de qual modelo foi efetivamente usado

### Fase 5: Orquestrador multi-agent
**Arquivos**: `core/src/agent/role.rs`, templates

- [ ] Habilitar `AgentRole::Orchestrator` em `ALL_ROLES`
- [ ] Implementar fluxo: Orchestrator → Planner → Developer → Reviewer
- [ ] Cada sub-agent usa seu proprio modelo via role config
- [ ] Coordenacao de resultados entre agents

---

## 6. Arquivos-Chave de Referencia

| Arquivo | Descricao |
|---------|-----------|
| `jarvis-rs/core/src/agent/role.rs` | Enum AgentRole, AgentProfile, apply_to_config() |
| `jarvis-rs/core/src/config/profile.rs` | ConfigProfile struct |
| `jarvis-rs/core/templates/agents/*.md` | Prompts por role (planner, developer, reviewer, orchestrator) |
| `config.toml.example` | Profiles de configuracao |
| `.env.example` | Variaveis de modelo por role |
| `jarvis-rs/core/src/model_provider_info.rs` | Registry de providers (inclui Google, OpenRouter) |
| `jarvis-rs/core/src/intent/` | Intent detection (separado do role system) |
| `jarvis-rs/core/src/autonomous/` | Autonomous agent engine |
| `docs/features/agents-registry.md` | Design doc do Agent Registry (planejado) |
| `docs/agents/README.md` | Pagina central unificada do sistema de agents |
| `docs/features/agent-analytics.md` | Analytics de uso de agents e tools |
| `docs/architecture/autonomy-roadmap.md` | Roadmap de autonomia (G1-G6) |

---

## 7. Exemplo de Uso Futuro

```bash
# Selecao automatica (intent detection escolhe o role e modelo)
jarvis chat "Analise o projeto e crie um plano de refatoracao"
# → Detecta intent=Plan → Role=Planner → Modelo=deepseek-r1

# Override manual de profile
jarvis chat -p developer "Implemente a funcao de login"
# → Role=Developer → Modelo do profile developer no config.toml

# Override manual de modelo (maximo controle)
jarvis chat -p planner -m "anthropic/claude-3.5-sonnet" "Planeje a arquitetura"
# → Role=Planner, Modelo=claude-3.5-sonnet (override explicito)

# Fluxo multi-agent orquestrado
jarvis autonomous "Implemente autenticacao JWT"
# → Orchestrator detecta complexidade
# → Spawna Planner (deepseek-r1) → cria plano
# → Spawna Developer (gemma-27b) → implementa
# → Spawna Reviewer (deepseek-r1) → revisa
# → Ciclo ate aprovacao
```
