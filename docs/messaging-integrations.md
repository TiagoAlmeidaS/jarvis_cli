# Integrações de Mensageria: WhatsApp e Telegram

Este documento descreve a arquitetura e implementação das integrações do Codex com WhatsApp e Telegram, permitindo que o assistente execute funcionalidades e responda conversas através dessas plataformas.

## Visão Geral

As integrações de mensageria permitem que o Codex:
- **Receba mensagens** de WhatsApp e Telegram
- **Envie respostas** através dessas plataformas
- **Execute funcionalidades** do Codex via comandos de texto
- **Acesse histórico de conversas** para contexto
- **Gerencie múltiplas conversas** simultaneamente

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│                    Codex Core                               │
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
│ codex-whatsapp  │  │ codex-telegram  │  │ codex-messaging │
│                 │  │                 │  │   (common)      │
│ ┌────────────┐  │  │ ┌────────────┐  │  │                │
│ │  Webhook   │  │  │ │  Webhook   │  │  │ ┌────────────┐ │
│ │  Server    │  │  │ │  Server    │  │  │ │  Protocol   │ │
│ └────────────┘  │  │ └────────────┘  │  │ │  Handler    │ │
│ ┌────────────┐  │  │ ┌────────────┐  │  │ └────────────┘ │
│ │   Client   │  │  │ │   Client   │  │  │ ┌────────────┐ │
│ │   API      │  │  │ │   API      │  │  │ │  Message   │ │
│ └────────────┘  │  │ └────────────┘  │  │ │  Router    │ │
└─────────────────┘  └─────────────────┘  │ └────────────┘ │
                                           └────────────────┘
```

### Estrutura de Crates

1. **`codex-messaging`** (crate comum)
   - Protocolos e tipos compartilhados
   - Interface abstrata para plataformas de mensageria
   - Gerenciamento de conversas e contexto

2. **`codex-whatsapp`**
   - Integração com WhatsApp Business API
   - Webhook server para receber mensagens
   - Cliente para enviar mensagens

3. **`codex-telegram`**
   - Integração com Telegram Bot API
   - Webhook server para receber mensagens
   - Cliente para enviar mensagens

## Fluxo de Funcionamento

### Recebimento de Mensagens

1. **Webhook recebe mensagem** da plataforma (WhatsApp/Telegram)
2. **Validação e parsing** da mensagem
3. **Criação de contexto** de conversa (ou recuperação do existente)
4. **Roteamento para Codex Core** com contexto da conversa
5. **Processamento pelo Codex** (execução de tools, geração de resposta)
6. **Envio da resposta** de volta para a plataforma

### Execução de Funcionalidades

1. Usuário envia comando via mensagem (ex: `/exec ls -la`)
2. Codex identifica o comando e tool necessário
3. Executa a funcionalidade através do sistema de tools existente
4. Retorna resultado formatado como mensagem

## Implementação Técnica

### Estrutura de Diretórios

```
codex-rs/
├── messaging/              # Crate comum
│   ├── src/
│   │   ├── lib.rs
│   │   ├── protocol.rs      # Protocolos de mensageria
│   │   ├── message.rs       # Tipos de mensagem
│   │   ├── conversation.rs # Gerenciamento de conversas
│   │   └── handler.rs       # Handler base
│   └── Cargo.toml
├── whatsapp/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── client.rs        # Cliente WhatsApp Business API
│   │   ├── webhook.rs       # Servidor webhook
│   │   ├── message.rs       # Tipos específicos WhatsApp
│   │   └── config.rs        # Configuração
│   └── Cargo.toml
└── telegram/
    ├── src/
    │   ├── lib.rs
    │   ├── client.rs        # Cliente Telegram Bot API
    │   ├── webhook.rs       # Servidor webhook
    │   ├── message.rs       # Tipos específicos Telegram
    │   └── config.rs        # Configuração
    └── Cargo.toml
```

### Tipos de Mensagem

```rust
// codex-messaging/src/message.rs
pub enum MessageType {
    Text(String),
    Image { url: String, caption: Option<String> },
    Document { url: String, filename: String },
    Audio { url: String },
    Video { url: String, caption: Option<String> },
    Location { latitude: f64, longitude: f64 },
    Command { command: String, args: Vec<String> },
}

