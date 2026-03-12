# Configurar Jarvis CLI para usar a VPS via Tailscale

Objetivo: usar a infraestrutura na VPS (Qdrant, Postgres, **Redis**) no IP **100.98.213.86** para cache, analytics e, quando habilitado, RAG. Por enquanto **não usamos Ollama**; as funcionalidades de cache e economia de tokens ficam mapeadas no **Redis**.

---

## Como está a integração hoje

| Serviço   | O Jarvis CLI já usa? | Como |
|-----------|----------------------|------|
| **Qdrant**   | **Sim** (quando RAG está ativo) | O config carregado de `~/.jarvis/config.toml` (ou `.jarvis/config.toml` do projeto) inclui `[rag]`. O binário (TUI e `jarvis` exec) usa `config.rag.qdrant_url` para construir o RAG: vetores vêm do Qdrant. Se `[rag]` tiver `qdrant_url = "http://100.98.213.86:6333"`, o CLI **já olha para o Qdrant na VPS**. |
| **Postgres** | **Sim** (quando RAG está ativo) | O mesmo `[rag]` tem `postgres_url`. O core usa isso para o repositório de documentos do RAG. Com `postgres_url` apontando para a VPS, o CLI **já usa o Postgres na VPS** para documentos. |
| **Redis**    | **Só no comando analytics** | Não existe `[cache]` no config. O fluxo principal (TUI, chat) **não** lê Redis do config. O Redis só é usado quando você roda `jarvis analytics ...` e informa `--redis-url` ou `JARVIS_REDIS_URL`. Para o CLI “olhar” para o Redis na VPS, use a variável de ambiente (veja abaixo). |

Resumo: **Qdrant e Postgres** são integrados via `[rag]` no config — ao colar o `config.toml.vps` (e ajustar a senha do Postgres), ao reativar RAG (`enabled = true`) com um provedor de embeddings, o CLI passa a usar Qdrant e Postgres na VPS. **Redis** hoje só é usado no subcomando analytics; para o resto do app usar Redis (ex.: cache de embeddings no futuro) seria uma evolução do core.

## Pré-requisitos

1. **Tailscale** instalado e conectado na sua máquina e na VPS (mesma rede).
2. **Infra na VPS** no ar:
   ```bash
   # Na VPS
   docker compose -f docker-compose.vps.yml up -d
   ```
   Serviços: Qdrant (6333), Postgres (5432), Redis (6379), Adminer (8080).

3. **Senha do Postgres** definida (ex.: no `.env` da VPS: `POSTGRES_PASSWORD=...`). Use a mesma no `postgres_url` do config.

## Opção 1: Config no arquivo (~/.jarvis/config.toml)

Copie o conteúdo de **config.toml.vps** (na raiz do repo) para `~/.jarvis/config.toml` ou mescle as seções no seu config atual.

- **postgres_url** (em `[rag]`): troque `jarvis_secure_password_change_me` pela senha real do Postgres na VPS. O RAG fica com `enabled = false` por enquanto (sem Ollama); Qdrant e Postgres já apontam para a VPS para quando reativar.
- **Redis**: use variável de ambiente (veja seção Redis abaixo); não existe `[cache]` no config.

## Opção 2: Variáveis de ambiente

Para Redis (cache e analytics) — **recomendado**:

```bash
export JARVIS_REDIS_URL="redis://100.98.213.86:6379"
```

Para quando for usar RAG de novo (Qdrant + Postgres na VPS):

```bash
export JARVIS_RAG_QDRANT_URL="http://100.98.213.86:6333"
export JARVIS_RAG_POSTGRES_URL="postgres://jarvis:SUA_SENHA@100.98.213.86:5432/jarvis?sslmode=disable"
```

## Verificação

1. **Tailscale**: `ping 100.98.213.86` deve responder.
2. **Redis**: `redis-cli -h 100.98.213.86 PING` deve retornar `PONG` (uso principal por enquanto).
3. **Qdrant**: `curl http://100.98.213.86:6333/healthz` (para quando RAG estiver ativo).
4. **Postgres**: `psql "postgres://jarvis:...@100.98.213.86:5432/jarvis?sslmode=disable" -c 'SELECT 1'` ou Adminer em `http://100.98.213.86:8080`.

Com RAG desligado por enquanto, o uso principal da VPS é **Redis** (cache e analytics). Quando reativar RAG (com um provedor de embeddings), Qdrant e Postgres na VPS passam a ser usados.

## Redis — funcionalidades mapeadas (cache e analytics)

**Faz sentido?** Sim. Usar Redis para cache permite:

- **Cache de embeddings**: o mesmo texto gera o mesmo vetor; em vez de chamar o Ollama toda vez, consultamos o Redis (chave = hash do texto, TTL ex.: 24h). Menos chamadas ao modelo de embeddings = **economia de tokens e latência**.
- **Cache de resultados RAG**: para a mesma query (ou query muito parecida), devolver os chunks já recuperados por um tempo curto (ex.: 5–15 min), evitando nova busca vetorial e, em alguns fluxos, nova geração de embedding da query.

Hoje o jarvis-rs usa Redis apenas no comando **analytics** (`jarvis analytics ... --redis-url` ou `JARVIS_REDIS_URL`). O módulo de RAG **ainda não** usa Redis para cache de embeddings; isso seria uma melhoria futura (issue sugerida em docs/issues). Mesmo assim, já vale configurar o Redis na VPS para:

1. Usar `jarvis analytics` com cache na VPS.
2. Deixar pronto para quando o core ganhar cache de embeddings/retrieval em Redis.

**Configuração:** variável de ambiente (não existe `[cache]` no config.toml do Rust hoje):

```bash
export JARVIS_REDIS_URL="redis://100.98.213.86:6379"
```

Se o Redis na VPS tiver senha: `redis://:SENHA@100.98.213.86:6379`.

---

## Resumo dos endpoints (VPS 100.98.213.86)

| Serviço   | Porta | Uso no Jarvis        |
|----------|-------|----------------------|
| Qdrant   | 6333  | RAG – vetores        |
| Postgres | 5432  | RAG – documentos     |
| Redis    | 6379  | Analytics hoje; cache de embeddings/RAG (futuro) – economia de tokens |
| Adminer  | 8080  | UI para Postgres     |

## Referências

- [config.toml.vps](../../config.toml.vps) — exemplo de config
- [docker-compose.vps.yml](../../docker-compose.vps.yml) — definição da infra na VPS
- [RAG / Qdrant no core](jarvis-rs/core): `RagConfigToml` em `core/src/config/types.rs` (qdrant_url, postgres_url)
