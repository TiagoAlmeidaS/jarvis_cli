# 🚀 Setup: Serviços Jarvis na VPS via Tailscale

## 📋 Visão Geral

Este guia configura **todos os serviços backend do Jarvis na VPS** acessíveis via Tailscale:

- **Qdrant** (Vector Database para RAG) - Porta 6333
- **PostgreSQL** (Dados persistentes) - Porta 5432
- **Redis** (Cache distribuído) - Porta 6379
- **Ollama** (LLM local) - Porta 11434 ✅ (já rodando)

**Vantagens:**
- ✅ Todos os dados centralizados na VPS
- ✅ Acesso via Tailscale (seguro, sem expor à internet)
- ✅ Fácil manutenção e backup
- ✅ Compartilhamento de contexto entre máquinas
- ✅ Performance consistente

---

## 🛠️ Instalação

### Passo 1: Deploy na VPS

```bash
# Tornar script executável
chmod +x deploy-jarvis-services-vps.sh

# Fazer deploy
./deploy-jarvis-services-vps.sh
```

O script vai:
1. Conectar na VPS via Tailscale
2. Copiar o docker-compose.yml
3. Criar arquivo .env com credenciais
4. Iniciar todos os containers

### Passo 2: Verificar Containers

```bash
# Conectar na VPS
ssh root@100.98.213.86

# Verificar containers
cd /opt/jarvis
docker-compose ps

# Deve mostrar:
# jarvis-qdrant     - UP
# jarvis-postgres   - UP
# jarvis-redis      - UP
# jarvis-ollama     - UP
# jarvis-adminer    - UP
```

### Passo 3: Testar Conexões

```bash
# Qdrant
curl http://100.98.213.86:6333/collections
# Esperado: {"result":{"collections":[]},"status":"ok","time":0.000...}

# PostgreSQL
psql -h 100.98.213.86 -U jarvis -d jarvis
# Senha: jarvis_secure_password_2026

# Redis
redis-cli -h 100.98.213.86 ping
# Esperado: PONG

# Ollama (já está rodando)
curl http://100.98.213.86:11434/api/tags
# Esperado: lista de modelos
```

### Passo 4: Atualizar Configuração Local

```bash
# Fazer backup do config atual
cp ~/.jarvis/config.toml ~/.jarvis/config.toml.backup

# Adicionar configurações da VPS
cat config.toml.vps >> ~/.jarvis/config.toml
```

**Ou editar manualmente `~/.jarvis/config.toml`:**

```toml
# Atualizar seção [rag.qdrant]
[rag.qdrant]
url = "http://100.98.213.86:6333"  # ← Mudar de localhost para VPS

# Adicionar [database]
[database]
provider = "postgres"
host = "100.98.213.86"
port = 5432
database = "jarvis"
username = "jarvis"
password = "jarvis_secure_password_2026"

# Adicionar [cache]
[cache]
enabled = true
provider = "redis"
host = "100.98.213.86"
port = 6379
```

---

## 🧪 Teste: Indexar Projeto

Agora que o Qdrant está rodando na VPS, vamos indexar o projeto:

```bash
cd jarvis-rs

# Verificar conexão
curl http://100.98.213.86:6333/collections

# Indexar documentação
./target/debug/jarvis.exe context add ../README.md --doc-type markdown
./target/debug/jarvis.exe context add ./README.md --doc-type markdown

# Indexar config
./target/debug/jarvis.exe context add ./Cargo.toml --doc-type project
./target/debug/jarvis.exe context add ./cli/Cargo.toml --doc-type project

# Verificar indexação
./target/debug/jarvis.exe context stats
./target/debug/jarvis.exe context list

# Testar busca
./target/debug/jarvis.exe context search "jarvis cli" --limit 5
```

---

## 📊 Dashboards e UIs

### Qdrant Dashboard
```
http://100.98.213.86:6333/dashboard
```
- Ver coleções
- Explorar vetores
- Testar buscas

