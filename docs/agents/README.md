# Agents — Sistema de Agentes Inteligentes do Jarvis

**Ultima atualizacao**: 2026-02-13

---

## 1. Visao Geral

O Jarvis utiliza um sistema de **agentes especializados** (Agents) que permite delegar diferentes tipos de tarefas para roles otimizados, cada um com seu proprio modelo LLM, instrucoes e nivel de autonomia.

O sistema de Agents e composto por 4 pilares:

```
┌───────────────────────────────────────────────────────────────────┐
│                     AGENTS SYSTEM                                  │
│                                                                    │
│  ┌────────────────┐  ┌────────────────┐  ┌─────────────────────┐  │
│  │ Agent Roles    │  │ Model          │  │ Agent Registry      │  │
│  │                │  │ Selection      │  │                     │  │
│  │ Planner        │  │                │  │ Matching automatico │  │
│  │ Developer      │  │ Cascata:       │  │ Scoring por query   │  │
│  │ Reviewer       │  │ 1. Role model  │  │ Metricas de uso     │  │
│  │ Explorer       │  │ 2. Env var     │  │ Custom agents       │  │
│  │ Worker         │  │ 3. Config.toml │  │                     │  │
│  │ Orchestrator   │  │ 4. Default     │  │                     │  │
│  └────────────────┘  └────────────────┘  └─────────────────────┘  │
│                                                                    │
│  ┌────────────────┐  ┌────────────────────────────────────────┐   │
│  │ Agent          │  │ Autonomy Integration                   │   │
│  │ Analytics      │  │                                        │   │
│  │                │  │ Agentic Loop (G5)                      │   │
│  │ Tool usage     │  │ Tool Calling Nativo (G4)               │   │
│  │ Success rates  │  │ Intent Detection → Role Selection      │   │
│  │ Chain patterns │  │ Proposal Executor (G1)                 │   │
│  │ Decisions      │  │ Goal System (G2)                       │   │
│  └────────────────┘  └────────────────────────────────────────┘   │
│                                                                    │
└───────────────────────────────────────────────────────────────────┘
```

---

## 2. Documentacao por Componente

### 2.1 Agent Roles (Implementado)

Os roles definem **quem** o agente e e **como** ele se comporta.

| Role | Descricao | Status |
|------|-----------|--------|
| **Default** | Herda configuracao do pai | Implementado |
| **Planner** | Analise estrategica e criacao de planos | Implementado |
| **Developer** | Implementacao de codigo | Implementado |
| **Reviewer** | Revisao de qualidade | Implementado |
| **Explorer** | Exploracao rapida do codebase | Implementado |
| **Worker** | Execucao paralela de tarefas | Implementado |
| **Orchestrator** | Coordenacao multi-agent | Experimental |

**Uso via CLI:**
```bash
jarvis --agent-role planner "Crie um plano de refatoracao"
jarvis --agent-role developer "Implemente o endpoint de login"
jarvis --agent-role reviewer "Revise a implementacao de auth"
```

**Documentacao detalhada**: [`jarvis-rs/docs/features/jarvis-agents.md`](../../jarvis-rs/docs/features/jarvis-agents.md)

**Codigo-fonte**:
- `jarvis-rs/core/src/agent/role.rs` — Enum AgentRole, AgentProfile
- `jarvis-rs/core/templates/agents/*.md` — Prompts por role

---

### 2.2 Selecao de Modelos por Role (Parcialmente implementado)

Cada role pode usar um **modelo LLM diferente**, otimizado para sua funcao.

| Role | Modelo sugerido (free) | Justificativa |
|------|----------------------|---------------|
| Planner | `deepseek-r1` | Raciocinio profundo |
| Developer | `gemma-3-27b` | Equilibrio velocidade/qualidade |
| Reviewer | `deepseek-r1` | Analise critica |
| Explorer | `nemotron-nano-9b` | Velocidade |
| FastChat | `step-3.5-flash` | Minima latencia |

**Cascata de resolucao**: Role override → Env var → Config.toml profile → Provider default

**Documentacao detalhada**: [`docs/AGENT_MODEL_SELECTION_ARCHITECTURE.md`](../AGENT_MODEL_SELECTION_ARCHITECTURE.md)

**Codigo-fonte**:
- `jarvis-rs/core/src/agent/role.rs` — `apply_to_config()`
- `jarvis-rs/core/src/config/profile.rs` — ConfigProfile
- `.env.example` — Variaveis `{PROVIDER}_MODEL_{ROLE}`

---

### 2.3 Agent Registry (Planejado)

