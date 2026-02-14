# Features - Jarvis CLI Rust

Este diretório contém documentação técnica detalhada para funcionalidades recomendadas para o Jarvis CLI Rust, baseadas nas funcionalidades existentes no Jarvis.CLI .NET.

## 📋 Índice de Features

### Integrações com Serviços Externos

| Feature | Status | Prioridade | Documentação |
|---------|--------|------------|--------------|
| [Visão Geral das Integrações](./integrations-overview.md) | 📝 Planejado | 🔴 Alta | [integrations-overview.md](./integrations-overview.md) |
| [Integração Qdrant](./qdrant-integration.md) | 📝 Planejado | 🔴 Alta | [qdrant-integration.md](./qdrant-integration.md) |
| [Integração Redis](./redis-integration.md) | 📝 Planejado | 🔴 Alta | [redis-integration.md](./redis-integration.md) |
| [Integração SQL Server](./sqlserver-integration.md) | 📝 Planejado | 🔴 Alta | [sqlserver-integration.md](./sqlserver-integration.md) |

### Autonomia & Agente Inteligente

> Roadmap completo: [docs/architecture/autonomy-roadmap.md](../architecture/autonomy-roadmap.md)  
> Pagina central de Agents: [docs/agents/README.md](../agents/README.md)

| Feature | Status | Prioridade | Gap | Documentação |
|---------|--------|------------|-----|--------------|
| [Proposal Executor](./proposal-executor.md) | ✅ Implementado | 🔴 CRITICA | G1 | [proposal-executor.md](./proposal-executor.md) |
| [Goal System](./goal-system.md) | ✅ Implementado | 🔴 CRITICA | G2 | [goal-system.md](./goal-system.md) |
| [Real Data Integration](./real-data-integration.md) | ✅ Implementado (CLI manual) | 🔴 CRITICA | G3 | [real-data-integration.md](./real-data-integration.md) |
| [Tool Calling Nativo](./tool-calling-native.md) | ✅ Implementado | 🔴 Alta | G4 | [tool-calling-native.md](./tool-calling-native.md) |
| [Agentic Loop](./agentic-loop.md) | ✅ Implementado (core) | 🔴 Alta | G5 | [agentic-loop.md](./agentic-loop.md) |
| [Sandbox Execution](./sandbox-execution.md) | ✅ Implementado (parcial) | 🔴 Alta | G6 | [sandbox-execution.md](./sandbox-execution.md) |

### Automação & Monetização

| Feature | Status | Prioridade | Documentação |
|---------|--------|------------|--------------|
| [Daemon Automation](./daemon-automation.md) | 🚧 Em Progresso | 🔴 Alta | [daemon-automation.md](./daemon-automation.md) |
| [Daemon Feedback Loop](./daemon-feedback-loop.md) | ✅ Implementado | 🔴 Alta | [daemon-feedback-loop.md](./daemon-feedback-loop.md) |
| [Yolo Mode](./yolo-mode.md) | ✅ Implementado | 🟡 Média | [yolo-mode.md](./yolo-mode.md) |

### Features Principais

| Feature | Status | Prioridade | Documentação |
|---------|--------|------------|--------------|
| [Sistema de Skills](./skills-system.md) | 📝 Planejado | 🔴 Alta | [skills-system.md](./skills-system.md) |
| [RAG e Context Management](./rag-context-management.md) | 📝 Planejado | 🔴 Alta | [rag-context-management.md](./rag-context-management.md) |
| [Sistema Undo/Redo](./undo-redo-system.md) | 📝 Planejado | 🟡 Média | [undo-redo-system.md](./undo-redo-system.md) |
| [Integração GitHub](./github-integration.md) | 📝 Planejado | 🟡 Média | [github-integration.md](./github-integration.md) |
| [Agents Registry](./agents-registry.md) | 📝 Planejado | 🟡 Média | [agents-registry.md](./agents-registry.md) |
| [Sistema de Testes Automático](./auto-testing.md) | 📝 Planejado | 🟡 Média | [auto-testing.md](./auto-testing.md) |
| [Refatoração Avançada](./advanced-refactoring.md) | 📝 Planejado | 🟢 Baixa | [advanced-refactoring.md](./advanced-refactoring.md) |
| [Planning Engine](./planning-engine.md) | 📝 Planejado | 🟢 Baixa | [planning-engine.md](./planning-engine.md) |
| [Memory Tools](./memory-tools.md) | 📝 Planejado | 🟢 Baixa | [memory-tools.md](./memory-tools.md) |
| [Composer Orchestration](./composer-orchestration.md) | 📝 Planejado | 🟢 Baixa | [composer-orchestration.md](./composer-orchestration.md) |

