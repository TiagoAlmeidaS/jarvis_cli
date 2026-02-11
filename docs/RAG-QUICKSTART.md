# RAG Quick Start Guide

**5 minutos para começar a usar RAG no Jarvis CLI** 🚀

---

## O que é RAG?

RAG (Retrieval Augmented Generation) permite que o Jarvis busque informações nos seus documentos antes de responder. Isso significa:

- ✅ Respostas específicas sobre SEU código
- ✅ Menos "copiar e colar" contexto
- ✅ Jarvis "lembra" do que você indexou

---

## Instalação

### Requisitos

```bash
# 1. Jarvis CLI instalado
jarvis --version

# 2. Rust features habilitadas
cd jarvis-rs
cargo build --release --features qdrant,postgres
```

### Verificação

```bash
# Ver se comando 'context' existe
jarvis context --help

# Se aparecer, RAG está instalado! ✅
```

---

## Primeiros Passos (5 minutos)

### 1️⃣ Adicionar Seu Primeiro Documento (30 segundos)

```bash
# Adicionar o README do projeto
jarvis context add README.md

# Você verá:
# 📄 Adding document to context...
# 🔮 Generating embeddings...
# ✅ Document indexed successfully
```

### 2️⃣ Verificar o que Foi Indexado (10 segundos)

```bash
# Ver estatísticas
jarvis context stats

# Saída:
# 📊 Context Statistics
# ─────────────────────
#   Total Documents: 1
#   Total Chunks: 3
#   Total Size: 1.2 KB
```

### 3️⃣ Testar Busca Semântica (20 segundos)

```bash
# Buscar algo no documento
jarvis context search "features" -n 3

# Saída:
# 🔍 Searching context...
# Results: (2 results)
#
# 1. Result [87.5% similarity]
#    Source: README.md
#    Content: "## Features..."
```

### 4️⃣ Usar no Chat! (2 minutos)

```bash
# Modo não-interativo
jarvis exec "What is this project about?"

# Você verá:
# 🔮 RAG Context: 1 documents, 3 chunks
# [Resposta usando informações do README]
```

**OU** modo interativo:

```bash
# Iniciar chat
jarvis

# No chat, perguntar:
> What features does this project have?

# Jarvis vai buscar no README automaticamente!
```

### 🎉 Pronto!

Você já está usando RAG! O Jarvis agora pode responder perguntas sobre seus documentos.

---

## Próximos Passos

### Indexar Mais Arquivos

```bash
# Documentação
jarvis context add docs/**/*.md --tags documentation

# Código fonte
jarvis context add src/**/*.rs --tags code,rust

# Configuração
jarvis context add Cargo.toml --tags config
```

### Ver Todos os Documentos

```bash
# Listar documentos indexados
jarvis context list

# Saída:
# 📚 Documents in context (3 total):
#
# 1. README.md
#    ID: doc-abc123
#    Chunks: 3
#    Tags: documentation
#
# 2. src/main.rs
#    ID: doc-def456
#    Chunks: 5
#    Tags: code, rust
# ...
```

### Buscar Conhecimento

```bash
# Buscar informações específicas
jarvis context search "authentication" -n 5

# Com filtro de score
jarvis context search "database" --min-score 0.7
```

---

## Exemplos Práticos

### Exemplo 1: Perguntar sobre Arquitetura

```bash
# Indexar docs de arquitetura
jarvis context add docs/architecture.md

# Perguntar
jarvis exec "Explain the system architecture"

# Resposta vai usar o documento indexado!
```

### Exemplo 2: Entender Código

```bash
# Indexar módulo de autenticação
jarvis context add src/auth.rs --tags auth,security

# Perguntar detalhes
jarvis exec "How does the JWT validation work?"

# Jarvis vai citar o código em auth.rs!
```

### Exemplo 3: Comparar Padrões

```bash
# Indexar múltiplos arquivos
jarvis context add src/**/*.rs --tags code

# Perguntar sobre padrões
jarvis exec "Do all my Rust files follow the same error handling pattern?"

# Jarvis compara os arquivos indexados!
```

---

## Comandos Essenciais

### Adicionar Documento
```bash
jarvis context add <ARQUIVO> [--tags TAG1,TAG2]
```

### Listar Documentos
```bash
jarvis context list [--tag TAG]
```

### Buscar
```bash
jarvis context search "query" [-n NUM] [--min-score SCORE]
```

### Estatísticas
```bash
jarvis context stats
```

### Remover Documento
```bash
jarvis context remove <DOC_ID> [--force]
```

