# Integration Tests - Jarvis CLI

Este documento descreve como executar e manter os testes de integração do Jarvis CLI.

## 📋 Pré-requisitos

### Obrigatório
- **Docker Desktop** instalado e rodando
- **Docker Compose** (incluído no Docker Desktop)
- **Rust** e **Cargo** instalados

### Recursos Mínimos
- RAM: 4GB disponível
- Disk: 5GB disponível para imagens Docker
- Portas livres: 1433 (SQL Server), 6379 (Redis), 6333-6334 (Qdrant)

## 🚀 Execução Rápida

### Windows (PowerShell)
```powershell
cd jarvis-rs
.\run-integration-tests.ps1
```

### Linux/Mac (Bash)
```bash
cd jarvis-rs
chmod +x run-integration-tests.sh
./run-integration-tests.sh
```

## 🎯 Opções de Execução

### Manter Containers Rodando
Útil para debug ou execuções repetidas:

```powershell
# Windows
.\run-integration-tests.ps1 -KeepContainers

# Linux/Mac
./run-integration-tests.sh --keep-containers
```

Para parar manualmente:
```bash
docker-compose -f docker-compose.test.yml down
```

### Filtrar Testes Específicos
Execute apenas testes de um módulo específico:

```powershell
# Windows - apenas SQL Server
.\run-integration-tests.ps1 -Filter "sqlserver"

# Windows - apenas Redis
.\run-integration-tests.ps1 -Filter "redis"

# Linux/Mac - apenas Qdrant
./run-integration-tests.sh --filter "qdrant"
```

## 🐳 Serviços Docker

### SQL Server
- **Imagem**: `mcr.microsoft.com/mssql/server:2022-latest`
- **Porta**: 1433
- **Credenciais**:
  - User: `sa`
  - Password: `YourPassword123!`
- **Connection String**: `Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True`

### Redis
- **Imagem**: `redis:7-alpine`
- **Porta**: 6379
- **URL**: `redis://localhost:6379`

### Qdrant
- **Imagem**: `qdrant/qdrant:latest`
- **Portas**:
  - HTTP API: 6333
  - gRPC: 6334
- **URL**: `http://localhost:6333`

## 🧪 Testes Cobertos

### SQL Server Integration Tests (9 testes)

**Módulo**: `integrations::sqlserver::database`
- `test_database_connection_real` - Conexão real com SQL Server
- `test_database_get_client` - Obter client de conexão
- `test_database_health_check` - Verificar saúde da conexão
- `test_database_is_available` - Verificar disponibilidade
- `test_database_connection_failure` - Falha de conexão esperada
- `test_database_invalid_credentials` - Credenciais inválidas
- `test_database_multiple_clients` - Múltiplas conexões simultâneas

**Módulo**: `integrations::sqlserver::migrations`
- `test_migrations_full_run` - Executar todas as migrations
- `test_migrations_idempotent` - Migrations são idempotentes

**Cobertura Esperada**: ~80% dos métodos async

### Redis Integration Tests (10 testes)

**Módulo**: `integrations::redis::cache`
- `test_redis_connection` - Conexão com Redis
- `test_redis_set_get` - Set e Get básico
- `test_redis_set_with_ttl` - TTL expiration
- `test_redis_delete` - Deletar chaves
- `test_redis_exists` - Verificar existência
- `test_redis_json_serialization` - Serialização JSON
- `test_redis_concurrent_access` - Acesso concorrente
- `test_redis_cache_miss` - Cache miss scenario
- `test_redis_connection_failure` - Falha de conexão
- `test_redis_reconnection` - Reconexão automática

**Cobertura Esperada**: ~80% do RedisCache

### Analytics Integration Tests (13 testes)

**Módulo**: `analytics::queries`
- `test_create_command_execution` - Criar registro de execução
- `test_get_command_statistics` - Obter estatísticas
- `test_record_response_quality` - Registrar qualidade
- `test_get_average_rating` - Média de avaliações
- `test_update_skill_usage` - Atualizar uso de skill
- `test_get_popular_skills` - Skills populares
- `test_complex_analytics_query` - Query complexa

**Módulo**: `analytics::self_improvement`
- `test_analyze_command_patterns` - Analisar padrões
- `test_generate_suggestions` - Gerar sugestões
- `test_learn_from_error` - Aprender com erros
- `test_detect_skill_issues` - Detectar problemas
- `test_generate_improvement_plan` - Plano de melhoria
- `test_full_analytics_pipeline` - Pipeline completo