## 🎯 Objetivo

Estes documentos descrevem funcionalidades que existem no projeto Jarvis.CLI (.NET) e que podem ser implementadas no Jarvis CLI (Rust) para melhorar suas capacidades. Cada documento inclui:

- **Visão Geral**: Descrição da funcionalidade e sua importância
- **Motivação**: Por que essa funcionalidade é necessária
- **Arquitetura**: Componentes principais e fluxo de dados
- **Especificação Técnica**: APIs, estruturas de dados e algoritmos propostos
- **Comandos CLI**: Interface de linha de comando
- **Exemplos de Uso**: Casos de uso práticos
- **Considerações de Implementação**: Dependências, desafios técnicos, performance e segurança
- **Roadmap de Implementação**: Fases sugeridas para implementação

## 📊 Status das Features

### 🔴 CRITICA — Fechar o Loop Autonomo

Gaps que impedem o Jarvis de funcionar como agente autonomo:

1. **[Proposal Executor](./proposal-executor.md)** (G1) — ✅ Executa propostas aprovadas automaticamente
2. **[Goal System](./goal-system.md)** (G2) — ✅ Metas mensuraveis com CLI completo
3. **[Real Data Integration](./real-data-integration.md)** (G3) — 🟡 Input manual via CLI implementado, APIs externas pendentes

### 🔴 Alta Prioridade — Empoderar o TUI

Capacidades que tornam o TUI tao capaz quanto Claude Code / Cursor:

4. **[Tool Calling Nativo](./tool-calling-native.md)** (G4) — Tools executadas client-side, independente do modelo
5. **[Agentic Loop](./agentic-loop.md)** (G5) — Loop think-execute-observe-repeat completo
6. **[Sandbox Execution](./sandbox-execution.md)** (G6) — Execucao segura com classificacao de risco e rollback

### 🔴 Alta Prioridade — Infraestrutura

7. **Integrações com Serviços Externos** - Qdrant, Redis e SQL Server para produção
   - [Visão Geral das Integrações](./integrations-overview.md)
   - [Integração Qdrant](./qdrant-integration.md) - Vector database para RAG
   - [Integração Redis](./redis-integration.md) - Cache distribuído
   - [Integração SQL Server](./sqlserver-integration.md) - Persistência de dados
8. **Sistema de Skills** - Base para extensibilidade e reutilização de funcionalidades
9. **RAG e Context Management** - Melhora significativamente a qualidade das respostas do LLM

### 🟡 Média Prioridade

Funcionalidades importantes que melhoram a experiência do usuário:

3. **Sistema Undo/Redo** - Segurança e confiança ao trabalhar com código
4. **Integração GitHub** - Verificar se já existe implementação básica
5. **Agents Registry** - Sistema de agents especializados
6. **Sistema de Testes Automático** - Validação automática de mudanças

### 🟢 Baixa Prioridade

Funcionalidades avançadas que podem ser implementadas posteriormente:

7. **Refatoração Avançada** - Complexo, requer integração com rust-analyzer
8. **Planning Engine** - Sistema complexo de planejamento de ações
9. **Memory Tools** - Sistema de memória persistente
10. **Composer Orchestration** - Orquestração de múltiplas ações

## 🔍 Como Usar Esta Documentação

1. **Para Desenvolvedores**: Use estes documentos como especificação técnica para implementação
2. **Para Arquitetos**: Use para entender o design e arquitetura proposta
3. **Para Product Owners**: Use para priorizar features e planejar releases
4. **Para Contribuidores**: Use como guia para contribuir com novas funcionalidades

## 📝 Convenções

- **Status**: 📝 Planejado | 🚧 Em Progresso | ✅ Implementado | ⚠️ Deprecado
- **Prioridade**: 🔴 Alta | 🟡 Média | 🟢 Baixa
- Cada documento segue um template padronizado para facilitar navegação

## 🔗 Referências

- [Jarvis.CLI .NET](../jarvis-cli-dotnet-reference.md) - Código fonte de referência
- [Documentação Principal](../README.md) - Documentação geral do projeto
- [Contribuindo](../contributing.md) - Guia de contribuição

---

**Última atualização**: 2026-02-13
**Versão**: 2.1.0
