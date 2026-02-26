# Jarvis Daemon API Documentation

Documentação completa dos endpoints REST e WebSocket para monitoramento e controle do Jarvis Daemon.

## Autenticação

Todos os endpoints (exceto `/api/health` e `/ws/daemon`) requerem autenticação via Bearer token:

```
Authorization: Bearer <sua-api-key>
```

A API key é configurada no `config.toml`:

```toml
[api]
api_key = "sua-chave-secreta-aqui"
```

## Endpoints REST

### Status

#### GET /api/daemon/status

Retorna o status geral do daemon.

**Resposta:**
```json
{
  "pipelines": {
    "total": 5,
    "enabled": 3
  },
  "jobs": {
    "running": 2
  },
  "content": {
    "published_last_7d": 12
  },
  "proposals": {
    "pending": 1
  },
  "revenue": {
    "total_usd_30d": 245.50
  },
  "goals": {
    "active": 4,
    "at_risk": 1
  }
}
```

### Dashboard

#### GET /api/daemon/dashboard

Retorna dados agregados para o dashboard completo.

**Query Parameters:**
- `days` (opcional, padrão: 30): Período em dias para métricas
- `compact` (opcional, padrão: false): Versão compacta

**Resposta:**
```json
{
  "health": "HEALTHY",
  "metrics": {
    "views": 15000.0,
    "clicks": 450.0,
    "impressions": 20000.0,
    "ctr": 2.25,
    "revenue": {
      "total_usd": 245.50,
      "by_source": [
        {
          "source": "adsense",
          "total_usd": 200.00,
          "record_count": 10
        }
      ]
    }
  },
  "pipelines": [...],
  "goals": [...],
  "experiments": [...],
  "recent_content": [...],
  "jobs_24h": {
    "completed": 15,
    "failed": 2
  }
}
```

### Pipelines

#### GET /api/daemon/pipelines

Lista todos os pipelines.

**Resposta:**
```json
{
  "pipelines": [
    {
      "id": "seo-blog-1",
      "name": "SEO Blog Pipeline",
      "strategy": "seo_blog",
      "enabled": true,
      "schedule_cron": "0 3 * * *",
      "config": {...}
    }
  ]
}
```

#### GET /api/daemon/pipelines/:id

Obtém um pipeline específico.

#### POST /api/daemon/pipelines

Cria um novo pipeline.

**Body:**
```json
{
  "id": "seo-blog-1",
  "name": "SEO Blog Pipeline",
  "strategy": "seo_blog",
  "schedule_cron": "0 3 * * *",
  "config": {...},
  "max_retries": 3,
  "retry_delay_sec": 300
}
```

#### POST /api/daemon/pipelines/:id/enable

Ativa um pipeline.

#### POST /api/daemon/pipelines/:id/disable

Desativa um pipeline.

### Jobs

#### GET /api/daemon/jobs

Lista jobs.

**Query Parameters:**
- `pipeline` (opcional): Filtrar por pipeline ID
- `status` (opcional): Filtrar por status (pending, running, completed, failed, cancelled)
- `limit` (opcional, padrão: 20): Limite de resultados

**Resposta:**
```json
{
  "jobs": [
    {
      "id": "uuid",
      "pipeline_id": "seo-blog-1",
      "status": "completed",
      "attempt": 1,
      "started_at": 1709123456,
      "completed_at": 1709123556,
      "duration_ms": 100000,
      "created_at": 1709123456,
      "error_message": null
    }
  ]
}
```

#### GET /api/daemon/jobs/:id

Obtém um job específico.

### Métricas

#### GET /api/daemon/metrics/summary

Resumo de métricas.

**Query Parameters:**
- `days` (opcional, padrão: 30): Período em dias

**Resposta:**
```json
{
  "views": 15000.0,
  "clicks": 450.0,
  "impressions": 20000.0,
  "ctr": 2.25,
  "revenue": 245.50
}
```

#### GET /api/daemon/revenue/summary

Resumo de revenue.

**Query Parameters:**
- `pipeline` (opcional): Filtrar por pipeline
- `last_days` (opcional, padrão: 30): Período em dias

#### GET /api/daemon/revenue

Lista registros de revenue.

**Query Parameters:**
- `pipeline` (opcional): Filtrar por pipeline
- `last_days` (opcional, padrão: 30): Período em dias
- `limit` (opcional, padrão: 50): Limite de resultados

#### GET /api/daemon/content/:id/metrics

Métricas de um conteúdo específico.

