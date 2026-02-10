# ⚙️ README - Configuração do Jarvis CLI

Guia rápido para configurar o Jarvis CLI com suas credenciais.

---

## 🚀 Início Rápido

### 1. Copiar Templates

```bash
# Copiar template de configuração JSON
cp appsettings.Example.json appsettings.json

# Copiar template de variáveis de ambiente
cp .env.example .env
```

### 2. Preencher Credenciais

Edite os arquivos copiados e preencha com suas credenciais reais:
- ✏️ `appsettings.json` - Configuração principal
- ✏️ `.env` - Variáveis de ambiente

### 3. Configurar .gitignore

```bash
# Verificar se está no .gitignore
cat .gitignore | grep -E "appsettings|\.env"

# Se não estiver, adicionar:
echo "appsettings.json" >> .gitignore
echo "appsettings.*.json" >> .gitignore
echo ".env" >> .gitignore
echo "!appsettings.Example.json" >> .gitignore
```

---

## 📚 Documentação Completa

### Guia Principal

**[📖 GUIA_CONFIGURACAO.md](./GUIA_CONFIGURACAO.md)** - Guia completo e detalhado

Este guia contém:
- ✅ Explicação de cada seção de configuração
- ✅ Como obter credenciais (API keys, tokens)
- ✅ Configuração de LLM providers (OpenAI, Azure, Databricks, etc.)
- ✅ Configuração de embeddings
- ✅ Configuração de banco de dados (SQL Server)
- ✅ Configuração de RAG e vector stores
- ✅ Exemplos completos para dev, prod e testes
- ✅ Best practices de segurança

### Arquivos Template

**[📄 appsettings.Example.json](./appsettings.Example.json)** - Template de configuração JSON
- Copie para `appsettings.json`
- Preencha com suas credenciais
- Nunca commite o arquivo final!

**[📄 .env.example](./.env.example)** - Template de variáveis de ambiente
- Copie para `.env`
- Preencha com suas credenciais
- Nunca commite o arquivo final!

---

## 🔑 Credenciais Necessárias

### Obrigatórias (Mínimo)

```
✅ SQL Server
   - Host, porta, database, user, password

✅ LLM Provider (escolha um)
   - OpenAI: API Key
   - Azure OpenAI: Endpoint + API Key
   - Databricks: Base URL + Token
   - OpenRouter: API Key
   - Ollama: Host URL (local)

✅ Embeddings (escolha um)
   - Azure OpenAI: Endpoint + API Key
   - OpenAI: API Key
   - Databricks: Token
```

### Opcionais (Recomendadas)

```
⭐ GitHub
   - Token (para integração com repositórios)

⭐ Redis
   - Host + porta (para cache multi-level)

⭐ Qdrant
   - URL (para vector search RAG)

⭐ Observability
   - Loki URL (logs)
   - Push Gateway URL (métricas)
```

---

## 🎯 Configuração por Ambiente

### Desenvolvimento Local

```bash
# Usar configuração mínima
appsettings.Development.json

Necessário:
- SQL Server local (via Docker)
- Ollama local (opção gratuita)
- Redis local (opcional)
- Qdrant local (opcional)
```

**Docker Compose para Dev:**
```bash
# Usar nossa infraestrutura de testes
cd jarvis-rs
docker-compose -f docker-compose.test.yml up -d
```

### Testes

```bash
# Usar configuração de testes
appsettings.Test.json

Necessário:
- SQL Server: localhost:1433
- Redis: localhost:6379
- Qdrant: localhost:6333
- Mock providers para LLM/Embeddings
```

**Executar testes:**
```bash
# Ver guia completo
cat COMO_TESTAR_O_PROJETO.md

# Quick start
cd jarvis-rs
cargo test --package jarvis-core --lib
```

### Produção

```bash
# Usar configuração de produção
appsettings.Production.json

Necessário:
- SQL Server em servidor dedicado
- LLM provider com API key válida
- Embeddings configurado
- Observability habilitada
- Backups configurados
```

