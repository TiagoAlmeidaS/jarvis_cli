# Comandos CLI Autônomos

**Data**: 2026-02-20
**Status**: Parcialmente implementado (muitos comandos já existem)
**Módulo**: `jarvis-rs/cli/src/`

## Overview

O Jarvis CLI já possui uma implementação sólida de comandos para as funcionalidades autônomas planejadas no `PLANO_IMPLEMENTACAO.md`.

Este documento descreve:
- ✅ O que já está implementado
- 🔧 O que falta implementar/validar
- 📋 Plano de ação futuro

## Comandos Já Implementados

### 1. Intent Detection (`jarvis intent`)
**Arquivo**: [`intent_cmd.rs`](../../../jarvis-rs/cli/src/intent_cmd.rs)

#### Comandos
```bash
jarvis intent detect "criar uma skill para processar CSV"
jarvis intent list
jarvis intent test
```

#### Detalhes
- Detecta 7 tipos de intent: CreateSkill, ExecuteSkill, ListSkills, Explore, Plan, AskCapabilities, NormalChat
- Suporte a thresholds de confiança
- Formatos de saída: human (colorido), json, simple
- Test suite com exemplos pré-definidos

### 2. Skills Development (`jarvis skills`)
**Arquivo**: [`skills_cmd.rs`](../../../jarvis-rs/cli/src/skills_cmd.rs)

#### Comandos
```bash
jarvis skills create <nome> --requirements="processar CSV" --language=rust
jarvis skills evaluate <skill_file>
jarvis skills list
jarvis skills test
```

### 3. Agent Operations (`jarvis agent`)
**Arquivo**: [`agent_cmd.rs`](../../../jarvis-rs/cli/src/agent_cmd.rs)

#### Comandos
```bash
jarvis agent explore <query> --path=. --thoroughness=medium
jarvis agent plan <task>
jarvis agent session list
```

### 4. Context Management (`jarvis context`)
**Arquivo**: [`context_cmd.rs`](../../../jarvis-rs/cli/src/context_cmd.rs)

#### Comandos
```bash
jarvis context add <path> --type=code
jarvis context search <query> --limit=5
jarvis context list
jarvis context compress
```

### 5. Safety Verification (`jarvis safety`)
**Arquivo**: [`safety_cmd.rs`](../../../jarvis-rs/cli/src/safety_cmd.rs)

#### Comandos
```bash
jarvis safety check <action>
jarvis safety verify <command>
jarvis safety analyze <file>
```

### 6. Autonomous Execution (`jarvis autonomous`)
**Arquivo**: [`autonomous_cmd.rs`](../../../jarvis-rs/cli/src/autonomous_cmd.rs)

#### Comandos
```bash
jarvis autonomous plan <task>
jarvis autonomous execute <plan_id>
jarvis autonomous run "tarefa natural"
```

## O Que FALTA Implementar/Validar

### GAP 1: Testes Unitários
**Status**: ❌ Não validado

- Criar testes para cada comando CLI
- Testes de smoke para verificação básica
- Integração com `cargo test -p jarvis-cli`

### GAP 2: Integração com Daemon
**Status**: ⚠️ Parcial

- Comandos que disparam jobs no daemon
- Comandos que consultam estado do daemon em tempo real
- Async execution with callbacks

### GAP 3: Documentação de Uso
**Status**: ⚠️ Incompleta

- Guia de introdução para cada comando
- Exemplos práticos de uso em workflow
- Best practices e troubleshooting

### GAP 4: Remover Flag Experimental
**Status**: 🔧 Pendente

- Validação em produção com dados reais
- Feedback de usuários
- Testes de regressão estáveis

## Plano de Ação (Priorizado)

### Sprint 1 (1 semana): Validar

| # | Tarefa | Prioridade | Esforço |
|---|--------|------------|---------|
| 1 | Criar testes de smoke | 🔴 Crítica | 2 dias |
| 2 | Documentar examples | 🟡 Média | 1 dia |
| 3 | Atualizar help text | 🟢 Baixa | 0.5 dia |

### Sprint 2 (1 semana): Integrar

| # | Tarefa | Prioridade | Esforço |
|---|--------|------------|---------|
| 5 | Integração CLI → Daemon | 🔴 Crítica | 2 dias |
| 6 | Comandos de consulta ao daemon | 🟡 Média | 1.5 dias |
| 7 | Async execution com callbacks | 🟡 Média | 2 dias |

### Sprint 3 (2 semanas): Features

| # | Tarefa | Prioridade | Esforço |
|---|--------|------------|---------|
| 9 | Autocomplete shell | 🟢 Baixa | 2 dias |
| 10 | History system | 🟡 Média | 2 dias |
| 11 | Exportadores | 🟡 Média | 3 dias |

## Checklist de Production Ready

- [ ] Testes de smoke para todos os 6 comandos
- [ ] Documentação completa em `docs/features/`
- [ ] Benchmark de performance
- [ ] Integração com daemon 100%
- [ ] Autocomplete shell funcionando
- [ ] Sem bugs críticos
- [ ] Changelog e release notes

## Referências

- [PLANO_IMPLEMENTACAO.md](../../PLANO_IMPLEMENTACAO.md)
- [AUTONOMY_IMPLEMENTATION_STATUS.md](../../AUTONOMY_IMPLEMENTATION_STATUS.md)
- [ANALISE_AUTONOMIA_JARVIS.md](../../ANALISE_AUTONOMIA_JARVIS.md)

---

**Última atualização**: 2026-02-20
