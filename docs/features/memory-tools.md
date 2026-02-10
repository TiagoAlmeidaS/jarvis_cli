# Memory Tools

## Visão Geral

O sistema de Memory Tools permite que o Jarvis CLI armazene e recupere informações persistentes entre sessões. Isso permite que o LLM "lembre" de informações importantes sobre o projeto, preferências do usuário, e contexto histórico.

O sistema inclui:
- **Remember Tool**: Armazenar informações na memória
- **Memory Search Tool**: Buscar informações na memória
- **Persistência**: Memória persistente entre sessões
- **Busca Semântica**: Busca inteligente usando embeddings

## Motivação

Problemas que o sistema resolve:

1. **Persistência**: Manter informações entre sessões
2. **Contexto**: Lembrar preferências e contexto do projeto
3. **Eficiência**: Evitar repetir informações
4. **Personalização**: Adaptar comportamento baseado em histórico
5. **Conhecimento**: Acumular conhecimento sobre o projeto

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│                  Memory Tools System                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Remember     │───▶│ Memory       │───▶│ Memory       │ │
│  │ Tool         │    │ Storage      │    │ Search       │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Embedding    │    │ Vector       │    │ Relevance    │ │
│  │ Generator    │    │ Store        │    │ Scorer       │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Armazenamento**:
   - Informação é recebida via `remember` tool
   - Embedding é gerado
   - Informação + embedding são armazenados

2. **Busca**:
   - Query é recebida via `memory_search` tool
   - Embedding da query é gerado
   - Busca semântica no vector store
   - Resultados são retornados ordenados por relevância

3. **Persistência**:
   - Memórias são salvas em SQLite
   - Embeddings são armazenados no vector store
   - Sincronização entre storage e vector store

### Integrações

- **Vector Store**: Armazenamento de embeddings (pode usar mesmo do RAG)
- **SQLite**: Persistência de metadados
- **LLM Gateway**: Tools expostos ao LLM
- **RAG System**: Pode integrar com sistema RAG existente

## Especificação Técnica

### APIs e Interfaces

```rust
// Memory storage trait
pub trait MemoryStorage: Send + Sync {
    async fn store(
        &self,
        key: &str,
        content: &str,
        metadata: HashMap<String, Value>,
    ) -> Result<MemoryId>;
    
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryResult>>;
    
    async fn get(
        &self,
        memory_id: MemoryId,
    ) -> Result<Option<Memory>>;
    
    async fn delete(
        &self,
        memory_id: MemoryId,
    ) -> Result<()>;
}

// Remember tool
pub struct RememberTool {
    storage: Arc<dyn MemoryStorage>,
}

// Memory search tool
pub struct MemorySearchTool {
    storage: Arc<dyn MemoryStorage>,
}
```

### Estruturas de Dados

```rust
pub type MemoryId = Uuid;

pub struct Memory {
    pub id: MemoryId,
    pub key: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub access_count: u64,
    pub last_accessed: Option<DateTime<Utc>>,
}

pub struct MemoryResult {
    pub memory: Memory,
    pub relevance_score: f32,
}

pub struct RememberParams {
    pub key: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, Value>>,
}

pub struct MemorySearchParams {
    pub query: String,
    pub limit: Option<usize>,
    pub min_score: Option<f32>,
    pub tags: Option<Vec<String>>,
}
```

### Algoritmos

#### Busca Semântica

1. Gerar embedding da query
2. Buscar no vector store usando cosine similarity
3. Filtrar por tags se especificado
4. Filtrar por score mínimo
5. Ordenar por relevância
6. Retornar top N resultados

#### Gerenciamento de Memória

1. **Limpeza Automática**: Remover memórias antigas não acessadas
2. **Deduplicação**: Detectar memórias similares
3. **Atualização**: Atualizar memórias existentes com mesma key

## Comandos CLI

**Nota**: Memory tools são principalmente usados pelo LLM durante conversas. Comandos CLI podem ser adicionados para gerenciamento.

### `jarvis memory list`

Lista todas as memórias armazenadas.

**Opções:**
- `--limit <number>`: Limitar resultados
- `--tags <tags>`: Filtrar por tags

**Exemplo:**
```bash
jarvis memory list
jarvis memory list --tags "project,config"
```

