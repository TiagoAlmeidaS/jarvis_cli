# Guia de Configuração - Dashboard Web do Daemon

Este guia cobre todos os passos necessários para configurar e executar o dashboard web do Jarvis Daemon.

## Pré-requisitos

- Rust instalado (para compilar)
- Banco SQLite do daemon (`~/.jarvis/daemon.db`) - será criado automaticamente se não existir
- API key configurada no `config.toml`

## Passo 1: Configurar API Key

### Opção A: Via config.toml (Recomendado)

Edite ou crie `~/.jarvis/config.toml`:

```toml
[api]
api_key = "sua-chave-secreta-aqui"  # Obrigatório
port = 3000                          # Opcional, padrão: 3000
bind_address = "0.0.0.0"            # Opcional, padrão: 0.0.0.0
enable_cors = false                 # Opcional, padrão: false
```

### Gerar API Key

**Linux/Mac:**
```bash
openssl rand -hex 32
```

**Windows (PowerShell):**
```powershell
-join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
```

**Ou use um gerador online:**
- https://www.uuidgenerator.net/
- Gere um UUID v4 e use como API key

## Passo 2: Compilar o Web API

```bash
cd jarvis-rs
cargo build --package jarvis-web-api --release
```

O binário estará em `target/release/jarvis-web-api` (ou `jarvis-web-api.exe` no Windows).

## Passo 3: Verificar/Criar Banco do Daemon

O dashboard precisa acessar o banco SQLite do daemon. Por padrão, ele está em:

- **Linux/Mac**: `~/.jarvis/daemon.db`
- **Windows**: `C:\Users\<usuario>\.jarvis\daemon.db`

### Opção A: Banco já existe (daemon já rodou antes)

Se o daemon já foi executado anteriormente, o banco já existe. Nenhuma ação necessária.

### Opção B: Criar banco vazio (primeira vez)

O banco será criado automaticamente na primeira execução do `jarvis-web-api`. Você pode também criar manualmente executando:

```bash
# Criar diretório se não existir
mkdir -p ~/.jarvis  # Linux/Mac
# ou
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.jarvis"  # Windows

# O banco será criado automaticamente quando o web-api iniciar
```

### Opção C: Usar banco em localização customizada

Defina a variável de ambiente:

```bash
export JARVIS_DAEMON_DB=/caminho/para/daemon.db  # Linux/Mac
# ou
$env:JARVIS_DAEMON_DB = "C:\caminho\para\daemon.db"  # Windows PowerShell
```

## Passo 4: Executar o Web API

### Execução Direta

```bash
# Linux/Mac
./target/release/jarvis-web-api

# Windows
.\target\release\jarvis-web-api.exe
```

O servidor estará disponível em `http://localhost:3000`.

### Verificar se está funcionando

```bash
# Health check
curl http://localhost:3000/api/health

# Deve retornar:
# {"status":"ok","version":"..."}
```

## Passo 5: Acessar o Dashboard

1. Abra o navegador em: `http://localhost:3000/daemon`
2. Na primeira vez, será solicitada a API key
3. Digite a API key configurada no `config.toml`
4. A API key será salva no `localStorage` do navegador

## Passo 6: Configurar Primeiro Pipeline (Opcional)

Se você ainda não tem pipelines configurados, pode criar um via dashboard ou CLI:

### Via Dashboard Web

1. Acesse a aba "Pipelines"
2. Use a interface para criar um novo pipeline (funcionalidade futura)

### Via CLI

```bash
# Criar pipeline de exemplo
jarvis daemon pipeline add pipeline-exemplo.json
```

Exemplo de `pipeline-exemplo.json`:

```json
{
  "id": "seo-blog-exemplo",
  "name": "SEO Blog - Exemplo",
  "strategy": "seo_blog",
  "schedule_cron": "0 3 * * *",
  "config": {
    "llm": {
      "provider": "openrouter",
      "model": "mistralai/mistral-nemo"
    },
    "seo": {
      "niche": "Tecnologia",
      "language": "pt-BR"
    }
  }
}
```

## Troubleshooting

### Erro: "Daemon endpoints will be unavailable"

**Causa**: Banco SQLite do daemon não foi encontrado ou não pôde ser criado.

**Solução**:
1. Verifique se o diretório `~/.jarvis` existe e tem permissões de escrita
2. Verifique se a variável `JARVIS_DAEMON_DB` está correta (se usada)
3. Verifique os logs do servidor para mais detalhes

### Erro: "API not configured" ou "Missing Authorization header"

**Causa**: API key não está configurada no `config.toml`.

**Solução**:
1. Verifique se `~/.jarvis/config.toml` existe
2. Verifique se a seção `[api]` existe
3. Verifique se `api_key` está preenchida

### Dashboard não carrega dados

**Causa**: Banco do daemon está vazio ou não tem dados.

**Solução**:
1. Execute o daemon pelo menos uma vez para criar goals padrão
2. Ou crie pipelines e execute jobs manualmente
3. O dashboard funcionará mesmo sem dados (mostrará zeros/vazios)

### WebSocket não conecta

**Causa**: Token não fornecido ou inválido.

**Solução**:
1. Verifique se a API key está correta no `config.toml`
2. O WebSocket valida o token via query parameter: `ws://localhost:3000/ws/daemon?token=SUA_API_KEY`
3. O dashboard JavaScript gerencia isso automaticamente

## Variáveis de Ambiente

| Variável | Descrição | Padrão |
|----------|-----------|--------|
| `JARVIS_HOME` | Diretório base do Jarvis | `~/.jarvis` |
| `JARVIS_DAEMON_DB` | Caminho do banco do daemon | `~/.jarvis/daemon.db` |
| `PORT` | Porta do servidor web | `3000` (ou do config.toml) |
| `RUST_LOG` | Nível de log | `info` |

## Próximos Passos

Após configurar o dashboard:

1. **Subir o daemon**: Execute `jarvis-daemon run` para começar a processar pipelines
2. **Monitorar**: Use o dashboard para acompanhar jobs, métricas e proposals
3. **Configurar pipelines**: Adicione pipelines via CLI ou interface web
4. **Revisar proposals**: Aprove/rejeite propostas do strategy analyzer via dashboard

## Referências

- [Documentação da API](./DAEMON_API.md) - Endpoints REST completos
- [Guia de Setup do Daemon](./DAEMON_SETUP.md) - Como configurar o daemon
- [Guia de Monitoramento](./DAEMON_MONITORING.md) - Formas de monitorar o daemon
