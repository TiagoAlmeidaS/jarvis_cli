# Redis cache para embeddings e RAG (economia de tokens)

**Status:** Proposto  
**Objetivo:** Reduzir custo e latência reutilizando embeddings e resultados de RAG via Redis na VPS.

---

## Por que faz sentido

- **Embeddings:** O mesmo texto sempre gera o mesmo vetor. Hoje cada consulta RAG chama o Ollama para gerar o embedding da query; com cache (chave = hash do texto normalizado, valor = vetor, TTL ex.: 24h), evitamos chamadas repetidas → **economia de tokens e tempo**.
- **Resultados RAG:** Para a mesma query (ou muito similar), podemos devolver os chunks já recuperados por um período (ex.: 5–15 min), reduzindo buscas vetoriais e, em fluxos que reembedam a query, novas chamadas ao Ollama.
- **Redis na VPS:** Já temos Redis no `docker-compose.vps.yml` (100.98.213.86:6379). O core já usa Redis para analytics (`JARVIS_REDIS_URL`); estender para RAG mantém a infra única e centralizada.

---

## Estado atual

- **jarvis-rs:** Módulo `core/src/integrations/redis/cache.rs` com `DistributedCache` e `RedisCache`; usado em analytics e self-improvement. RAG (`rag/embeddings.rs`, `rag/chat_integration.rs`) **não** usa cache: toda query chama `embedding_gen.generate_embedding(query)`.
- **Config:** Não existe `[cache]` ou `redis_url` no `ConfigToml`; apenas `JARVIS_REDIS_URL` no CLI de analytics.

---

## Proposta de implementação (issue sugerida)

**Título:** `feat(rag): cache de embeddings em Redis para economia de tokens`

**Descrição:**

- Introduzir cache opcional de embeddings no pipeline RAG: antes de chamar o Ollama, consultar Redis com chave `embedding:{hash(normalized_text)}`; se existir, usar o vetor em cache; senão, gerar, persistir no Redis com TTL (ex.: 24h) e usar.
- Chave: hash SHA256 do texto normalizado (trim, lowercase ou outro critério estável); valor: JSON do vetor `Vec<f32>`.
- Config: usar `JARVIS_RAG_REDIS_URL` ou `JARVIS_REDIS_URL`; se não definido, comportamento atual (sem cache). TTL configurável (ex.: 86400 s).
- Opcional: cache de resultados de retrieval (query → top-k chunk IDs ou payloads) com TTL curto (ex.: 300 s).

**Critério de aceite:**

- Com Redis configurado, embeddings repetidos (mesmo texto) não disparam nova chamada ao Ollama; métrica ou log indica cache hit.
- Sem Redis ou com Redis indisponível, fluxo igual ao atual (sem cache).
- Doc atualizada (este arquivo e config/vps-tailscale.md).

**Referências:** [config/vps-tailscale.md](../config/vps-tailscale.md), [integrations/redis/cache.rs](jarvis-rs/core/src/integrations/redis/cache.rs).

---

**Última atualização:** 2026-03-11
