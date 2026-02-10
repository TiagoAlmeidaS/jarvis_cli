# RAG e Context Management

## Visão Geral

O sistema de RAG (Retrieval Augmented Generation) e Context Management permite que o Jarvis CLI mantenha e utilize conhecimento do projeto de forma inteligente. O sistema indexa documentos, código e contexto de desenvolvimento, permitindo busca semântica e compressão de contexto para otimizar o uso de tokens do LLM.

O sistema inclui:
- **Indexação de Documentos**: Indexa código, documentação e arquivos do projeto
- **Vector Store**: Armazena embeddings para busca semântica
- **Context Compression**: Reduz contexto mantendo informações relevantes
- **Development Context Tracking**: Rastreia contexto de desenvolvimento ao longo do tempo

## Motivação

Problemas que o sistema resolve:

1. **Limite de Tokens**: LLMs têm limites de contexto; precisamos incluir apenas informação relevante
2. **Busca de Conhecimento**: Encontrar informações relevantes em grandes codebases
3. **Contexto Persistente**: Manter contexto entre sessões
4. **Otimização de Custo**: Reduzir tokens usados = reduzir custos
5. **Qualidade de Respostas**: Contexto relevante = respostas melhores

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│              RAG & Context Management                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Document     │───▶│ Vector       │───▶│ Retrieval    │ │
│  │ Indexer      │    │ Store        │    │ Engine       │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Chunking     │    │ Embedding     │    │ Context      │ │
│  │ Strategy     │    │ Generator     │    │ Compressor   │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
│  ┌──────────────┐                                          │
│  │ Development  │                                          │
│  │ Context      │                                          │
│  │ Tracker      │                                          │
│  └──────────────┘                                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Indexação**:
   - Documentos são lidos e divididos em chunks
   - Chunks são convertidos em embeddings
   - Embeddings são armazenados no vector store

2. **Busca**:
   - Query do usuário é convertida em embedding
   - Busca semântica no vector store
   - Retorna chunks mais relevantes

3. **Compressão**:
   - Contexto completo é analisado
   - Informações redundantes são removidas
   - Informações mais relevantes são mantidas
   - Contexto comprimido é retornado

### Integrações

- **LLM Gateway**: Usa contexto comprimido nas chamadas ao LLM
- **File System**: Lê e indexa arquivos do projeto
- **Git**: Rastreia mudanças e atualiza índice
- **Vector Database**: Armazena embeddings (opcional: Qdrant, Chroma, etc.)
  - **Qdrant**: Vector database recomendado para produção (veja [Integração Qdrant](./qdrant-integration.md))
  - **In-Memory**: Fallback automático quando Qdrant não está disponível

## Especificação Técnica

### APIs e Interfaces

```rust
// Document indexer trait
pub trait DocumentIndexer: Send + Sync {
    async fn index_document(
        &self,
        path: &Path,
        content: &str,
    ) -> Result<Vec<DocumentChunk>>;
    
    async fn index_directory(
        &self,
        dir: &Path,
        patterns: &[Glob],
    ) -> Result<IndexingResult>;
    
    async fn update_index(
        &self,
        path: &Path,
    ) -> Result<()>;
    
    async fn remove_from_index(
        &self,
        path: &Path,
    ) -> Result<()>;
}

// Vector store trait
pub trait VectorStore: Send + Sync {
    async fn add_chunks(
        &self,
        chunks: Vec<DocumentChunk>,
    ) -> Result<()>;
    
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>>;
    
    async fn search_by_embedding(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>>;
}

// Context compressor trait
pub trait ContextCompressor: Send + Sync {
    async fn compress(
        &self,
        context: &Context,
        max_tokens: usize,
    ) -> Result<CompressedContext>;
    
    async fn compress_with_relevance(
        &self,
        context: &Context,
        query: &str,
        max_tokens: usize,
    ) -> Result<CompressedContext>;
}

// Development context tracker trait
pub trait DevelopmentContextTracker: Send + Sync {
    async fn track_file_change(
        &self,
        path: &Path,
        change_type: ChangeType,
    ) -> Result<()>;
    
    async fn track_conversation(
        &self,
        thread_id: ThreadId,
        messages: &[Message],
    ) -> Result<()>;
    
    async fn get_recent_context(
        &self,
        limit: usize,
    ) -> Result<Vec<ContextEntry>>;
}
```

### Estruturas de Dados

