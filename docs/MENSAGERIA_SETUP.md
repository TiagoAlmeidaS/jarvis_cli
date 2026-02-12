# Guia de Configuração - Integrações de Mensageria

Este guia explica como configurar e usar as integrações de mensageria do Jarvis com WhatsApp e Telegram.

## Visão Geral

O Jarvis suporta integrações com:
- **WhatsApp Business API**: Para receber e enviar mensagens via WhatsApp
- **Telegram Bot API**: Para receber e enviar mensagens via Telegram

Ambas as integrações permitem que você interaja com o Jarvis através de comandos de texto, executando ferramentas e acessando funcionalidades do sistema.

## Pré-requisitos

### WhatsApp Business API
1. Conta no Facebook Business
2. Aplicativo criado no Facebook Developers
3. WhatsApp Business API configurado
4. Token de acesso (Access Token)
5. ID do número de telefone (Phone Number ID)
6. Token de verificação para webhook

### Telegram Bot API
1. Conta no Telegram
2. Bot criado através do [@BotFather](https://t.me/botfather)
3. Token do bot fornecido pelo BotFather

## Configuração

### 1. Configuração no config.toml

Adicione a seção `[messaging]` ao seu arquivo `config.toml`:

```toml
[messaging]
enabled = true

[messaging.whatsapp]
enabled = true
api_url = "https://graph.facebook.com/v18.0"
access_token = "your_access_token"
verify_token = "your_verify_token"
phone_number_id = "your_phone_number_id"
webhook_port = 8080

[messaging.telegram]
enabled = true
bot_token = "your_bot_token"
webhook_url = "https://your-domain.com/telegram/webhook"
webhook_port = 8081
webhook_secret = "your_webhook_secret"  # Opcional
```

### 2. Variáveis de Ambiente (Alternativa)

Você pode usar variáveis de ambiente em vez de valores hardcoded no `config.toml`:

**WhatsApp:**
- `WHATSAPP_ACCESS_TOKEN`: Token de acesso da API
- `WHATSAPP_VERIFY_TOKEN`: Token de verificação do webhook
- `WHATSAPP_PHONE_NUMBER_ID`: ID do número de telefone

**Telegram:**
- `TELEGRAM_BOT_TOKEN`: Token do bot
- `TELEGRAM_WEBHOOK_SECRET`: Token secreto do webhook (opcional)

### 3. Configuração de Webhooks

#### WhatsApp

1. Configure o webhook no Facebook Developers Console:
   - URL: `https://your-domain.com/whatsapp/webhook`
   - Token de verificação: O mesmo valor configurado em `verify_token`

2. O servidor webhook do Jarvis escuta na porta configurada (padrão: 8080)

#### Telegram

1. Configure o webhook via API do Telegram:
   ```bash
   curl -X POST "https://api.telegram.org/bot<BOT_TOKEN>/setWebhook" \
     -H "Content-Type: application/json" \
     -d '{"url": "https://your-domain.com/telegram/webhook"}'
   ```

2. O servidor webhook do Jarvis escuta na porta configurada (padrão: 8081)

## Comandos Suportados

O Jarvis suporta os seguintes comandos via mensageria:

### `/exec <comando> [args...]`
Executa um comando do sistema.

**Exemplo:**
```
/exec ls -la
/exec git status
```

### `/read <caminho>`
Lê o conteúdo de um arquivo.

**Exemplo:**
```
/read /path/to/file.txt
/read ./src/main.rs
```

### `/list <caminho>`
Lista o conteúdo de um diretório.

**Exemplo:**
```
/list /path/to/directory
/list ./src
```

### `/search <query>`
Busca por arquivos ou conteúdo.

**Exemplo:**
```
/search function_name
/search TODO
```

### `/help`
Exibe a lista de comandos disponíveis.

## Segurança

### Rate Limiting

O Jarvis implementa rate limiting para proteger contra abuso:
- Limite por IP: 100 requisições por minuto
- Limite por chat: 50 requisições por minuto

### Validação de Webhooks

- **WhatsApp**: Valida o token de verificação (`verify_token`) durante a configuração do webhook
- **Telegram**: Valida o token secreto (`webhook_secret`) se configurado, ou o header `x-telegram-bot-api-secret-token`

## Exemplos de Uso

### Exemplo 1: Executar Comando

**WhatsApp/Telegram:**
```
/exec echo "Hello, Jarvis!"
```

**Resposta:**
```
Hello, Jarvis!
```

### Exemplo 2: Ler Arquivo

**WhatsApp/Telegram:**
```
/read README.md
```

**Resposta:**
```
[Conteúdo do arquivo README.md]
```

### Exemplo 3: Listar Diretório

**WhatsApp/Telegram:**
```
/list ./src
```

**Resposta:**
```
src/
├── main.rs
├── lib.rs
└── config.rs
```

## Troubleshooting

### Webhook não recebe mensagens

1. Verifique se o servidor está rodando e acessível
2. Confirme que a porta está corretamente configurada
3. Verifique os logs do Jarvis para erros
4. Para WhatsApp, confirme que o webhook está verificado no Facebook Developers Console
5. Para Telegram, confirme que o webhook foi configurado corretamente via API

### Erro de autenticação

1. Verifique se os tokens estão corretos
2. Confirme que as variáveis de ambiente estão definidas (se usadas)
3. Para WhatsApp, verifique se o token de acesso não expirou
4. Para Telegram, confirme que o token do bot está correto

### Rate limiting muito restritivo

Os limites padrão podem ser ajustados no código se necessário. Consulte a documentação técnica para mais detalhes.

## Arquitetura Técnica

Para mais detalhes sobre a arquitetura técnica, consulte:
- [Planejamento Técnico](./PLANEJAMENTO_INTEGRACOES_MENSAGERIA.md)
- [Status das Integrações](./INTEGRACOES_MENSAGERIA_STATUS.md)

## Suporte

Para problemas ou dúvidas:
1. Consulte os logs do Jarvis
2. Verifique a documentação técnica
3. Abra uma issue no repositório do projeto
