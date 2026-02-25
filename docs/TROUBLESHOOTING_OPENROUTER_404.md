# 🔧 Troubleshooting: Erro 404 no OpenRouter Free Models

## Erro

```
unexpected status 404 Not Found: No endpoints found for deepseek/deepseek-r1:free., 
url: https://openrouter.ai/api/v1/responses
```

## Causa

Modelos free do OpenRouter podem não suportar o endpoint `/responses` e precisam usar `/chat/completions`.

## Solução 1: Configurar Chat Completions no OpenRouter

Edite `~/.jarvis/config.toml` e adicione:

```toml
[model_providers.openrouter]
name = "OpenRouter"
base_url = "https://openrouter.ai/api/v1"
env_key = "OPENROUTER_API_KEY"
wire_api = "responses"
uses_chat_completions_api = true  # ← Adicione esta linha
```

## Solução 2: Verificar Nome do Modelo

O modelo pode ter um nome diferente. Tente:

1. **Verificar no site do OpenRouter**: https://openrouter.ai/models
2. **Usar formato alternativo**: `deepseek/deepseek-r1-0528:free` (com `-0528`)
3. **Usar modelo diferente**: `google/gemini-2.0-flash:free`

## Solução 3: Usar Google AI Studio (Alternativa)

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
# Testar com Chat Completions habilitado
jarvis chat -c model_provider=openrouter -m "deepseek/deepseek-r1:free"

# Ou testar Google AI Studio
jarvis chat -c model_provider=google -m "gemini-2.5-flash"
```

## Próximos Passos

1. ✅ Adicionar `uses_chat_completions_api = true` no config.toml
2. ✅ Testar novamente
3. ✅ Se ainda falhar, usar Google AI Studio como alternativa