pub struct IncomingMessage {
    pub id: String,
    pub platform: Platform,
    pub from: Contact,
    pub chat_id: String,
    pub message_type: MessageType,
    pub timestamp: DateTime<Utc>,
    pub context: Option<ConversationContext>,
}

pub struct OutgoingMessage {
    pub chat_id: String,
    pub message_type: MessageType,
    pub reply_to: Option<String>,
}
```

### Interface de Plataforma

```rust
// codex-messaging/src/handler.rs
#[async_trait]
pub trait MessagingPlatform: Send + Sync {
    async fn send_message(&self, message: OutgoingMessage) -> Result<MessageId>;
    async fn get_conversation_history(&self, chat_id: &str) -> Result<Vec<IncomingMessage>>;
    async fn start_webhook_server(&self, handler: Box<dyn MessageHandler>) -> Result<()>;
    fn platform_name(&self) -> &str;
}
```

## Configuração

### WhatsApp

```toml
# config.toml
[messaging.whatsapp]
enabled = true
api_url = "https://graph.facebook.com/v18.0"
access_token = "${WHATSAPP_ACCESS_TOKEN}"
verify_token = "${WHATSAPP_VERIFY_TOKEN}"
webhook_port = 8080
phone_number_id = "${WHATSAPP_PHONE_NUMBER_ID}"
```

### Telegram

```toml
# config.toml
[messaging.telegram]
enabled = true
bot_token = "${TELEGRAM_BOT_TOKEN}"
webhook_url = "https://your-domain.com/telegram/webhook"
webhook_port = 8081
```

## Segurança

### Autenticação e Validação

- **WhatsApp**: Validação de `verify_token` no webhook
- **Telegram**: Validação de assinatura HMAC-SHA256
- **Rate limiting**: Limitação de requisições por usuário/chat
- **Sanitização**: Validação e sanitização de todas as mensagens recebidas

### Privacidade

- Armazenamento seguro de tokens e credenciais
- Criptografia de dados sensíveis em trânsito
- Logs não devem conter conteúdo de mensagens
- Conformidade com LGPD/GDPR

## Ferramentas (Tools) Disponíveis

As seguintes ferramentas do Codex estarão disponíveis via mensageria:

1. **`/exec <command>`** - Executa comando shell
2. **`/read <file>`** - Lê conteúdo de arquivo
3. **`/list <directory>`** - Lista diretório
4. **`/search <query>`** - Busca em arquivos
5. **`/help`** - Lista comandos disponíveis

## Exemplos de Uso

### Exemplo 1: Executar Comando

```
Usuário: /exec ls -la
Codex: Executando comando...
Codex: [resultado do comando]
```

### Exemplo 2: Conversa Natural

```
Usuário: Qual é o status do projeto?
Codex: Analisando repositório...
Codex: O projeto tem 15 arquivos modificados e 3 PRs abertos.
```

### Exemplo 3: Leitura de Arquivo

```
Usuário: /read README.md
Codex: [conteúdo do README.md]
```

## Roadmap

### Fase 1: MVP (Mínimo Viável)
- [x] Estrutura de crates
- [ ] Integração básica WhatsApp
- [ ] Integração básica Telegram
- [ ] Envio/recebimento de mensagens texto
- [ ] Execução de comandos básicos

### Fase 2: Funcionalidades Avançadas
- [ ] Suporte a mídia (imagens, documentos)
- [ ] Histórico de conversas
- [ ] Múltiplas conversas simultâneas
- [ ] Comandos personalizados

### Fase 3: Otimizações
- [ ] Cache de contexto
- [ ] Rate limiting inteligente
- [ ] Compressão de mensagens longas
- [ ] Suporte a grupos/canais

## Referências

- [WhatsApp Business API](https://developers.facebook.com/docs/whatsapp)
- [Telegram Bot API](https://core.telegram.org/bots/api)
- [Codex Tools Documentation](./tools.md)