**Cobertura Esperada**: ~85% dos métodos async

## 🔧 Troubleshooting

### Erro: "Porta já em uso"
```bash
# Verificar se há containers antigos rodando
docker ps -a | grep jarvis-test

# Parar containers antigos
docker-compose -f docker-compose.test.yml down

# Se necessário, matar processo na porta
# Windows
netstat -ano | findstr :1433
taskkill /PID <PID> /F

# Linux/Mac
lsof -ti:1433 | xargs kill -9
```

### Erro: "Containers não ficam healthy"
```bash
# Ver logs dos containers
docker-compose -f docker-compose.test.yml logs

# Ver logs de um serviço específico
docker-compose -f docker-compose.test.yml logs sqlserver
docker-compose -f docker-compose.test.yml logs redis
docker-compose -f docker-compose.test.yml logs qdrant

# Restart forçado
docker-compose -f docker-compose.test.yml down -v
docker-compose -f docker-compose.test.yml up -d
```

### Erro: "Testes falham com timeout"
Aumentar timeout nos testes ou dar mais tempo para containers iniciarem:
```bash
# Esperar manualmente
docker-compose -f docker-compose.test.yml up -d
sleep 30  # ou Start-Sleep -Seconds 30 no PowerShell

# Então executar testes
cargo test --package jarvis-core --lib -- --ignored
```

### Erro: "SQL Server authentication failed"
Verificar se o container SQL Server está rodando corretamente:
```bash
# Testar conexão manualmente
docker exec -it jarvis-test-sqlserver /opt/mssql-tools/bin/sqlcmd \
  -S localhost -U sa -P 'YourPassword123!' -Q 'SELECT @@VERSION'
```

## 📊 Métricas de Execução

### Tempo Esperado
- **Startup de containers**: ~30-60 segundos
- **Execução de testes**: ~2-5 minutos
- **Cleanup**: ~10 segundos
- **Total**: ~3-6 minutos

### Recursos Utilizados
- **CPU**: 2-4 cores durante execução
- **RAM**: ~2-3GB para containers
- **Disk I/O**: Moderado (principalmente SQL Server)
- **Network**: Apenas localhost (sem tráfego externo)

## 🎯 Melhores Práticas

### Durante Desenvolvimento
1. **Use `-KeepContainers`** para evitar restart constante
2. **Filtre testes** para executar apenas o módulo em desenvolvimento
3. **Monitore logs** com `docker-compose logs -f <service>`

### Antes de Commit
1. **Execute todos os testes** sem filtros
2. **Verifique cleanup** - não deixe containers rodando
3. **Revise falhas** - não ignore testes falhando

### CI/CD
```yaml
# Exemplo GitHub Actions
- name: Start test services
  run: docker-compose -f jarvis-rs/docker-compose.test.yml up -d

- name: Wait for services
  run: sleep 30

- name: Run integration tests
  run: cd jarvis-rs && cargo test --lib -- --ignored

- name: Cleanup
  if: always()
  run: docker-compose -f jarvis-rs/docker-compose.test.yml down
```

## 🔐 Segurança

### Credenciais de Teste
- ⚠️  **NUNCA** use estas credenciais em produção
- ⚠️  Containers são apenas para testes locais
- ⚠️  Não exponha portas publicamente

### Isolamento
- Containers usam rede bridge isolada
- Sem acesso à rede externa
- Dados são efêmeros (não persistidos)

## 📚 Referências

- [Docker Compose Docs](https://docs.docker.com/compose/)
- [SQL Server Docker](https://hub.docker.com/_/microsoft-mssql-server)
- [Redis Docker](https://hub.docker.com/_/redis)
- [Qdrant Docker](https://hub.docker.com/r/qdrant/qdrant)
- [Cargo Test Docs](https://doc.rust-lang.org/cargo/commands/cargo-test.html)

## 🐛 Reportar Problemas

Se encontrar problemas com os testes de integração:

1. **Verifique os logs** dos containers
2. **Tente reiniciar** os serviços Docker
3. **Verifique recursos** disponíveis (RAM, CPU, Disk)
4. **Reporte** no issue tracker com logs completos

---

**Última Atualização**: 2026-02-07
**Mantido por**: Equipe Jarvis CLI