### Adminer (PostgreSQL UI)
```
http://100.98.213.86:8080
```
**Login:**
- System: PostgreSQL
- Server: postgres
- Username: jarvis
- Password: jarvis_secure_password_2026
- Database: jarvis

### Redis (via redis-cli)
```bash
redis-cli -h 100.98.213.86

# Comandos úteis:
> PING
> INFO
> KEYS *
> GET <key>
```

---

## 🔧 Manutenção

### Ver logs dos containers
```bash
ssh root@100.98.213.86

cd /opt/jarvis

# Todos os logs
docker-compose logs -f

# Log específico
docker-compose logs -f qdrant
docker-compose logs -f postgres
docker-compose logs -f redis
```

### Reiniciar serviços
```bash
# Reiniciar um serviço
docker-compose restart qdrant

# Reiniciar todos
docker-compose restart

# Parar todos
docker-compose stop

# Iniciar todos
docker-compose start
```

### Backup de dados
```bash
# Backup Qdrant
docker exec jarvis-qdrant tar -czf - /qdrant/storage > qdrant-backup-$(date +%Y%m%d).tar.gz

# Backup PostgreSQL
docker exec jarvis-postgres pg_dump -U jarvis jarvis > jarvis-db-backup-$(date +%Y%m%d).sql

# Backup Redis
docker exec jarvis-redis redis-cli SAVE
docker cp jarvis-redis:/data/dump.rdb redis-backup-$(date +%Y%m%d).rdb
```

---

## 🐛 Troubleshooting

### Qdrant não aceita conexões
```bash
# Verificar se está rodando
docker ps | grep qdrant

# Ver logs
docker logs jarvis-qdrant

# Reiniciar
docker restart jarvis-qdrant

# Verificar porta
curl http://100.98.213.86:6333/healthz
```

### PostgreSQL não aceita conexões
```bash
# Verificar container
docker ps | grep postgres

# Testar conexão
psql -h 100.98.213.86 -U jarvis -d jarvis

# Ver logs
docker logs jarvis-postgres
```

### Redis não responde
```bash
# Testar
redis-cli -h 100.98.213.86 ping

# Ver logs
docker logs jarvis-redis
```

### Contexto não persiste
```bash
# Verificar se Qdrant está acessível
curl http://100.98.213.86:6333/collections

# Verificar config
cat ~/.jarvis/config.toml | grep -A 5 "\[rag.qdrant\]"

# Ver logs do Jarvis
tail -f ~/.jarvis/log/jarvis.log
```

---

## 🎯 Resultado Final

Após completar este setup, você terá:

✅ **RAG funcional** com contexto persistente
✅ **Banco de dados** para sessões e conhecimento
✅ **Cache distribuído** para performance
✅ **LLM local** para embeddings e inferência
✅ **Acesso seguro** via Tailscale (sem expor à internet)
✅ **Backups centralizados** na VPS

E o Jarvis finalmente vai **entender o projeto local** ao invés de buscar na web! 🎉

---

## 📝 Notas Importantes

1. **Segurança**: Os serviços estão expostos via Tailscale, que já é uma VPN. Não configure senhas complexas se for apenas você usando.

2. **Performance**: Acessar via Tailscale adiciona latência (~10-50ms), mas é aceitável para RAG.

3. **Custos**: Tudo roda na sua VPS, sem custos adicionais de APIs.

4. **Backup**: Configure backups automáticos dos volumes Docker.

5. **Monitoramento**: Considere adicionar Prometheus + Grafana no futuro para monitorar os serviços.

---

## 💡 Próximos Passos

1. ✅ Deploy dos serviços (este guia)
2. 📚 Indexar todo o projeto (ver `index-project.sh`)
3. 🤖 Testar chat com contexto local
4. 🔄 Configurar sync automático de contexto
5. 📊 Implementar analytics na VPS
