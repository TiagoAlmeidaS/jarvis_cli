# 🔧 Troubleshooting: Erro 404 no OpenRouter Free Models

## Erro

```
unexpected status 404 Not Found: No endpoints found for deepseek/deepseek-r1:free., 
url: https://openrouter.ai/api/v1/responses
```

## Causa

Modelos free do OpenRouter podem não suportar o endpoint `/responses` e precisam usar `/chat/completions`.

## ✅ Solução Automática (Implementada)

**O Jarvis agora detecta automaticamente erros 404 do OpenRouter e retenta com `/chat/completions`.**

Quando você recebe um erro 404 com a mensagem "No endpoints found", o sistema:

1. **Fluxo Principal (CLI/TUI)**: Automaticamente retenta a requisição usando `/chat/completions` em vez de `/responses`
2. **Daemon**: Classifica o erro como recuperável e aciona fallback para o próximo provider configurado

**Não é mais necessário configurar manualmente `uses_chat_completions_api = true`** - o sistema faz isso automaticamente quando detecta o erro.

## Solução Manual (Se o Fallback Automático Não Funcionar)

Se por algum motivo o fallback automático não funcionar, você pode configurar manualmente:

Edite `~/.jarvis/config.toml` e adicione:

```toml
[model_providers.openrouter]
name = "OpenRouter"
base_url = "https://openrouter.ai/api/v1"
env_key = "OPENROUTER_API_KEY"
wire_api = "responses"
uses_chat_completions_api = true  # ← Força uso de /chat/completions
```

## Solução Alternativa: Verificar Nome do Modelo

O modelo pode ter um nome diferente. Tente:

1. **Verificar no site do OpenRouter**: https://openrouter.ai/models
2. **Usar formato alternativo**: `deepseek/deepseek-r1-0528:free` (com `-0528`)
3. **Usar modelo diferente**: `google/gemini-2.0-flash:free`

## Solução Alternativa: Usar Google AI Studio

Google AI Studio funciona com modelos free:

```bash
# 1. Obter API key: https://ai.google.dev
# 2. Configurar
$env:GOOGLE_API_KEY = "sua-chave"

# 3. Usar
jarvis chat -c model_provider=google -m "gemini-2.5-flash"
```

## Teste Rápido

```bash
# Testar - o fallback automático deve funcionar
jarvis chat -c model_provider=openrouter -m "deepseek/deepseek-r1:free"

# Ou testar Google AI Studio
jarvis chat -c model_provider=google -m "gemini-2.5-flash"
```

## Como Funciona o Fallback Automático

### Fluxo Principal (CLI/TUI)
1. Tenta usar `/responses` (padrão)
2. Se receber 404 com "No endpoints found":
   - Detecta automaticamente o erro
   - Retenta com `/chat/completions` (usa `uses_chat_completions_api = true`)
   - Se ainda falhar, retorna erro ao usuário

### Daemon
1. Tenta usar o provider/modelo configurado
2. Se receber 404 com "No endpoints found":
   - Classifica como `EndpointNotSupported` (erro recuperável)
   - Aciona fallback automático para o próximo provider na lista de fallbacks
   - Se todos os providers falharem, retorna erro final

## Próximos Passos

1. ✅ **Fallback automático implementado** - não é mais necessário configurar manualmente
2. ✅ Testar novamente - o sistema deve funcionar automaticamente
3. ✅ Se ainda falhar, verificar se o modelo existe no OpenRouter ou usar Google AI Studio como alternativa
