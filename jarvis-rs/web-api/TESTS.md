# Testes da API Web

Este documento descreve a estrutura de testes da API Web do Jarvis.

## Estrutura de Testes

### Testes Unitários

Os testes unitários estão organizados em módulos `#[cfg(test)]` dentro de cada arquivo:

#### `handlers/health.rs`
- ✅ `test_health_check` - Verifica que o endpoint retorna status OK e versão
- ✅ `test_health_check_no_auth_required` - Verifica que health check não requer autenticação

#### `handlers/config.rs`
- ✅ `test_get_config_with_auth` - Verifica que o endpoint retorna configuração com autenticação válida
- ✅ `test_get_config_requires_auth` - Verifica que o endpoint requer autenticação

#### `handlers/threads.rs`
- ✅ `test_list_threads` - Verifica que o endpoint retorna lista de threads (vazia por enquanto)
- ✅ `test_list_threads_requires_auth` - Verifica que o endpoint requer autenticação

#### `handlers/chat.rs`
- ✅ `test_chat_requires_auth` - Verifica que o endpoint requer autenticação
- ✅ `test_chat_invalid_json` - Verifica tratamento de JSON inválido
- ✅ `test_chat_missing_prompt` - Verifica tratamento de prompt ausente

#### `middleware/auth.rs`
- ✅ `test_auth_middleware_valid_token` - Verifica autenticação com token válido
- ✅ `test_auth_middleware_invalid_token` - Verifica rejeição de token inválido
- ✅ `test_auth_middleware_missing_header` - Verifica rejeição quando header está ausente
- ✅ `test_auth_middleware_skips_health` - Verifica que health check não requer auth
- ✅ `test_auth_middleware_skips_static_files` - Verifica que arquivos estáticos não requerem auth

### Testes de Integração

Os testes de integração estão em `tests/integration_test.rs`:

- ✅ `test_health_endpoint_integration` - Teste de integração do health check
- ✅ `test_config_endpoint_integration` - Teste de integração do config
- ✅ `test_threads_endpoint_integration` - Teste de integração do threads
- ✅ `test_auth_required_for_protected_endpoints` - Verifica que endpoints protegidos requerem auth
- ✅ `test_invalid_auth_token` - Verifica rejeição de token inválido

## Utilitários de Teste

### `test_utils.rs`

Módulo compartilhado com funções auxiliares para criar `AppState` de teste:

- `create_test_app_state()` - Cria AppState com API key padrão
- `create_test_app_state_with_api_key(api_key)` - Cria AppState com API key específica

## Executando os Testes

### Todos os testes
```bash
cd jarvis-rs
cargo test --package jarvis-web-api
```

### Apenas testes unitários
```bash
cargo test --package jarvis-web-api --lib
```

### Apenas testes de integração
```bash
cargo test --package jarvis-web-api --test integration_test
```

### Testes específicos
```bash
# Testes de health
cargo test --package jarvis-web-api handlers::health::tests

# Testes de autenticação
cargo test --package jarvis-web-api middleware::auth::tests
```

## Cobertura de Testes

### Endpoints Testados
- ✅ `/api/health` - Health check
- ✅ `/api/config` - Configuração
- ✅ `/api/threads` - Lista de threads
- ⚠️ `/api/chat` - Chat (testes básicos, sem mock do core)

### Middleware Testado
- ✅ Autenticação (validação de tokens)
- ✅ Exceções (health check, arquivos estáticos)

### Casos de Erro Testados
- ✅ Token ausente
- ✅ Token inválido
- ✅ JSON inválido
- ✅ Dados faltando

## Próximos Passos

### Melhorias Futuras
1. **Testes de Chat com Mock**: Criar mocks do `ThreadManager` para testar o handler de chat sem depender do core real
2. **Testes de Thread Resumption**: Testar resumo de threads existentes
3. **Testes de HTML Response**: Verificar que respostas HTML são formatadas corretamente
4. **Testes de CORS**: Verificar comportamento quando CORS está habilitado
5. **Testes de Performance**: Adicionar testes de carga/carga básicos

### Dependências de Teste

- `axum-test` - Para testar handlers Axum
- `tempfile` - Para criar diretórios temporários
- `pretty_assertions` - Para assertions mais legíveis
- `tokio-test` - Para utilitários de teste async
