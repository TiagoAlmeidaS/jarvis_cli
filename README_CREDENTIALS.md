# 🔐 Configuração de Credenciais - Jarvis CLI

## ✅ Status: Credenciais Configuradas

Suas credenciais foram configuradas com sucesso nos seguintes arquivos:

### 📁 Arquivos Criados

1. **`.env`** (raiz do projeto)
   - Localização: `E:\projects\ia\jarvis_cli\.env`
   - Contém: Google Client ID, Databricks, OpenRouter

2. **`jarvis-rs/.env`** (diretório Rust)
   - Localização: `E:\projects\ia\jarvis_cli\jarvis-rs\.env`
   - Usado durante desenvolvimento e build

3. **`~/.jarvis/config.toml`** (configuração global)
   - Localização: `C:\Users\tiago\.jarvis\config.toml`
   - Configuração principal do Jarvis CLI

---

## 🔑 Credenciais Configuradas

### 1. Google OAuth
```
Client ID: 765554684645-geg8l26m1vkukn792bfdgtm0urhe905v.apps.googleusercontent.com
```
**Uso:** Autenticação do usuário via Google OAuth 2.0

### 2. Databricks (Provedor LLM Principal)
```
API Key:  your_databricks_api_key_here
Base URL: https://adb-926216925051160.0.azuredatabricks.net
```

**Modelos Configurados:**
- **Planner** (Planejamento): `databricks-claude-opus-4-5`
- **Developer** (Desenvolvimento): `databricks-meta-llama-3-1-405b`
- **Reviewer** (Revisão): `databricks-meta-llama-3-1-405b`
- **FastChat** (Chat Rápido): `databricks-claude-haiku-4-5`

### 3. OpenRouter (Provedor Alternativo)
```
API Key:  your_openrouter_api_key_here
Base URL: https://openrouter.ai/api/v1/
```

**Modelos Configurados:**
- **Planner**: `anthropic/claude-3.5-sonnet`
- **Developer**: `anthropic/claude-3.5-sonnet`
- **Reviewer**: `openai/gpt-4o`
- **FastChat**: `google/gemini-2.0-flash-exp`

---

## 🚀 Como Usar

### Opção 1: Configurar Variáveis de Ambiente (Recomendado)

Execute o script PowerShell:
```powershell
cd E:\projects\ia\jarvis_cli
.\configure-credentials.ps1
```

Ou configure manualmente:
```powershell
$env:GOOGLE_CLIENT_ID="765554684645-geg8l26m1vkukn792bfdgtm0urhe905v.apps.googleusercontent.com"
$env:DATABRICKS_API_KEY="your_databricks_api_key_here"
$env:DATABRICKS_BASE_URL="https://adb-926216925051160.0.azuredatabricks.net"
$env:OPENROUTER_API_KEY="your_openrouter_api_key_here"
```

### Opção 2: Usar arquivos .env

Os arquivos `.env` já foram criados e serão carregados automaticamente.

---

## 🧪 Testar a Configuração

### 1. Build do projeto
```bash
cd E:\projects\ia\jarvis_cli\jarvis-rs
cargo build --package jarvis-cli
```

### 2. Executar o Jarvis CLI
```bash
.\target\debug\jarvis.exe chat
```

### 3. Verificar autenticação
Ao executar, você deverá ver:
1. Opção "Sign in with Google"
2. Browser abre para login Google OAuth
3. Após login, o CLI usa Databricks como provider padrão

---

## 🔄 Trocar de Provider

### Usar Databricks (padrão)
```bash
jarvis --profile default chat
# ou simplesmente
jarvis chat
```

### Usar OpenRouter
```bash
jarvis --profile openrouter chat
```

### Usar modelo específico
```bash
jarvis --model databricks-claude-haiku-4-5 chat
```

---

## 📊 Parâmetros dos Modelos

Configurados em `config.toml`:

| Perfil     | Temperature | Timeout | Max Tokens |
|------------|-------------|---------|------------|
| Planner    | 0.0         | 300s    | 4096       |
| Developer  | 0.2         | 30s     | 4096       |
| Reviewer   | 0.5         | 30s     | 4096       |
| FastChat   | 0.7         | 30s     | 4096       |

---

## 🔐 Segurança

⚠️ **IMPORTANTE:**
- Os arquivos `.env` contêm credenciais sensíveis
- Não commite esses arquivos no Git
- Os arquivos já estão no `.gitignore`

### Verificar se .env está ignorado:
```bash
git status
# .env NÃO deve aparecer na lista
```

---

## 🛠️ Troubleshooting

### Erro: "Google Client ID not found"
```powershell
# Re-executar script de configuração
.\configure-credentials.ps1
```

### Erro: "Databricks API key invalid"
Verificar se a chave está correta:
```bash
echo $env:DATABRICKS_API_KEY
```

### Erro: "Failed to connect to Databricks"
Verificar URL base:
```bash
echo $env:DATABRICKS_BASE_URL
```

---

## 📞 Suporte

Se encontrar problemas:
1. Verifique se as variáveis de ambiente estão configuradas
2. Confirme se os arquivos `.env` existem
3. Tente recompilar: `cargo clean && cargo build`
4. Verifique logs em: `C:\Users\tiago\.jarvis\logs\`

---

## 🎯 Próximos Passos

1. ✅ Credenciais configuradas
2. ✅ Arquivos criados
3. ⏳ Testar autenticação Google OAuth
4. ⏳ Testar integração com Databricks
5. ⏳ Verificar funcionamento dos modelos

**Status:** Pronto para uso! 🚀
