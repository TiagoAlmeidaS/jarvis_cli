# Guia do Usuário - Sistema RAG do Jarvis CLI

**Versão**: 1.0
**Data**: 2026-02-10
**Autor**: Claude Sonnet 4.5

---

## 📚 Índice

1. [O que é RAG?](#o-que-é-rag)
2. [Como Funciona no Jarvis](#como-funciona-no-jarvis)
3. [Primeiros Passos](#primeiros-passos)
4. [Comandos Disponíveis](#comandos-disponíveis)
5. [Exemplos Práticos](#exemplos-práticos)
6. [Melhores Práticas](#melhores-práticas)
7. [Troubleshooting](#troubleshooting)
8. [FAQ](#faq)

---

## O que é RAG?

**RAG (Retrieval Augmented Generation)** é uma técnica que permite ao Jarvis buscar informações relevantes nos seus documentos antes de responder perguntas.

### Sem RAG ❌
```
Você: "Como funciona a autenticação neste projeto?"

Jarvis: "Não tenho informações específicas sobre o seu projeto.
Geralmente, autenticação pode ser implementada usando..."
```
*Resposta genérica, não usa seu código*

### Com RAG ✅
```
Você: "Como funciona a autenticação neste projeto?"

Jarvis: "Baseado no seu código em jarvis-rs/core/src/auth.rs:
A autenticação usa JWT tokens gerenciados pelo AuthManager.
O sistema suporta OAuth e API keys..."
```
*Resposta específica, cita seu código real!*

---

## Como Funciona no Jarvis

### Arquitetura Simplificada

```
┌─────────────────┐
│ Seus Documentos │
│  README.md      │
│  src/main.rs    │
│  docs/*.md      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   jarvis        │
│   context add   │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────┐
│  1. Divide em Pedaços       │
│     (Chunks de ~500 chars)  │
└────────┬────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│  2. Gera Embeddings         │
│     (Vetores 768D)          │
│     via Ollama              │
└────────┬────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│  3. Armazena no Qdrant      │
│     (Busca Semântica)       │
└─────────────────────────────┘

Quando você pergunta algo:

┌─────────────────┐
│ "Como funciona  │
│  autenticação?" │
└────────┬────────┘
         │
         ▼
┌──────────────────────────────┐
│  1. Gera embedding da query  │
└────────┬─────────────────────┘
         │
         ▼
┌──────────────────────────────┐
│  2. Busca chunks similares   │
│     (Cosine similarity)      │
└────────┬─────────────────────┘
         │
         ▼
┌──────────────────────────────┐
│  3. Injeta contexto no LLM   │
│     + sua pergunta           │
└────────┬─────────────────────┘
         │
         ▼
┌──────────────────────────────┐
│  Resposta usando seu código! │
└──────────────────────────────┘
```

---

## Primeiros Passos

### 1. Verificar Instalação

```bash
# Ver se o Jarvis está instalado
jarvis --version

# Ver se RAG está disponível
jarvis context --help
```

Se o comando `context` aparecer, RAG está disponível! ✅

### 2. Adicionar Seu Primeiro Documento

```bash
# Adicionar o README do projeto
jarvis context add README.md

# Você verá:
# 📄 Adding document to context...
# ─────────────────────────────────────────
#   Path: README.md
# 🔮 Generating embeddings...
#   Processing chunk 1/3...
# 💾 Saving document metadata...
# ✅ Document indexed successfully
# ─────────────────────────────────────────
#   ID: doc-abc123
#   Title: README
#   Chunks: 3
#   Size: 1024 bytes
#   Storage: Qdrant (VPS)
```

### 3. Verificar o que Foi Indexado

```bash
# Ver estatísticas
jarvis context stats

# Saída:
# 📊 Context Statistics
# ──────────────────────────
#   Total Documents: 1
#   Total Chunks: 3
#   Total Size: 1.0 KB
#   Avg Chunks/Doc: 3.0
```

### 4. Testar Busca Semântica

```bash
# Buscar informações sobre autenticação
jarvis context search "authentication" -n 3

# Saída:
# 🔍 Searching context...
# Results: (2 results)
#
# 1. Result [85.2% similarity]
#    Source: README.md
#    Chunk: chunk-README-2
#
# 2. Result [72.1% similarity]
#    Source: src/auth.rs
#    Chunk: chunk-auth-0
```

### 5. Usar no Chat!

```bash
# Modo não-interativo
jarvis exec "How does authentication work in this project?"

# Você verá:
# 🔮 RAG Context: 1 documents, 3 chunks
# [Resposta usando informações do README.md]
```

Ou modo interativo:
```bash
# Iniciar chat
jarvis

# No chat, digite:
> How does authentication work?

# Jarvis vai buscar contexto automaticamente e responder!
```

---

## Comandos Disponíveis

### `jarvis context add` - Adicionar Documentos

**Sintaxe:**
```bash
jarvis context add <ARQUIVO> [OPTIONS]
```

**Opções:**
- `--tags <TAGS>` - Tags separadas por vírgula (ex: `auth,code,rust`)
- `-t, --doc-type <TYPE>` - Tipo do documento (ex: `code`, `docs`, `markdown`)
- `-l, --language <LANG>` - Linguagem do código (ex: `rust`, `python`)
- `-o, --output <FORMAT>` - Formato de saída (`human` ou `json`)

**Exemplos:**
```bash
# Básico
jarvis context add README.md

# Com tags
jarvis context add src/auth.rs --tags authentication,security,rust

# Especificar tipo
jarvis context add docs/guide.md --doc-type docs --tags tutorial

# Saída JSON
jarvis context add config.toml -o json
```

---

### `jarvis context list` - Listar Documentos

**Sintaxe:**
```bash
jarvis context list [OPTIONS]
```

**Opções:**
- `-t, --doc-type <TYPE>` - Filtrar por tipo
- `--tag <TAG>` - Filtrar por tag
- `-o, --output <FORMAT>` - Formato de saída

**Exemplos:**
```bash
# Listar tudo
jarvis context list

# Apenas documentos de código
jarvis context list --doc-type code

# Apenas com tag "auth"
jarvis context list --tag auth

# Saída JSON
jarvis context list -o json
```

---

### `jarvis context search` - Buscar Conhecimento

**Sintaxe:**
```bash
jarvis context search <QUERY> [OPTIONS]
```

**Opções:**
- `-n, --limit <NUM>` - Número máximo de resultados (padrão: 5)
- `--min-score <SCORE>` - Score mínimo 0.0-1.0 (padrão: 0.3)
- `--show-source` - Mostrar informações da fonte (padrão: true)
- `-o, --output <FORMAT>` - Formato de saída

**Exemplos:**
```bash
# Busca básica
jarvis context search "authentication"

# Mais resultados
jarvis context search "RAG implementation" -n 10

# Score mais alto (mais específico)
jarvis context search "JWT tokens" --min-score 0.7

# Menos específico (mais resultados)
jarvis context search "database" --min-score 0.3

# JSON para parsing
jarvis context search "API" -o json
```

---

### `jarvis context stats` - Estatísticas

**Sintaxe:**
```bash
jarvis context stats [OPTIONS]
```

**Opções:**
- `-o, --output <FORMAT>` - Formato de saída

**Exemplos:**
```bash
# Estatísticas gerais
jarvis context stats

# JSON
jarvis context stats -o json
```

---

### `jarvis context remove` - Remover Documento

**Sintaxe:**
```bash
jarvis context remove <DOC_ID> [OPTIONS]
```

**Opções:**
- `-f, --force` - Não pedir confirmação

**Exemplos:**
```bash
# Remover com confirmação
jarvis context remove doc-abc123

# Forçar remoção
jarvis context remove doc-abc123 --force

# Obter ID para remover
jarvis context list  # ver IDs
jarvis context remove <ID>
```

---

## Exemplos Práticos

### Exemplo 1: Indexar Projeto Rust Completo

```bash
# Estrutura do projeto
# my-project/
# ├── README.md
# ├── Cargo.toml
# ├── src/
# │   ├── main.rs
# │   ├── auth.rs
# │   └── db.rs
# └── docs/
#     └── architecture.md

# Adicionar documentação
jarvis context add README.md --tags docs,overview
jarvis context add docs/architecture.md --tags docs,architecture

# Adicionar configuração
jarvis context add Cargo.toml --tags config,rust

# Adicionar código principal
jarvis context add src/main.rs --tags code,rust,entry-point
jarvis context add src/auth.rs --tags code,rust,authentication
jarvis context add src/db.rs --tags code,rust,database

# Verificar
jarvis context stats
# Deve mostrar 6 documentos
```

### Exemplo 2: Buscar e Perguntar

```bash
# 1. Buscar informações sobre autenticação
jarvis context search "authentication flow" -n 5

# 2. Verificar se encontrou algo relevante
# Se encontrou, perguntar detalhes:

jarvis exec "Explain how the authentication flow works in detail"

# Jarvis vai usar os chunks encontrados para dar uma resposta detalhada!
```

### Exemplo 3: Projeto com Múltiplas Linguagens

```bash
# Backend (Rust)
jarvis context add backend/src/**/*.rs --tags backend,rust,code

# Frontend (TypeScript)
jarvis context add frontend/src/**/*.ts --tags frontend,typescript,code

# Documentação
jarvis context add docs/**/*.md --tags documentation

# Configs
jarvis context add *.toml --tags config
jarvis context add *.json --tags config

# Buscar apenas backend
jarvis context list --tag backend

# Buscar apenas docs
jarvis context list --tag documentation
```

### Exemplo 4: Workflow Completo de Desenvolvimento

```bash
# 1. Início do dia - indexar código novo
jarvis context add src/new_feature.rs --tags feature,wip

# 2. Durante desenvolvimento - perguntar
jarvis exec "How should I integrate this with the existing auth system?"
# RAG vai buscar em auth.rs e responder com base no código existente

# 3. Code review - verificar padrões
jarvis exec "Does this code follow the same patterns as existing features?"
# RAG compara com código indexado

# 4. Documentação - gerar baseado no código
jarvis exec "Write documentation for the authentication module"
# RAG usa o código em auth.rs para gerar docs precisas

# 5. Fim do dia - adicionar docs criadas
jarvis context add docs/auth.md --tags docs,authentication
```

---

## Melhores Práticas

### ✅ DO: Faça Isso

1. **Indexe documentos importantes primeiro**
   ```bash
   # Prioridade alta
   jarvis context add README.md
   jarvis context add docs/architecture.md
   jarvis context add src/main.rs
   ```

2. **Use tags descritivas**
   ```bash
   jarvis context add auth.rs --tags auth,security,core,rust
   # Melhor que apenas: --tags code
   ```

3. **Mantenha documentos atualizados**
   ```bash
   # Se modificou auth.rs, re-indexe:
   jarvis context remove doc-auth-old --force
   jarvis context add src/auth.rs --tags auth,security
   ```

4. **Use min-score apropriado**
   ```bash
   # Busca específica: score alto
   jarvis context search "JWT validation" --min-score 0.7

   # Busca exploratória: score baixo
   jarvis context search "security" --min-score 0.3
   ```

5. **Verifique stats regularmente**
   ```bash
   jarvis context stats
   # Se tiver muitos docs (>100), considere limpar antigos
   ```

### ❌ DON'T: Evite Isso

1. **Não indexe arquivos binários**
   ```bash
   # ❌ Ruim
   jarvis context add image.png
   jarvis context add binary.exe

   # ✅ Bom - apenas texto
   jarvis context add README.md
   jarvis context add src/code.rs
   ```

2. **Não indexe arquivos gerados**
   ```bash
   # ❌ Evite
   jarvis context add node_modules/**
   jarvis context add target/**
   jarvis context add dist/**

   # ✅ Apenas código fonte
   jarvis context add src/**/*.rs
   ```

3. **Não use tags genéricas demais**
   ```bash
   # ❌ Pouco útil
   jarvis context add auth.rs --tags file,code

   # ✅ Específico e útil
   jarvis context add auth.rs --tags authentication,jwt,security,core
   ```

4. **Não ignore erros de indexação**
   ```bash
   # Se falhar:
   jarvis context add large.md
   # ⚠️ Failed to connect to Ollama

   # Verificar serviços:
   curl http://100.98.213.86:11434/api/tags
   curl http://100.98.213.86:6333/collections
   ```

---

## Troubleshooting

### Problema: "Failed to connect to Qdrant"

**Sintoma:**
```
⚠️  Failed to connect to Qdrant, using in-memory storage
   Error: Connection refused
```

**Solução:**
```bash
# 1. Verificar se Qdrant está rodando
curl http://100.98.213.86:6333/collections

# 2. Se não responder, o sistema vai usar InMemory (OK para testes)
# 3. Para produção, garantir que Qdrant está acessível
```

**Workaround:**
```bash
# Usar storage local (temporário)
export RUST_LOG=jarvis_core::rag=debug
jarvis context add README.md
# Sistema vai automaticamente usar InMemory
```

---

### Problema: "Failed to generate embedding"

**Sintoma:**
```
❌ Failed to generate embedding
   Error: Ollama service unavailable
```

**Solução:**
```bash
# 1. Verificar Ollama
curl http://100.98.213.86:11434/api/tags

# 2. Se não estiver rodando:
# Contatar admin do servidor VPS ou usar Ollama local:

# Instalar Ollama localmente
# https://ollama.ai/download

# Rodar modelo
ollama pull nomic-embed-text

# Configurar Jarvis para usar local (futuro)
# Por ora, aguardar VPS voltar
```

---

### Problema: RAG não está injetando contexto

**Sintoma:**
```bash
jarvis exec "How does auth work?"
# Resposta genérica, não usa código do projeto
```

**Diagnóstico:**
```bash
# 1. Verificar se há documentos
jarvis context list
# Se vazio: adicione documentos!

# 2. Verificar busca
jarvis context search "auth" --min-score 0.1
# Se nada encontrado: min_score muito alto ou documentos não relacionados

# 3. Ver logs
RUST_LOG=jarvis_core::rag=debug jarvis exec "auth"
# Deve mostrar: "Found N relevant chunks"
```

**Solução:**
```bash
# 1. Adicionar documentos relevantes
jarvis context add src/auth.rs

# 2. Usar query mais específica
jarvis exec "Explain the authentication implementation in auth.rs"

# 3. Baixar min_score
jarvis context search "authentication" --min-score 0.3
```

---

### Problema: "Document too large"

**Sintoma:**
```
❌ Document exceeds maximum size
```

**Solução:**
```bash
# 1. Dividir documento grande em partes
split -l 1000 large_file.md part_

# 2. Indexar partes separadamente
jarvis context add part_aa --tags large-doc,part1
jarvis context add part_ab --tags large-doc,part2

# Ou: extrair apenas seções relevantes
# e indexar separadamente
```

---

## FAQ

### P: Quanto custa usar RAG?

**R:** O RAG no Jarvis é **gratuito**! Usa serviços open-source (Qdrant, Ollama) rodando na VPS do projeto.

---

### P: Meus documentos são privados?

**R:** Sim! Os documentos ficam apenas:
- No seu computador (JSON file)
- Na VPS do projeto (se usar Qdrant/PostgreSQL)

Não são enviados para nenhum serviço externo de terceiros.

---

### P: Quantos documentos posso indexar?

**R:**
- **InMemory**: Limitado pela RAM (~1000 documentos pequenos)
- **Qdrant**: Praticamente ilimitado (milhões de vectors)
- **Recomendação**: 50-200 documentos para performance ideal

---

### P: RAG funciona offline?

**R:** Depende:
- ✅ **Sim** se usar InMemory storage e Ollama local
- ❌ **Não** se usar Qdrant/PostgreSQL na VPS (precisa de internet)

---

### P: Posso usar RAG com outros modelos?

**R:** Atualmente usa Ollama (nomic-embed-text). No futuro, poderá suportar:
- OpenAI embeddings
- Cohere embeddings
- Sentence Transformers locais

---

### P: Como remover todos os documentos?

**R:**
```bash
# Listar todos
jarvis context list -o json > docs.json

# Remover um por um
jarvis context list | grep "ID:" | awk '{print $2}' | while read id; do
    jarvis context remove "$id" --force
done

# Ou deletar arquivo JSON (se não usar PostgreSQL)
rm ~/.jarvis/documents.json
```

---

### P: RAG deixa o Jarvis mais lento?

**R:**
- **Latência adicional**: ~200-500ms por query
- **Busca**: ~100ms
- **Embedding generation**: ~100-300ms
- **Formatação**: ~10ms

**Total**: Quase imperceptível para o usuário!

---

### P: Posso desabilitar RAG?

**R:** Sim, em breve haverá flag:
```bash
# Futuro
jarvis exec "query" --no-rag
```

Por ora, simplesmente não adicione documentos e RAG ficará inativo.

---

## Próximos Passos

1. **Indexe seus documentos principais**
   ```bash
   jarvis context add README.md
   jarvis context add docs/**/*.md
   ```

2. **Teste busca semântica**
   ```bash
   jarvis context search "seu tema"
   ```

3. **Use no chat!**
   ```bash
   jarvis exec "pergunta sobre seu projeto"
   ```

4. **Explore comandos avançados**
   - Filtros por tag
   - JSON output para automação
   - Min-score para precisão

---

**Dúvidas?** Pergunte no chat:
```bash
jarvis
> How do I use RAG effectively?
```

O próprio Jarvis vai te ensinar usando este guia! 😄

---

**Versão**: 1.0
**Última atualização**: 2026-02-10