```rust
pub struct DocumentChunk {
    pub id: Uuid,
    pub document_path: PathBuf,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub chunk_index: usize,
    pub embedding: Option<Vec<f32>>,
    pub metadata: HashMap<String, Value>,
}

pub struct SearchResult {
    pub chunk: DocumentChunk,
    pub score: f32,
    pub relevance: RelevanceScore,
}

pub enum RelevanceScore {
    High(f32),    // > 0.8
    Medium(f32),  // 0.5 - 0.8
    Low(f32),     // < 0.5
}

pub struct Context {
    pub chunks: Vec<DocumentChunk>,
    pub messages: Vec<Message>,
    pub file_contexts: Vec<FileContext>,
    pub total_tokens: usize,
}

pub struct CompressedContext {
    pub chunks: Vec<DocumentChunk>,
    pub messages: Vec<Message>,
    pub compression_ratio: f32,
    pub tokens_saved: usize,
}

pub struct FileContext {
    pub path: PathBuf,
    pub content: String,
    pub language: Option<String>,
    pub last_modified: DateTime<Utc>,
}

pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { old_path: PathBuf },
}

pub struct ContextEntry {
    pub timestamp: DateTime<Utc>,
    pub entry_type: ContextEntryType,
    pub content: String,
    pub metadata: HashMap<String, Value>,
}

pub enum ContextEntryType {
    FileChange { path: PathBuf },
    Conversation { thread_id: ThreadId },
    Command { command: String },
    Error { error: String },
}
```

### Algoritmos

#### Chunking Strategy

```rust
pub enum ChunkingStrategy {
    FixedSize {
        chunk_size: usize,
        overlap: usize,
    },
    Semantic {
        min_chunk_size: usize,
        max_chunk_size: usize,
        similarity_threshold: f32,
    },
    CodeAware {
        language: String,
        preserve_structure: bool,
    },
}
```

#### Context Compression

1. **Análise de Relevância**: Calcular relevância de cada chunk em relação à query
2. **Remoção de Redundância**: Identificar e remover informações duplicadas
3. **Sumarização**: Sumarizar chunks menos relevantes
4. **Priorização**: Manter chunks mais relevantes completos
5. **Validação**: Garantir que contexto comprimido ainda contém informação necessária

## Comandos CLI

### `jarvis context add <path>`

Adiciona arquivo ou diretório ao índice de contexto.

**Opções:**
- `--recursive, -r`: Adicionar recursivamente
- `--pattern <glob>`: Padrão de arquivos a incluir
- `--exclude <glob>`: Padrão de arquivos a excluir

**Exemplo:**
```bash
jarvis context add ./src
jarvis context add ./docs --recursive --pattern "*.md"
jarvis context add ./src --exclude "*.test.rs"
```

### `jarvis context list`

Lista todos os documentos indexados.

**Opções:**
- `--format <format>`: Formato de saída (table, json)
- `--limit <number>`: Limitar resultados

**Exemplo:**
```bash
jarvis context list
jarvis context list --format json --limit 50
```

### `jarvis context remove <path>`

Remove arquivo ou diretório do índice.

**Exemplo:**
```bash
jarvis context remove ./old-docs
```

### `jarvis context search <query>`

Busca semântica no contexto indexado.

**Opções:**
- `--limit, -l <number>`: Número de resultados (padrão: 10)
- `--threshold <score>`: Score mínimo de relevância
- `--format <format>`: Formato de saída

**Exemplo:**
```bash
jarvis context search "authentication"
jarvis context search "error handling" --limit 5 --threshold 0.7
```

### `jarvis context compress`

Testa compressão de contexto (para debug).

**Opções:**
- `--max-tokens <number>`: Tokens máximos no contexto comprimido
- `--query <query>`: Query para calcular relevância

**Exemplo:**
```bash
jarvis context compress --max-tokens 2000
jarvis context compress --max-tokens 1000 --query "refactoring"
```

### `jarvis context update`

Atualiza índice com mudanças recentes.

**Exemplo:**
```bash
jarvis context update
```

## Exemplos de Uso

### Exemplo 1: Indexar Projeto

```bash
# Indexar todo o projeto
$ jarvis context add . --recursive

# Indexar apenas código fonte
$ jarvis context add ./src --pattern "*.rs"

# Indexar documentação
$ jarvis context add ./docs --pattern "*.md"
```

### Exemplo 2: Busca Semântica

```bash
# Buscar informações sobre autenticação
$ jarvis context search "authentication"

# Resultados:
# 1. src/auth/mod.rs (score: 0.92)
#    - Implementação de JWT authentication
# 2. docs/auth.md (score: 0.87)
#    - Guia de autenticação
# 3. src/middleware/auth.rs (score: 0.81)
#    - Middleware de autenticação
```

### Exemplo 3: Uso Automático em Conversas

Durante conversas, o sistema automaticamente:
1. Busca contexto relevante baseado na query do usuário
2. Comprime contexto para caber no limite de tokens
3. Inclui contexto comprimido na chamada ao LLM

