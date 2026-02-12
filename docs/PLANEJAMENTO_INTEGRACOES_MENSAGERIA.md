# Planejamento: Integrações WhatsApp e Telegram

## 📋 Situação Atual

### Estado dos Crates
- ✅ **Crates criados** em `codex-rs/`:
  - `codex-messaging` (crate comum)
  - `codex-whatsapp` (integração WhatsApp)
  - `codex-telegram` (integração Telegram)

### Problemas Identificados
1. ❌ Crates **não estão registrados** no workspace principal (`jarvis-rs/Cargo.toml`)
2. ❌ Dependências usam nomes antigos (`codex-core`, `codex-common`) ao invés de (`jarvis-core`, `jarvis-common`)
3. ❌ Crates estão em `codex-rs/` mas o workspace principal está em `jarvis-rs/`
4. ❌ **Não há integração** com o sistema core do Jarvis
5. ❌ **Servidores webhook não estão implementados** completamente
6. ❌ **Falta handler** que conecta mensagens ao sistema de tools do Jarvis

## 🎯 Objetivos

### Funcionalidades Principais
1. **Receber mensagens** de WhatsApp e Telegram via webhooks
2. **Enviar respostas** através das plataformas
3. **Executar comandos do Jarvis** via mensagens de texto
4. **Manter contexto** de conversas
5. **Suporte a múltiplas conversas** simultâneas

### Funcionalidades Secundárias (Futuro)
- Suporte a mídia (imagens, documentos, áudio)
- Histórico persistente de conversas
- Comandos personalizados
- Rate limiting inteligente

## 🏗️ Arquitetura Proposta

```
┌─────────────────────────────────────────────────────────────┐
│                    Jarvis Core                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Tools      │  │  Messaging   │  │   Context    │      │
│  │  Registry    │  │   Manager    │  │   Manager    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
┌────────▼────────┐  ┌────────▼────────┐  ┌────────▼────────┐
│ jarvis-whatsapp │  │ jarvis-telegram │  │ jarvis-messaging │
│                 │  │                 │  │   (common)       │
│ ┌────────────┐  │  │ ┌────────────┐  │  │                  │
│ │  Webhook   │  │  │ │  Webhook   │  │  │ ┌──────────────┐ │
│ │  Server    │  │  │ │  Server    │  │  │ │  Protocol    │ │
│ └────────────┘  │  │ └────────────┘  │  │ │  Handler    │ │
│ ┌────────────┐  │  │ ┌────────────┐  │  │ └──────────────┘ │
│ │   Client   │  │  │ │   Client   │  │  │ ┌──────────────┐ │
│ │   API      │  │  │ │   API      │  │  │ │  Message     │ │
│ └────────────┘  │  │ └────────────┘  │  │ │  Router     │ │
└─────────────────┘  └─────────────────┘  │ └──────────────┘ │
                                          └──────────────────┘
```

## 📝 Plano de Implementação

### Fase 1: Reestruturação e Migração (Prioridade ALTA)

#### 1.1 Mover e Renomear Crates
- [ ] Mover crates de `codex-rs/` para `jarvis-rs/`
- [ ] Renomear crates:
  - `codex-messaging` → `jarvis-messaging`
  - `codex-whatsapp` → `jarvis-whatsapp`
  - `codex-telegram` → `jarvis-telegram`
- [ ] Atualizar todos os imports e referências

#### 1.2 Atualizar Dependências
- [ ] Substituir `codex-core` por `jarvis-core`
- [ ] Substituir `codex-common` por `jarvis-common`
- [ ] Verificar e atualizar outras dependências do workspace

#### 1.3 Registrar no Workspace
- [ ] Adicionar crates ao `jarvis-rs/Cargo.toml`:
  ```toml
  "messaging",
  "whatsapp",
  "telegram",
  ```
- [ ] Adicionar dependências no `[workspace.dependencies]`:
  ```toml
  jarvis-messaging = { path = "messaging" }
  jarvis-whatsapp = { path = "whatsapp" }
  jarvis-telegram = { path = "telegram" }
  ```

### Fase 2: Integração com Core (Prioridade ALTA)

