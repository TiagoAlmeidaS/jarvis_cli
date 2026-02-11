# RAG Troubleshooting Guide

**Versão**: 1.0
**Data**: 2026-02-10
**Para**: Jarvis CLI com RAG

---

## 📋 Índice

1. [Diagnóstico Rápido](#diagnóstico-rápido)
2. [Problemas de Conexão](#problemas-de-conexão)
3. [Problemas de Indexação](#problemas-de-indexação)
4. [Problemas de Busca](#problemas-de-busca)
5. [Problemas de Performance](#problemas-de-performance)
6. [Logs e Debug](#logs-e-debug)
7. [Casos Específicos](#casos-específicos)

---

## Diagnóstico Rápido

### Checklist de Verificação Rápida

Execute esta sequência para diagnóstico inicial:

```bash
# 1. Verificar se Jarvis está instalado corretamente
jarvis --version

# 2. Verificar se comandos RAG estão disponíveis
jarvis context --help

# 3. Verificar conectividade com serviços
curl -s http://100.98.213.86:6333/collections
curl -s http://100.98.213.86:11434/api/tags
curl -s http://100.98.213.86:5432/

# 4. Ver estatísticas RAG
jarvis context stats

# 5. Testar indexação básica
echo "# Test Doc" > /tmp/test.md
jarvis context add /tmp/test.md

# 6. Testar busca
jarvis context search "test" -n 1
```

**Resultado esperado:**
- ✅ Todos os comandos executam sem erro
- ✅ Pelo menos 1 serviço responde (Qdrant ou Ollama)
- ✅ Documento de teste é indexado
- ✅ Busca retorna o documento

**Se falhar:** Continue para a seção específica do problema abaixo.

---

## Problemas de Conexão

### ⚠️ "Failed to connect to Qdrant"

#### Sintoma Completo
```
❌ Error adding document to context
   Failed to connect to Qdrant vector store
   Error: Connection refused (os error 111)
   Host: http://100.98.213.86:6333
```

#### Diagnóstico
```bash
# Testar conectividade
curl -v http://100.98.213.86:6333/collections

# Verificar se Qdrant está respondendo
curl http://100.98.213.86:6333/health
```

#### Causas Possíveis

**1. Qdrant está offline**
```bash
# Se você tem acesso ao servidor:
ssh user@100.98.213.86
sudo systemctl status qdrant
sudo systemctl start qdrant
```

**2. Firewall bloqueando**
```bash
# Verificar firewall local
sudo ufw status
# Porta 6333 deve estar aberta

# Testar de outra máquina
ping 100.98.213.86
telnet 100.98.213.86 6333
```

**3. Tailscale desconectado**
```bash
# Verificar conexão Tailscale
tailscale status
# Deve mostrar 100.98.213.86 como "active"

# Reconectar se necessário
tailscale up
```

#### Soluções

**Solução 1: Usar InMemory Fallback**
```bash
# RAG automaticamente usa InMemory se Qdrant falhar
export RUST_LOG=jarvis_core::rag=info
jarvis context add README.md
# Você verá: "Using InMemory vector store"
```

**Limitações do InMemory:**
- ⚠️ Dados perdidos ao reiniciar Jarvis
- ⚠️ Limitado pela RAM
- ✅ OK para testes e desenvolvimento

**Solução 2: Aguardar Qdrant Voltar**
```bash
# Monitorar status
watch -n 5 'curl -s http://100.98.213.86:6333/health'

# Quando voltar, re-indexar documentos
jarvis context add *.md --tags docs
```

---

### ⚠️ "Failed to connect to Ollama"

#### Sintoma Completo
```
❌ Failed to generate embedding
   Ollama service unavailable at http://100.98.213.86:11434
   Error: Connection timeout after 30s
```

#### Diagnóstico
```bash
# Testar conectividade
curl -v http://100.98.213.86:11434/api/tags

# Verificar modelo
curl http://100.98.213.86:11434/api/show -d '{
  "name": "nomic-embed-text"
}'
```

#### Causas Possíveis

**1. Ollama está offline**
```bash
# Se você tem acesso ao servidor:
ssh user@100.98.213.86
systemctl status ollama
sudo systemctl start ollama
```

**2. Modelo não está disponível**
```bash
# Listar modelos disponíveis
curl http://100.98.213.86:11434/api/tags | jq '.models[].name'

# Verificar se "nomic-embed-text" está na lista
```

**3. Ollama sobrecarregado**
```bash
# Verificar carga
ssh user@100.98.213.86
htop
# Verificar CPU/RAM do processo ollama
```

#### Soluções

**Solução 1: Usar Ollama Local**
```bash
# 1. Instalar Ollama localmente
# macOS/Linux:
curl https://ollama.ai/install.sh | sh

# Windows:
# Download de https://ollama.ai/download

# 2. Baixar modelo
ollama pull nomic-embed-text

# 3. Configurar Jarvis (futuro)
# Por ora, editar config:
export OLLAMA_HOST=http://localhost:11434
jarvis context add README.md
```

**Solução 2: Aguardar Ollama Voltar**
```bash
# Monitorar status
watch -n 10 'curl -s http://100.98.213.86:11434/api/tags | head -5'

# Quando voltar, indexar
jarvis context add docs/**/*.md
```

**Solução 3: Desabilitar RAG Temporariamente**
```bash
# Usar Jarvis sem RAG
jarvis exec "your query"
# (sem documentos indexados, RAG não será usado)
```

---

### ⚠️ "PostgreSQL connection failed"

#### Sintoma Completo
```
⚠️  PostgreSQL unavailable, using JSON file storage
   Error: Connection refused
   Falling back to: ~/.jarvis/documents.json
```

#### Diagnóstico
```bash
# Testar PostgreSQL
psql -h 100.98.213.86 -U jarvis -d jarvis_db -c '\l'

# Verificar conectividade
telnet 100.98.213.86 5432
```

#### Solução

**Esta é uma situação OK:**
- ✅ Sistema automaticamente usa JSON file
- ✅ Funcionalidade RAG mantida
- ✅ Persistência local funciona

**Se PostgreSQL é crítico:**
```bash
# Verificar credenciais
cat ~/.jarvis/config.toml | grep postgres

# Reconectar
ssh user@100.98.213.86
sudo systemctl restart postgresql
```

---

## Problemas de Indexação

### ⚠️ "Document too large"

#### Sintoma
```
❌ Document exceeds maximum size (10 MB)
   File: large_document.md (15 MB)
```

#### Soluções

**Solução 1: Dividir Documento**
```bash
# Dividir em partes de 5000 linhas
split -l 5000 large_document.md part_

# Indexar partes
for file in part_*; do
  jarvis context add "$file" --tags large-doc
done
```

**Solução 2: Extrair Seções Relevantes**
```bash
# Apenas indexar seções importantes
awk '/^## Important Section/,/^## Next Section/' large_document.md > important.md
jarvis context add important.md --tags extracted
```

**Solução 3: Aumentar Limite (futuro)**
```toml
# ~/.jarvis/config.toml
[rag]
max_document_size = "20MB"
```

---

### ⚠️ "Failed to chunk document"

#### Sintoma
```
❌ Chunking failed: Invalid UTF-8 sequence
   File: binary_file.exe
```

#### Causa
Tentativa de indexar arquivo binário.

#### Solução
```bash
# Apenas indexar arquivos de texto
jarvis context add src/**/*.rs   # ✅ Código fonte
jarvis context add docs/**/*.md  # ✅ Markdown
jarvis context add *.txt         # ✅ Texto

# Evitar binários
# ❌ *.exe, *.png, *.pdf, *.zip
```

---

### ⚠️ "Embedding generation timeout"

#### Sintoma
```
⚠️  Embedding generation timeout after 30s
   Chunk: 0/5
   Retrying...
```

#### Causas

1. **Ollama sobrecarregado**: Muitas requisições simultâneas
2. **Chunk muito grande**: >2000 caracteres
3. **Rede lenta**: Latência alta para VPS

#### Soluções

**Solução 1: Reduzir Tamanho dos Chunks**
```toml
# ~/.jarvis/config.toml (futuro)
[rag.chunking]
chunk_size = 300
chunk_overlap = 50
```

**Solução 2: Indexar Sequencialmente**
```bash
# Ao invés de:
jarvis context add docs/*.md  # Paralelo, pode sobrecarregar

# Fazer:
for file in docs/*.md; do
  jarvis context add "$file"
  sleep 2  # Aguardar entre documentos
done
```

**Solução 3: Usar Ollama Local**
```bash
# Instalar Ollama localmente (sem latência de rede)
ollama pull nomic-embed-text
export OLLAMA_HOST=http://localhost:11434
jarvis context add large_doc.md
```

---

## Problemas de Busca

### ⚠️ "No results found"

#### Sintoma
```bash
jarvis context search "authentication"
# 🔍 Searching context...
# Results: (0 results)
```

#### Diagnóstico
```bash
# 1. Verificar se há documentos
jarvis context list
# Se vazio: adicione documentos!

# 2. Verificar stats
jarvis context stats
# Total Documents: 0? Problema!

# 3. Tentar busca genérica
jarvis context search "the" --min-score 0.1
# Se funcionar: query original muito específica
```

#### Soluções

**Solução 1: Adicionar Documentos**
```bash
# Se não há documentos indexados
jarvis context add README.md
jarvis context add src/**/*.rs
jarvis context stats  # Verificar
```

**Solução 2: Reduzir min-score**
```bash
# Busca original (muito restrita)
jarvis context search "JWT authentication" --min-score 0.7
# Resultado: 0 results

# Busca mais ampla
jarvis context search "JWT authentication" --min-score 0.3
# Resultado: 3 results
```

**Solução 3: Usar Palavras-chave Diferentes**
```bash
# Ao invés de:
jarvis context search "JWT bearer tokens"  # Muito específico

# Tentar:
jarvis context search "authentication"      # Mais genérico
jarvis context search "auth"                # Ainda mais amplo
```

---

### ⚠️ "Search returns irrelevant results"

#### Sintoma
```bash
jarvis context search "database"
# Results:
# 1. [65%] README.md - "...data base system..."  # ❌ Falso positivo
# 2. [45%] config.toml - "...data: base_url..."  # ❌ Não relacionado
```

#### Solução

**Aumentar min-score:**
```bash
# Busca mais precisa
jarvis context search "database" --min-score 0.7
# Apenas resultados altamente relevantes
```

**Usar query mais específica:**
```bash
# Ao invés de:
jarvis context search "database"

# Tentar:
jarvis context search "PostgreSQL database connection"
```

---

## Problemas de Performance

### ⚠️ "RAG is slow"

#### Sintoma
```bash
jarvis exec "query"
# 🔮 RAG Context: 100 documents, 500 chunks
# [Aguardando 5-10 segundos...]
```

#### Diagnóstico
```bash
# Medir latência de cada componente
time curl -X POST http://100.98.213.86:11434/api/embeddings -d '{
  "model": "nomic-embed-text",
  "prompt": "test query"
}'
# Esperado: < 500ms

time curl http://100.98.213.86:6333/collections/jarvis/points/search -d '{
  "vector": [0.1, 0.2, ...],
  "limit": 5
}'
# Esperado: < 100ms
```

#### Soluções

**Solução 1: Reduzir max_chunks**
```bash
# Configuração (futuro)
# ~/.jarvis/config.toml
[rag]
max_chunks = 3  # Ao invés de 5
min_score = 0.7  # Apenas muito relevantes
```

**Solução 2: Limpar Documentos Antigos**
```bash
# Ver todos os documentos
jarvis context list

# Remover documentos não utilizados
jarvis context remove <OLD_DOC_ID> --force
```

**Solução 3: Usar Ollama Local**
```bash
# Eliminar latência de rede
ollama pull nomic-embed-text
export OLLAMA_HOST=http://localhost:11434
# Latência reduzida de 300ms → 50ms
```

---

### ⚠️ "High memory usage"

#### Sintoma
```bash
# Jarvis consumindo >2GB RAM
htop
# jarvis: 2.5 GB VIRT, 2.1 GB RES
```

#### Diagnóstico
```bash
# Verificar número de documentos
jarvis context stats
# Total Documents: 500+  # ⚠️ Muitos!
# Total Chunks: 5000+    # ⚠️ Muito grande!
```

#### Solução

**InMemory Store:**
- Cada embedding: ~3 KB (768 floats)
- 5000 chunks: ~15 MB embeddings
- 500 docs: ~5 MB metadata

**Se usando InMemory:**
```bash
# Reduzir número de documentos
jarvis context list | wc -l

# Remover documentos desnecessários
jarvis context remove <ID> --force
```

**Se usando Qdrant:**
```bash
# Verificar configuração
# Qdrant deveria manter dados externamente
# Memory alto pode indicar fallback para InMemory

# Verificar logs
RUST_LOG=jarvis_core::rag=debug jarvis exec "test"
# Procurar por: "Using InMemory vector store"
```

---

## Logs e Debug

### Habilitar Logs Detalhados

```bash
# Logs completos RAG
export RUST_LOG=jarvis_core::rag=debug
jarvis context add README.md

# Logs apenas erros
export RUST_LOG=jarvis_core::rag=error
jarvis exec "query"

# Logs de todos os componentes
export RUST_LOG=debug
jarvis context search "test"
```

### Interpretar Logs

#### Log Normal (Sucesso)
```
DEBUG jarvis_core::rag: Initializing RAG context injector
DEBUG jarvis_core::rag: Connected to Qdrant at 100.98.213.86:6333
DEBUG jarvis_core::rag: Connected to PostgreSQL
INFO  jarvis_core::rag: RAG system ready (Qdrant + PostgreSQL)

DEBUG jarvis_core::rag: Adding document: README.md
DEBUG jarvis_core::rag: Chunking document (config: size=500, overlap=100)
DEBUG jarvis_core::rag: Generated 3 chunks
DEBUG jarvis_core::rag: Generating embeddings (batch=3)
DEBUG jarvis_core::rag: Embeddings generated in 287ms
DEBUG jarvis_core::rag: Storing embeddings in Qdrant
DEBUG jarvis_core::rag: Storing document metadata in PostgreSQL
INFO  jarvis_core::rag: Document indexed successfully (id=doc-abc123)
```

#### Log com Fallback
```
WARN  jarvis_core::rag: Failed to connect to Qdrant: Connection refused
INFO  jarvis_core::rag: Using InMemory vector store as fallback
WARN  jarvis_core::rag: Failed to connect to PostgreSQL
INFO  jarvis_core::rag: Using JsonFile document store as fallback
INFO  jarvis_core::rag: RAG system ready (InMemory + JsonFile)
```

#### Log com Erro
```
ERROR jarvis_core::rag: Failed to generate embedding
ERROR jarvis_core::rag: Ollama error: Connection timeout after 30s
ERROR jarvis_core::rag: Document indexing failed: README.md
```

### Arquivos de Log

```bash
# Logs do sistema (se configurado)
tail -f ~/.jarvis/logs/jarvis.log

# Logs do Qdrant (no servidor)
ssh user@100.98.213.86
tail -f /var/log/qdrant/qdrant.log

# Logs do Ollama (no servidor)
ssh user@100.98.213.86
journalctl -u ollama -f
```

---

## Casos Específicos

### Caso 1: RAG não injeta contexto no chat

#### Problema
```bash
jarvis exec "How does authentication work in this project?"
# Resposta: "I don't have specific information about your project..."
# ❌ Não usou contexto dos documentos!
```

#### Diagnóstico Completo
```bash
# Passo 1: Verificar documentos
jarvis context stats
# Se Total Documents: 0 → Adicionar documentos!

# Passo 2: Verificar busca
jarvis context search "authentication" -n 3
# Se 0 results → Documentos não contêm info relevante

# Passo 3: Ver logs
RUST_LOG=jarvis_core::rag=debug jarvis exec "authentication"
# Procurar por:
# - "RAG Context: X documents" → Deve aparecer
# - "Found N relevant chunks" → Deve ser > 0
# - "Injecting context" → Deve aparecer
```

#### Possíveis Causas

**Causa 1: Nenhum documento indexado**
```bash
# Solução:
jarvis context add README.md
jarvis context add src/auth.rs
```

**Causa 2: Documentos não relacionados**
```bash
# Indexou apenas README, mas pergunta é sobre implementação
# Solução: Indexar código também
jarvis context add src/**/*.rs --tags code
```

**Causa 3: min_score muito alto**
```bash
# Configuração padrão: min_score=0.7
# Solução: Documentos podem ter score < 0.7
jarvis context search "authentication" --min-score 0.3
```

**Causa 4: RAG desabilitado**
```bash
# Verificar configuração (futuro)
cat ~/.jarvis/config.toml | grep 'rag'
# Se: enabled = false → Habilitar
```

---

### Caso 2: Qdrant está cheio

#### Problema
```
ERROR Failed to add embedding to Qdrant
ERROR Storage limit reached: 100,000 points
```

#### Solução
```bash
# Conectar ao Qdrant
curl -X DELETE http://100.98.213.86:6333/collections/jarvis/points \
  -H 'Content-Type: application/json' \
  -d '{"points": ["doc-old-1", "doc-old-2", ...]}'

# Ou deletar coleção inteira e recriar
curl -X DELETE http://100.98.213.86:6333/collections/jarvis

# Reindexar documentos importantes
jarvis context add important_docs/**/*.md
```

---

### Caso 3: Embeddings inconsistentes

#### Problema
```bash
jarvis context search "authentication"
# Results: documentos completamente não relacionados
```

#### Diagnóstico
```bash
# Verificar modelo usado
curl http://100.98.213.86:11434/api/show -d '{"name": "nomic-embed-text"}'

# Verificar dimensões no Qdrant
curl http://100.98.213.86:6333/collections/jarvis

# Deve mostrar:
# "vector_size": 768
```

#### Solução

**Se dimensões incorretas:**
```bash
# Deletar coleção
curl -X DELETE http://100.98.213.86:6333/collections/jarvis

# Reindexar tudo
jarvis context add docs/**/*.md
```

---

## Recursos Adicionais

### Links Úteis

- **Guia do Usuário RAG**: `docs/RAG-USER-GUIDE.md`
- **Documentação Qdrant**: https://qdrant.tech/documentation/
- **Documentação Ollama**: https://ollama.ai/docs

### Comandos de Diagnóstico Salvos

Criar um script de diagnóstico:

```bash
#!/bin/bash
# diagnose-rag.sh

echo "=== RAG Diagnostics ==="
echo ""

echo "1. Jarvis Version:"
jarvis --version
echo ""

echo "2. RAG Commands Available:"
jarvis context --help > /dev/null 2>&1 && echo "✅ Yes" || echo "❌ No"
echo ""

echo "3. Qdrant Status:"
curl -s http://100.98.213.86:6333/health && echo "✅ Online" || echo "❌ Offline"
echo ""

echo "4. Ollama Status:"
curl -s http://100.98.213.86:11434/api/tags > /dev/null && echo "✅ Online" || echo "❌ Offline"
echo ""

echo "5. PostgreSQL Status:"
timeout 2 bash -c '</dev/tcp/100.98.213.86/5432' 2>/dev/null && echo "✅ Online" || echo "❌ Offline"
echo ""

echo "6. Context Statistics:"
jarvis context stats
echo ""

echo "7. Test Indexing:"
echo "# Test" > /tmp/rag-test.md
jarvis context add /tmp/rag-test.md 2>&1 | grep -q "successfully" && echo "✅ OK" || echo "❌ Failed"
rm /tmp/rag-test.md
echo ""

echo "=== End Diagnostics ==="
```

---

**Última atualização**: 2026-02-10
**Versão**: 1.0