```
User: "Como funciona a autenticação neste projeto?"

Jarvis: [Busca contexto relevante]
        [Comprime para ~2000 tokens]
        [Inclui no prompt]
        "Baseado no código em src/auth/mod.rs..."
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialização
- `uuid` - IDs únicos
- `walkdir` - Navegação de diretórios
- `glob` - Padrões de arquivos
- `chrono` - Timestamps
- `anyhow` / `thiserror` - Error handling

**Crates para embeddings:**
- `candle-core` / `candle-transformers` - Para gerar embeddings localmente
- `reqwest` - Para usar APIs de embedding (OpenAI, etc.)

**Crates para vector store:**
- `qdrant-client` - Cliente Qdrant (recomendado para produção)
  - Veja [Integração Qdrant](./qdrant-integration.md) para detalhes completos
- `chromadb` - Cliente ChromaDB (alternativa)
- Ou implementar store simples em SQLite com `sqlx` (fallback)

### Desafios Técnicos

1. **Geração de Embeddings**: Como gerar embeddings eficientemente?
   - **Solução**: Usar modelos locais (sentence-transformers via candle) ou APIs
   - Cachear embeddings para evitar recálculo

2. **Vector Store**: Qual vector database usar?
   - **Solução**: Implementar suporte a Qdrant como padrão (veja [Integração Qdrant](./qdrant-integration.md))
   - Fallback automático para in-memory quando Qdrant não disponível
   - Qdrant oferece melhor performance e escalabilidade para produção

3. **Chunking Inteligente**: Como dividir código mantendo contexto?
   - **Solução**: Usar chunking code-aware que preserva estruturas
   - Considerar AST para chunking semântico

4. **Compressão de Contexto**: Como comprimir sem perder informação crítica?
   - **Solução**: Usar LLM para sumarizar chunks menos relevantes
   - Manter chunks altamente relevantes completos

5. **Atualização Incremental**: Como atualizar índice eficientemente?
   - **Solução**: Rastrear mudanças via Git hooks ou file watchers
   - Re-indexar apenas arquivos modificados

### Performance

- **Indexação Assíncrona**: Indexar em background
- **Cache de Embeddings**: Evitar recalcular embeddings
- **Lazy Loading**: Carregar vector store apenas quando necessário
- **Batch Operations**: Processar múltiplos chunks em batch

### Segurança

- **Validação de Paths**: Prevenir path traversal attacks
- **Limites de Tamanho**: Limitar tamanho de arquivos indexados
- **Sanitização**: Sanitizar conteúdo antes de indexar
- **Privacidade**: Não indexar arquivos sensíveis (.env, secrets, etc.)

## Roadmap de Implementação

### Fase 1: Core Infrastructure (Sprint 1)

- [ ] Definir estruturas de dados (`DocumentChunk`, `Context`, etc.)
- [ ] Implementar `DocumentIndexer` básico
- [ ] Implementar chunking strategies (fixed-size, code-aware)
- [ ] Criar comandos básicos (`add`, `list`, `remove`)

### Fase 2: Vector Store (Sprint 2)

- [ ] Implementar integração com Qdrant (veja [Integração Qdrant](./qdrant-integration.md))
- [ ] Implementar fallback para in-memory vector store
- [ ] Integrar geração de embeddings
- [ ] Implementar busca semântica
- [ ] Adicionar comando `search`

### Fase 3: Context Compression (Sprint 3)

- [ ] Implementar `ContextCompressor`
- [ ] Algoritmo de análise de relevância
- [ ] Sumarização de chunks
- [ ] Integração com LLM Gateway

### Fase 4: Development Context Tracking (Sprint 4)

- [ ] Implementar `DevelopmentContextTracker`
- [ ] Rastrear mudanças de arquivos
- [ ] Rastrear conversas e comandos
- [ ] Comando `update` para sincronização

### Fase 5: Otimizações (Sprint 5)

- [ ] Indexação incremental
- [ ] Cache de embeddings
- [ ] Otimizações de performance
- [ ] Integração com Git hooks

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Agentic/Context/ContextEngine.cs` - Engine principal de contexto
- `Jarvis.CLI/Services/ContextCompressor.cs` - Compressão de contexto
- `Jarvis.CLI/Services/DevelopmentContextTracker.cs` - Rastreamento de contexto
- `Jarvis.Infrastructure/Services/QdrantVectorStore.cs` - Implementação Qdrant
- `Jarvis.Infrastructure/DependencyInjection/InfrastructureInjection.cs` - Configuração Qdrant

### Documentação Relacionada

- [Integração Qdrant](./qdrant-integration.md) - Documentação completa da integração Qdrant
- [Visão Geral das Integrações](./integrations-overview.md) - Visão geral de todas as integrações

### Documentação Externa

- [RAG (Retrieval Augmented Generation)](https://www.pinecone.io/learn/retrieval-augmented-generation/)
- [Vector Databases](https://www.pinecone.io/learn/vector-database/)
- [Sentence Transformers](https://www.sbert.net/)
- [Candle (Rust ML)](https://github.com/huggingface/candle)
- [SQLite Vector Extension](https://github.com/asg017/sqlite-vector)

---

**Status**: 📝 Planejado  
**Prioridade**: 🔴 Alta  
**Última atualização**: 2026-01-20