#### 2.1 Criar Handler de Mensagens
- [ ] Criar `jarvis-core/src/messaging/` module
- [ ] Implementar `MessageToCodexHandler` que:
  - Recebe mensagens das plataformas
  - Converte para formato interno do Jarvis
  - Roteia para sistema de tools
  - Formata respostas para envio

#### 2.2 Integrar com Sistema de Tools
- [ ] Criar tool handler para comandos de mensageria
- [ ] Implementar parser de comandos (ex: `/exec`, `/read`)
- [ ] Conectar com `ToolRouter` existente
- [ ] Tratar erros e formatar respostas

#### 2.3 Gerenciamento de Contexto
- [ ] Integrar `ConversationManager` com sistema de contexto do Jarvis
- [ ] Persistir contexto de conversas
- [ ] Implementar recuperação de histórico

### Fase 3: Implementação de Webhooks (Prioridade MÉDIA)

#### 3.1 WhatsApp Webhook
- [ ] Completar implementação do servidor webhook
- [ ] Implementar validação de `verify_token`
- [ ] Adicionar parsing completo de payloads WhatsApp
- [ ] Implementar tratamento de diferentes tipos de mensagem
- [ ] Adicionar rate limiting

#### 3.2 Telegram Webhook
- [ ] Completar implementação do servidor webhook
- [ ] Implementar validação HMAC-SHA256
- [ ] Adicionar parsing completo de payloads Telegram
- [ ] Implementar tratamento de diferentes tipos de mensagem
- [ ] Adicionar rate limiting

#### 3.3 Servidor Unificado
- [ ] Criar servidor HTTP unificado para ambos webhooks
- [ ] Configurar rotas separadas (`/whatsapp/webhook`, `/telegram/webhook`)
- [ ] Implementar middleware de segurança
- [ ] Adicionar logging e monitoramento

### Fase 4: Configuração e Setup (Prioridade MÉDIA)

#### 4.1 Configuração no Core
- [ ] Adicionar seção `[messaging]` ao `Config`:
  ```toml
  [messaging]
  enabled = true
  
  [messaging.whatsapp]
  enabled = true
  api_url = "..."
  access_token = "..."
  verify_token = "..."
  webhook_port = 8080
  phone_number_id = "..."
  
  [messaging.telegram]
  enabled = true
  bot_token = "..."
  webhook_url = "..."
  webhook_port = 8081
  ```
- [ ] Atualizar `config.schema.json`
- [ ] Adicionar validação de configuração

#### 4.2 Variáveis de Ambiente
- [ ] Documentar variáveis necessárias
- [ ] Criar `.env.example` atualizado
- [ ] Implementar carregamento seguro de credenciais

### Fase 5: Testes e Validação (Prioridade MÉDIA)

#### 5.1 Testes Unitários
- [ ] Testes para parsing de mensagens
- [ ] Testes para clientes de API
- [ ] Testes para gerenciamento de conversas
- [ ] Testes para handlers

#### 5.2 Testes de Integração
- [ ] Testes de webhook com mocks
- [ ] Testes de integração com tools
- [ ] Testes end-to-end com plataformas reais (sandbox)

#### 5.3 Testes de Carga
- [ ] Testes de rate limiting
- [ ] Testes de múltiplas conversas simultâneas
- [ ] Testes de recuperação de falhas

### Fase 6: Documentação (Prioridade BAIXA)

#### 6.1 Documentação Técnica
- [ ] Atualizar `docs/messaging-integrations.md`
- [ ] Documentar APIs internas
- [ ] Criar guia de desenvolvimento

#### 6.2 Documentação de Usuário
- [ ] Guia de configuração passo a passo
- [ ] Exemplos de uso
- [ ] Troubleshooting comum

## 🔧 Estrutura de Arquivos Proposta