---

## Dicas Rápidas

### ✅ DO - Faça Isso

1. **Indexe documentos importantes primeiro**
   ```bash
   jarvis context add README.md
   jarvis context add docs/getting-started.md
   ```

2. **Use tags descritivas**
   ```bash
   jarvis context add auth.rs --tags auth,security,core
   ```

3. **Mantenha atualizado**
   ```bash
   # Se modificou um arquivo, re-indexe:
   jarvis context remove <OLD_ID> --force
   jarvis context add src/auth.rs
   ```

### ❌ DON'T - Evite Isso

1. **Não indexe binários**
   ```bash
   # ❌ Não funciona
   jarvis context add image.png
   jarvis context add binary.exe
   ```

2. **Não indexe node_modules ou target/**
   ```bash
   # ❌ Muitos arquivos, pouco valor
   jarvis context add node_modules/**
   jarvis context add target/**
   ```

3. **Não use tags genéricas**
   ```bash
   # ❌ Pouco útil
   jarvis context add file.rs --tags file

   # ✅ Muito melhor
   jarvis context add file.rs --tags auth,jwt,security
   ```

---

## Troubleshooting Rápido

### Problema: "Failed to connect to Qdrant"

```bash
# Sistema automaticamente usa InMemory
# Funciona, mas dados são temporários
# ✅ OK para desenvolvimento
```

### Problema: "Failed to connect to Ollama"

```bash
# Instalar Ollama localmente
curl https://ollama.ai/install.sh | sh
ollama pull nomic-embed-text

# Configurar
export OLLAMA_HOST=http://localhost:11434
jarvis context add README.md
```

### Problema: "No results found"

```bash
# Adicione documentos primeiro!
jarvis context stats
# Se Total Documents: 0, adicione:
jarvis context add *.md
```

### Problema: RAG não está injetando contexto

```bash
# 1. Verificar se há docs
jarvis context list

# 2. Testar busca
jarvis context search "seu query" -n 3

# 3. Ver logs
RUST_LOG=jarvis_core::rag=debug jarvis exec "query"
```

---

## Serviços Usados

RAG usa 3 serviços (com fallbacks automáticos):

### Qdrant (Vector Store)
- **Host**: http://100.98.213.86:6333
- **Fallback**: InMemory (temporário)
- **Uso**: Busca semântica

### Ollama (Embeddings)
- **Host**: http://100.98.213.86:11434
- **Modelo**: nomic-embed-text (768D)
- **Fallback**: Nenhum (RAG desabilita se falhar)
- **Uso**: Gerar embeddings vetoriais

### PostgreSQL (Metadata)
- **Host**: http://100.98.213.86:5432
- **Fallback**: JsonFile → InMemory
- **Uso**: Metadados dos documentos

**Fallbacks garantem que RAG sempre funcione!**

---

## Workflow Recomendado

### Início do Projeto

```bash
# 1. Indexar docs principais
jarvis context add README.md --tags docs,overview
jarvis context add docs/**/*.md --tags documentation

# 2. Indexar código core
jarvis context add src/main.rs --tags code,entry
jarvis context add src/lib.rs --tags code,library

# 3. Verificar
jarvis context stats
# Esperado: 5-10 documentos
```

### Durante Desenvolvimento

```bash
# Adicionar novas features
jarvis context add src/new_feature.rs --tags feature,wip

# Perguntar sobre integração
jarvis exec "How should this integrate with existing auth?"
```

### Code Review

```bash
# Verificar padrões
jarvis exec "Does this code follow our error handling patterns?"

# Buscar exemplos
jarvis context search "error handling" -n 5
```

---

## Recursos Adicionais

- **Guia Completo**: `docs/RAG-USER-GUIDE.md`
- **Troubleshooting**: `docs/RAG-TROUBLESHOOTING.md`
- **Testes E2E**: `tests/test-rag-e2e.sh`

---

## Checklist de Sucesso

Depois de seguir este guia, você deve conseguir:

- [ ] ✅ Adicionar um documento
- [ ] ✅ Ver estatísticas (`jarvis context stats`)
- [ ] ✅ Buscar semanticamente (`jarvis context search`)
- [ ] ✅ Obter resposta usando RAG (`jarvis exec "query"`)
- [ ] ✅ Listar documentos indexados

**Se todos os itens estão ✅, parabéns! RAG está funcionando! 🎉**

---

**Tempo total**: ~5 minutos
**Dificuldade**: Fácil
**Última atualização**: 2026-02-10