---

## 📋 Checklist de Configuração

### Passo 1: Arquivos Base

- [ ] Copiei `appsettings.Example.json` → `appsettings.json`
- [ ] Copiei `.env.example` → `.env`
- [ ] Verifiquei que está no `.gitignore`

### Passo 2: Banco de Dados

- [ ] SQL Server instalado/acessível
- [ ] Connection string preenchida
- [ ] Testei conexão: `sqlcmd -S host -U user -P pass`
- [ ] Database criado: `CREATE DATABASE JarvisDB`

### Passo 3: LLM Provider

- [ ] Escolhi provider (OpenAI/Azure/Databricks/etc.)
- [ ] Obtive API key/token
- [ ] Preenchi na configuração
- [ ] Testei chamada simples

### Passo 4: Embeddings

- [ ] Escolhi provider para embeddings
- [ ] Obtive credenciais
- [ ] Preenchi configuração
- [ ] Modelo compatível (text-embedding-3-small recomendado)

### Passo 5: Integrações Opcionais

- [ ] GitHub token (se usar integração Git)
- [ ] Redis (se usar cache multi-level)
- [ ] Qdrant (se usar RAG com vector search)
- [ ] Observability (Loki, Prometheus)

### Passo 6: Testes

- [ ] Executei testes unitários
- [ ] Configurei Docker para integration tests
- [ ] Validei todas as conexões

---

## 🔐 Segurança

### ✅ Fazer

1. ✅ Usar variáveis de ambiente para credenciais sensíveis
2. ✅ Adicionar `appsettings.json` e `.env` no `.gitignore`
3. ✅ Rotacionar tokens regularmente
4. ✅ Usar senhas fortes (mínimo 16 caracteres)
5. ✅ Manter backups das configurações (sem credenciais)
6. ✅ Usar TLS/SSL em produção (`Encrypt=true` no SQL Server)

### ❌ Não Fazer

1. ❌ Commitar credenciais no Git
2. ❌ Compartilhar keys em chat/email
3. ❌ Usar mesma senha em dev/prod
4. ❌ Logar credenciais em logs
5. ❌ Usar keys de teste em produção

---

## 🛠️ Ferramentas Úteis

### Testar Conexões

```bash
# SQL Server
sqlcmd -S localhost,1433 -U sa -P "senha" -Q "SELECT 1"

# Redis
redis-cli -h localhost -p 6379 ping

# Qdrant
curl http://localhost:6333/collections

# Ollama
curl http://localhost:11434/api/tags
```

### Validar Configuração

```bash
# Validar JSON
cat appsettings.json | jq .

# Verificar variáveis de ambiente
cat .env | grep -v '^#' | grep -v '^$'
```

### Gerar Senhas Seguras

```bash
# Linux/Mac
openssl rand -base64 32

# PowerShell
[System.Web.Security.Membership]::GeneratePassword(32,4)

# Python
python -c "import secrets; print(secrets.token_urlsafe(32))"
```

---

## 📊 Configuração Baseada nos Testes

### SQL Server (Conforme implementado)

```json
{
  "ConnectionStrings": {
    "DefaultConnection": "Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True;MultipleActiveResultSets=true"
  }
}
```

### Redis (Multi-level Cache - Conforme testado)

```json
{
  "Redis": {
    "ConnectionString": "localhost:6379",
    "DefaultDatabase": 0,
    "Enabled": true,
    "MultiLevel": {
      "L1CacheSizeMB": 100,
      "L1TTLSeconds": 300,
      "L2TTLSeconds": 3600,
      "EnablePromotion": true
    }
  }
}
```

### Qdrant (Vector Store - Conforme testado)

```json
{
  "Qdrant": {
    "Url": "http://localhost:6333",
    "Enabled": true,
    "Collections": {
      "Embeddings": "jarvis_embeddings",
      "Projects": "jarvis_projects"
    },
    "VectorSize": 1536
  }
}
```