### `jarvis memory search <query>`

Busca memórias por query.

**Exemplo:**
```bash
jarvis memory search "authentication"
```

### `jarvis memory delete <id>`

Deleta uma memória específica.

**Exemplo:**
```bash
jarvis memory delete "550e8400-e29b-41d4-a716-446655440000"
```

## Exemplos de Uso

### Exemplo 1: Armazenar Informação

Durante conversas, o LLM pode armazenar informações:

```
User: "Este projeto usa Rust 1.70 e tokio para async"

Jarvis: [Armazena na memória]
        ✓ Informação armazenada
```

### Exemplo 2: Buscar Informação

```
User: "Qual versão do Rust este projeto usa?"

Jarvis: [Busca na memória]
        "Este projeto usa Rust 1.70 e tokio para async"
```

### Exemplo 3: Uso Automático

O sistema automaticamente busca memórias relevantes durante conversas:

```
User: "Adicione uma função de autenticação"

Jarvis: [Busca memórias sobre autenticação]
        [Usa contexto encontrado]
        "Baseado nas preferências armazenadas..."
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `uuid` - IDs únicos
- `serde` / `serde_json` - Serialização
- `tokio` - Async runtime
- `sqlx` - Persistência em SQLite
- `anyhow` / `thiserror` - Error handling
- `chrono` - Timestamps

**Crates para embeddings:**
- Mesmos do sistema RAG (candle-core ou APIs)

**Crates para vector store:**
- Mesmos do sistema RAG (SQLite + extensão ou Qdrant)

### Desafios Técnicos

1. **Integração com RAG**: Como integrar com sistema RAG?
   - **Solução**: Usar mesmo vector store e embedding generator
   - Diferenciar memórias de documentos via metadata

2. **Deduplicação**: Como evitar memórias duplicadas?
   - **Solução**: Usar key como identificador único
   - Detectar similaridade alta e sugerir merge

3. **Limpeza**: Como gerenciar memórias antigas?
   - **Solução**: Política de limpeza baseada em acesso
   - Remover memórias não acessadas há muito tempo

4. **Privacidade**: Como proteger informações sensíveis?
   - **Solução**: Permitir marcar memórias como privadas
   - Criptografar conteúdo sensível

### Performance

- **Caching**: Cachear memórias frequentes
- **Lazy Loading**: Carregar embeddings apenas quando necessário
- **Batch Operations**: Processar múltiplas memórias em batch

### Segurança

- **Validação**: Validar conteúdo antes de armazenar
- **Criptografia**: Criptografar memórias sensíveis
- **Acesso**: Controlar acesso a memórias por contexto

## Roadmap de Implementação

### Fase 1: Core Infrastructure (Sprint 1)

- [ ] Definir estruturas de dados (`Memory`, `MemoryStorage`)
- [ ] Implementar `MemoryStorage` básico (SQLite)
- [ ] Implementar `RememberTool`
- [ ] Implementar `MemorySearchTool`

### Fase 2: Vector Search (Sprint 2)

- [ ] Integrar com sistema de embeddings
- [ ] Implementar busca semântica
- [ ] Adicionar scoring de relevância
- [ ] Otimizar performance

### Fase 3: Management (Sprint 3)

- [ ] Comandos CLI para gerenciamento
- [ ] Deduplicação de memórias
- [ ] Limpeza automática
- [ ] Tags e metadata

### Fase 4: Integration (Sprint 4)

- [ ] Integração com RAG system
- [ ] Uso automático em conversas
- [ ] Analytics de uso
- [ ] Export/import de memórias

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Tools/RememberTool.cs` - Tool de armazenamento
- `Jarvis.CLI/Tools/MemorySearchTool.cs` - Tool de busca
- `Jarvis.CLI/Tools/MemoryTool.cs` - Tool base

### Documentação Externa

- [RAG System](./rag-context-management.md) - Sistema RAG relacionado
- [Vector Databases](https://www.pinecone.io/learn/vector-database/)
- [SQLite Vector Extension](https://github.com/asg017/sqlite-vector)

---

**Status**: 📝 Planejado  
**Prioridade**: 🟢 Baixa  
**Última atualização**: 2026-01-20