```
jarvis-rs/
├── messaging/                    # Crate comum
│   ├── src/
│   │   ├── lib.rs
│   │   ├── message.rs           # Tipos de mensagem
│   │   ├── conversation.rs      # Gerenciamento de conversas
│   │   ├── handler.rs           # Trait MessageHandler
│   │   └── platform.rs          # Trait MessagingPlatform
│   └── Cargo.toml
├── whatsapp/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── platform.rs          # Implementação WhatsAppPlatform
│   │   ├── client.rs            # Cliente WhatsApp Business API
│   │   ├── webhook.rs           # Servidor webhook
│   │   ├── message.rs           # Tipos específicos WhatsApp
│   │   └── config.rs            # Configuração
│   └── Cargo.toml
├── telegram/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── platform.rs          # Implementação TelegramPlatform
│   │   ├── client.rs            # Cliente Telegram Bot API
│   │   ├── webhook.rs           # Servidor webhook
│   │   ├── message.rs           # Tipos específicos Telegram
│   │   └── config.rs            # Configuração
│   └── Cargo.toml
└── core/
    └── src/
        └── messaging/            # Integração com core
            ├── mod.rs
            ├── handler.rs       # MessageToCodexHandler
            ├── router.rs        # Roteamento de comandos
            └── tools.rs         # Tools específicos de mensageria
```

## 🔄 Fluxo de Funcionamento

### Recebimento de Mensagem

```
1. Plataforma (WhatsApp/Telegram) → Webhook
2. Webhook → Validação e Parsing
3. Parsing → Conversão para IncomingMessage
4. IncomingMessage → MessageToCodexHandler
5. Handler → Parser de Comandos
6. Comando → ToolRouter (sistema existente)
7. Tool → Execução
8. Resultado → Formatação
9. Resposta → OutgoingMessage
10. OutgoingMessage → Cliente API
11. Cliente → Plataforma
```

### Exemplo: Comando `/exec ls -la`

```
Usuário: "/exec ls -la"
    ↓
Webhook recebe mensagem
    ↓
MessageToCodexHandler.process()
    ↓
Parser identifica comando "/exec"
    ↓
Extrai argumentos: ["ls", "-la"]
    ↓
ToolRouter.route("shell", args)
    ↓
Executa comando shell
    ↓
Formata resultado
    ↓
Envia resposta: "total 24\ndrwxr-xr-x ..."
```

## 📊 Priorização

### Sprint 1 (Semana 1-2)
- ✅ Fase 1: Reestruturação completa
- ✅ Fase 2.1: Criar handler básico

### Sprint 2 (Semana 3-4)
- ✅ Fase 2.2-2.3: Integração completa com tools
- ✅ Fase 3.1: WhatsApp webhook funcional

### Sprint 3 (Semana 5-6)
- ✅ Fase 3.2: Telegram webhook funcional
- ✅ Fase 4: Configuração completa

### Sprint 4 (Semana 7-8)
- ✅ Fase 5: Testes completos
- ✅ Fase 6: Documentação

## 🚨 Riscos e Mitigações

### Riscos Identificados
1. **Complexidade de integração** com sistema de tools existente
   - Mitigação: Análise detalhada do código existente antes de implementar

2. **Segurança de webhooks**
   - Mitigação: Implementar validação robusta e testes de segurança

3. **Rate limiting das APIs**
   - Mitigação: Implementar retry com backoff exponencial

4. **Gerenciamento de estado de conversas**
   - Mitigação: Usar sistema de estado existente do Jarvis

## 📚 Referências

- [WhatsApp Business API](https://developers.facebook.com/docs/whatsapp)
- [Telegram Bot API](https://core.telegram.org/bots/api)
- Documentação interna do Jarvis Core
- Sistema de Tools existente

## ✅ Checklist de Validação

Antes de considerar completo, verificar:

- [ ] Todos os crates compilam sem erros
- [ ] Webhooks recebem e validam mensagens corretamente
- [ ] Comandos básicos funcionam (`/exec`, `/read`, `/list`)
- [ ] Respostas são enviadas corretamente
- [ ] Contexto de conversas é mantido
- [ ] Configuração é carregada corretamente
- [ ] Testes passam (unitários e integração)
- [ ] Documentação está completa
- [ ] Exemplos de uso funcionam
- [ ] Rate limiting está implementado
- [ ] Logs e monitoramento estão funcionando