---

## 🆘 Troubleshooting

### "Connection string invalid"

```bash
# Formato correto:
Server=HOST,PORT;Database=DB;User Id=USER;Password=PASS;TrustServerCertificate=True

# Testar:
sqlcmd -S HOST,PORT -U USER -P PASS -Q "SELECT 1"
```

### "API key invalid"

```bash
# Verificar:
- Key está correta (sem espaços)
- Não expirou
- Tem permissões corretas
- Provider correto (openai vs azure-openai)

# Regenerar se necessário
```

### "Cannot connect to Redis"

```bash
# Verificar se Redis está rodando:
redis-cli ping

# Se não, iniciar:
docker run -d -p 6379:6379 redis:7-alpine
```

### "Qdrant connection failed"

```bash
# Verificar se Qdrant está rodando:
curl http://localhost:6333/collections

# Se não, iniciar:
docker run -d -p 6333:6333 qdrant/qdrant
```

---

## 📚 Documentação Relacionada

### Configuração

- **[GUIA_CONFIGURACAO.md](./GUIA_CONFIGURACAO.md)** - Guia completo e detalhado
- **[appsettings.Example.json](./appsettings.Example.json)** - Template JSON
- **[.env.example](./.env.example)** - Template de variáveis

### Testes

- **[COMO_TESTAR_O_PROJETO.md](./COMO_TESTAR_O_PROJETO.md)** - Como executar testes
- **[QUICK_START_TESTES.md](./QUICK_START_TESTES.md)** - Quick start em 5 min
- **[INTEGRATION_TESTS.md](./jarvis-rs/INTEGRATION_TESTS.md)** - Integration tests

### Estrutura

- **[ESTRUTURA_DE_TESTES.md](./ESTRUTURA_DE_TESTES.md)** - Estrutura do projeto
- **[TESTING_STRATEGY.md](./TESTING_STRATEGY.md)** - Estratégia de testes

---

## 📞 Próximos Passos

1. **Configurar**
   ```bash
   cp appsettings.Example.json appsettings.json
   cp .env.example .env
   # Editar e preencher credenciais
   ```

2. **Testar Conexões**
   ```bash
   # SQL Server
   sqlcmd -S localhost,1433 -U sa -P "senha" -Q "SELECT 1"

   # Executar testes
   cd jarvis-rs
   cargo test --package jarvis-core --lib
   ```

3. **Executar Aplicação**
   ```bash
   # Desenvolvimento
   dotnet run --project JarvisCLI

   # Produção
   dotnet run --project JarvisCLI --configuration Release
   ```

4. **Verificar Logs**
   ```bash
   # Ver logs de execução
   tail -f logs/executions/*.log

   # Ver logs da aplicação
   tail -f logs/jarvis-*.log
   ```

---

## ✅ Resumo

**Arquivos Criados:**
- ✅ `GUIA_CONFIGURACAO.md` - Guia completo (~1,000 linhas)
- ✅ `appsettings.Example.json` - Template de configuração
- ✅ `.env.example` - Template de variáveis de ambiente
- ✅ `README_CONFIGURACAO.md` - Este arquivo (overview)

**Para Começar:**
1. Copie os templates
2. Preencha suas credenciais
3. Teste as conexões
4. Execute os testes
5. Inicie a aplicação

**Documentação Completa:**
- Leia `GUIA_CONFIGURACAO.md` para detalhes
- Use `COMO_TESTAR_O_PROJETO.md` para testes
- Consulte `INDICE_DOCUMENTACAO_TESTES.md` para navegação

---

**Última Atualização**: 2026-02-09
**Status**: ✅ **GUIAS DE CONFIGURAÇÃO COMPLETOS**

---

<p align="center">
  <strong>⚙️ Configure o Jarvis CLI com segurança e facilidade! ⚙️</strong>
</p>