### Goals

#### GET /api/daemon/goals

Lista goals.

**Query Parameters:**
- `pipeline` (opcional): Filtrar por pipeline
- `all` (opcional, padrão: false): Incluir goals inativos

#### GET /api/daemon/goals/:id

Obtém um goal específico.

#### POST /api/daemon/goals

Cria um novo goal.

**Body:**
```json
{
  "name": "Revenue Mensal",
  "description": "Receita mensal via AdSense",
  "metric": "revenue",
  "target": 200.0,
  "period": "monthly",
  "unit": "USD",
  "pipeline": null,
  "priority": 1
}
```

#### POST /api/daemon/goals/:id/pause

Pausa um goal.

#### POST /api/daemon/goals/:id/resume

Retoma um goal.

### Proposals

#### GET /api/daemon/proposals

Lista proposals.

**Query Parameters:**
- `pipeline` (opcional): Filtrar por pipeline
- `all` (opcional, padrão: false): Incluir proposals não-pendentes
- `limit` (opcional, padrão: 20): Limite de resultados

#### GET /api/daemon/proposals/:id

Obtém uma proposal específica.

#### POST /api/daemon/proposals/:id/approve

Aprova uma proposal.

#### POST /api/daemon/proposals/:id/reject

Rejeita uma proposal.

### Logs

#### GET /api/daemon/logs

Lista logs.

**Query Parameters:**
- `pipeline` (opcional): Filtrar por pipeline
- `job` (opcional): Filtrar por job ID
- `limit` (opcional, padrão: 50): Limite de resultados

**Resposta:**
```json
{
  "logs": [
    {
      "id": 1,
      "job_id": "uuid",
      "pipeline_id": "seo-blog-1",
      "level": "info",
      "message": "Job started",
      "context_json": null,
      "created_at": 1709123456
    }
  ]
}
```

## WebSocket

### /ws/daemon

Conexão WebSocket para atualizações em tempo real.

**Query Parameters:**
- `token` (opcional): API key para autenticação

**Mensagens Enviadas pelo Servidor:**
```json
{
  "event_type": "status_update",
  "data": {
    "pipelines": {
      "total": 5,
      "enabled": 3
    },
    "jobs": {
      "running": 2
    },
    "revenue": {
      "total_usd_30d": 245.50
    },
    "proposals": {
      "pending": 1
    }
  }
}
```

**Mensagens Aceitas do Cliente:**
- `"ping"` - Retorna `"pong"` para verificar conexão

O servidor envia atualizações a cada 5 segundos automaticamente.

## Códigos de Status HTTP

- `200 OK` - Sucesso
- `204 No Content` - Sucesso sem conteúdo (para enable/disable)
- `400 Bad Request` - Requisição inválida
- `401 Unauthorized` - API key inválida ou ausente
- `404 Not Found` - Recurso não encontrado
- `503 Service Unavailable` - Banco de dados do daemon não disponível

## Exemplos de Uso

### cURL

```bash
# Obter status
curl -H "Authorization: Bearer sua-api-key" \
  http://localhost:3000/api/daemon/status

# Listar pipelines
curl -H "Authorization: Bearer sua-api-key" \
  http://localhost:3000/api/daemon/pipelines

# Ativar pipeline
curl -X POST -H "Authorization: Bearer sua-api-key" \
  http://localhost:3000/api/daemon/pipelines/seo-blog-1/enable

# Aprovar proposal
curl -X POST -H "Authorization: Bearer sua-api-key" \
  http://localhost:3000/api/daemon/proposals/proposal-id/approve
```

### JavaScript

```javascript
const apiKey = 'sua-api-key';

// Obter dashboard
const response = await fetch('/api/daemon/dashboard?days=30', {
  headers: {
    'Authorization': `Bearer ${apiKey}`
  }
});
const data = await response.json();

// WebSocket
const ws = new WebSocket(`ws://localhost:3000/ws/daemon?token=${apiKey}`);
ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log('Update:', update);
};
```

## Dashboard Web

Acesse o dashboard web em:

```
http://localhost:3000/daemon
```

O dashboard fornece uma interface visual completa para:
- Visualizar status geral
- Gerenciar pipelines (ativar/desativar)
- Monitorar jobs em tempo real
- Acompanhar goals e progresso
- Aprovar/rejeitar proposals
- Visualizar logs
- Gráficos de métricas e revenue

O dashboard usa a mesma autenticação via API key (solicitada na primeira vez).