O Registry adiciona **matching automatico**: dado o contexto do usuario, o sistema escolhe o agent ideal.

| Componente | Funcao |
|------------|--------|
| Registry | Registro centralizado de agents disponiveis |
| Matcher | Scoring por keywords, categoria, historico |
| Executor | Carrega agent, aplica instrucoes, chama LLM |
| Templates | Agents definidos via Markdown em `~/.jarvis/agents/` |

**Relacao com Roles**: Os roles atuais (`--agent-role`) sao a **base**. O Registry adiciona uma camada de matching e metricas **sobre** esses roles, permitindo:
- Selecao automatica (sem `--agent-role` explicito)
- Agents customizados pelo usuario
- A/B testing entre agents

**Documentacao detalhada**: [`docs/features/agents-registry.md`](../features/agents-registry.md)

---

### 2.4 Agent Analytics (Implementado)

Analytics rastreia como os agents e tools sao usados, permitindo melhoria continua.

| Metrica | Descricao |
|---------|-----------|
| Tool usage stats | Contagem de uso por ferramenta |
| Success rates | Taxa de sucesso por ferramenta |
| Average durations | Tempo medio de execucao |
| Chain patterns | Sequencias comuns de ferramentas |
| Decision stats | Padroes de aprovacao/negacao |

**Documentacao detalhada**: [`docs/features/agent-analytics.md`](../features/agent-analytics.md)

**Codigo-fonte**:
- `jarvis-rs/state/src/analytics.rs` — Queries analiticas
- `jarvis-rs/otel/src/metrics/agent_metrics.rs` — Metricas OpenTelemetry

---

## 3. Integracao com Autonomia

Os Agents sao parte central da estrategia de **autonomia** do Jarvis. Veja o [Autonomy Roadmap](../architecture/autonomy-roadmap.md) para o plano completo.

### Fluxo futuro completo

```
Input do usuario
      │
      ▼
Intent Detection ──── Detecta tipo da tarefa (plan, develop, review, explore)
      │
      ▼
Role Selection ────── Mapeia intent → AgentRole
      │
      ▼
Model Resolution ──── Cascata: role → env → config → default
      │
      ▼
Agent Loop (G5) ───── Think → Execute → Observe → Repeat
      │
      ├── Tool Calling Nativo (G4) ── Client-side tool dispatch
      │
      ├── Sandbox Execution (G6) ──── Classificacao de risco + rollback
      │
      └── Agent Analytics ─────────── Tracking de metricas
```

### Gaps relacionados a Agents

| Gap | Componente | Status | Doc |
|-----|-----------|--------|-----|
| G4 | Tool Calling Nativo | Planejado | [tool-calling-native.md](../features/tool-calling-native.md) |
| G5 | Agentic Loop | Planejado | [agentic-loop.md](../features/agentic-loop.md) |
| G6 | Sandbox Execution | Planejado | [sandbox-execution.md](../features/sandbox-execution.md) |

---

## 4. Workflow Multi-Agent

O fluxo tipico usando multiplos agents:

```
1. Planner ──── Analisa request, cria plano estruturado
      │
2. Developer ── Implementa codigo seguindo o plano
      │
3. Reviewer ─── Revisa implementacao
      │
4. Se precisa mudancas → volta ao Developer
      │
5. Aprovado → merge/deploy
```

Para tarefas complexas, o **Orchestrator** (experimental) coordena automaticamente esse fluxo, cada sub-agent usando seu proprio modelo.

---

## 5. Indice de Documentacao

| Documento | Descricao |
|-----------|-----------|
| [jarvis-agents.md](../../jarvis-rs/docs/features/jarvis-agents.md) | Roles disponiveis e como usa-los |
| [AGENT_MODEL_SELECTION_ARCHITECTURE.md](../AGENT_MODEL_SELECTION_ARCHITECTURE.md) | Arquitetura de selecao de modelos por role |
| [agents-registry.md](../features/agents-registry.md) | Design do Agent Registry (planejado) |
| [agent-analytics.md](../features/agent-analytics.md) | Analytics de uso de agents e tools |
| [agentic-loop.md](../features/agentic-loop.md) | Loop Think→Execute→Observe (planejado) |
| [tool-calling-native.md](../features/tool-calling-native.md) | Tool calling client-side (planejado) |
| [sandbox-execution.md](../features/sandbox-execution.md) | Execucao segura com classificacao de risco |
| [autonomy-roadmap.md](../architecture/autonomy-roadmap.md) | Roadmap completo de autonomia |
