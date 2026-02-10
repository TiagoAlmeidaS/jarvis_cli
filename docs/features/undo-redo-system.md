# Sistema Undo/Redo

## Visão Geral

O sistema de Undo/Redo permite que usuários desfaçam e refaçam mudanças feitas pelo Jarvis CLI. O sistema mantém um histórico de snapshots do estado do workspace, permitindo navegação temporal através das mudanças.

O sistema suporta:
- **Undo**: Desfazer última operação ou múltiplas operações
- **Redo**: Refazer operações desfeitas
- **Histórico**: Visualizar histórico de mudanças
- **Snapshots**: Capturas de estado do workspace em pontos específicos

## Motivação

Problemas que o sistema resolve:

1. **Segurança**: Permite reverter mudanças indesejadas
2. **Confiança**: Usuários podem experimentar sem medo
3. **Produtividade**: Evita ter que refazer trabalho manualmente
4. **Debugging**: Permite investigar o que mudou e quando
5. **Experimentation**: Facilita tentar diferentes abordagens

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│                  Undo/Redo System                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Edit         │───▶│ Snapshot     │───▶│ History      │ │
│  │ History      │    │ Manager      │    │ Manager      │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Snapshot     │    │ Workspace    │    │ File         │ │
│  │ Storage      │    │ State        │    │ Operations   │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Captura de Snapshot**:
   - Antes de cada operação, captura estado atual
   - Após operação bem-sucedida, salva novo snapshot
   - Snapshot inclui: arquivos modificados, conteúdo, metadados

2. **Undo**:
   - Restaura snapshot anterior
   - Aplica mudanças reversas
   - Move ponteiro de histórico para trás

3. **Redo**:
   - Restaura snapshot futuro
   - Aplica mudanças novamente
   - Move ponteiro de histórico para frente

4. **Limpeza**:
   - Remove snapshots antigos além do limite
   - Mantém apenas histórico relevante

### Integrações

- **File Operations**: Rastreia operações de arquivo (create, update, delete)
- **Git Operations**: Integra com Git quando aplicável
- **Session Management**: Mantém histórico por sessão
- **State Runtime**: Usa persistência existente

## Especificação Técnica

### APIs e Interfaces

```rust
// Edit history trait
pub trait EditHistory: Send + Sync {
    async fn capture_snapshot(
        &mut self,
        operation: Operation,
    ) -> Result<SnapshotId>;
    
    async fn undo(
        &mut self,
        steps: usize,
    ) -> Result<Vec<FileChange>>;
    
    async fn redo(
        &mut self,
        steps: usize,
    ) -> Result<Vec<FileChange>>;
    
    async fn get_history(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<HistoryEntry>>;
    
    async fn clear_history(
        &mut self,
        keep_recent: usize,
    ) -> Result<()>;
}

// Snapshot manager trait
pub trait SnapshotManager: Send + Sync {
    async fn create_snapshot(
        &self,
        workspace_path: &Path,
    ) -> Result<EditSnapshot>;
    
    async fn restore_snapshot(
        &self,
        snapshot: &EditSnapshot,
        workspace_path: &Path,
    ) -> Result<()>;
    
    async fn diff_snapshots(
        &self,
        old: &EditSnapshot,
        new: &EditSnapshot,
    ) -> Result<Vec<FileChange>>;
}
```

### Estruturas de Dados

```rust
pub struct EditSnapshot {
    pub id: SnapshotId,
    pub timestamp: DateTime<Utc>,
    pub operation: Operation,
    pub files: Vec<FileState>,
    pub metadata: HashMap<String, Value>,
}

pub type SnapshotId = Uuid;

pub struct FileState {
    pub path: PathBuf,
    pub content: Option<String>,  // None se arquivo não existia
    pub hash: String,             // SHA-256 hash do conteúdo
    pub metadata: FileMetadata,
}

pub struct FileMetadata {
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub permissions: Option<u32>,
}

pub enum Operation {
    FileEdit {
        path: PathBuf,
        change_type: FileChangeType,
    },
    BatchEdit {
        changes: Vec<FileChange>,
    },
    GitOperation {
        command: String,
        args: Vec<String>,
    },
    Custom {
        name: String,
        description: String,
    },
}

pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { old_path: PathBuf },
}

pub struct FileChange {
    pub path: PathBuf,
    pub change_type: FileChangeType,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub diff: Option<String>,
}

pub struct HistoryEntry {
    pub snapshot_id: SnapshotId,
    pub timestamp: DateTime<Utc>,
    pub operation: Operation,
    pub description: String,
    pub files_affected: Vec<PathBuf>,
}

pub struct HistoryState {
    pub current_index: usize,
    pub snapshots: Vec<EditSnapshot>,
    pub max_history: usize,
}
```

### Algoritmos

#### Captura de Snapshot

1. Listar todos os arquivos no workspace (respeitando .gitignore)
2. Para cada arquivo modificado desde último snapshot:
   - Ler conteúdo atual
   - Calcular hash
   - Armazenar estado
3. Criar snapshot com lista de estados
4. Salvar snapshot no histórico

#### Diff entre Snapshots

1. Comparar listas de arquivos
2. Identificar arquivos criados, modificados, deletados
3. Para arquivos modificados, calcular diff
4. Retornar lista de mudanças

#### Restauração de Snapshot

1. Carregar snapshot desejado
2. Calcular diff entre estado atual e snapshot
3. Aplicar mudanças reversas:
   - Criar arquivos deletados
   - Restaurar conteúdo de arquivos modificados
   - Deletar arquivos criados
   - Renomear arquivos se necessário
4. Validar restauração (checksums)

## Comandos CLI

### `jarvis undo [steps]`

Desfaz uma ou mais operações.

**Opções:**
- `steps`: Número de operações a desfazer (padrão: 1)
- `--dry-run`: Mostrar o que seria desfeito sem aplicar

**Exemplo:**
```bash
jarvis undo
jarvis undo 3
jarvis undo --dry-run
```

### `jarvis redo [steps]`

Refaz uma ou mais operações desfeitas.

**Opções:**
- `steps`: Número de operações a refazer (padrão: 1)
- `--dry-run`: Mostrar o que seria refeito sem aplicar

**Exemplo:**
```bash
jarvis redo
jarvis redo 2
```

### `jarvis history`

Mostra histórico de operações.

**Opções:**
- `--limit, -l <number>`: Limitar número de entradas (padrão: 20)
- `--format <format>`: Formato de saída (table, json)
- `--since <timestamp>`: Mostrar apenas desde timestamp

**Exemplo:**
```bash
jarvis history
jarvis history --limit 50
jarvis history --format json
jarvis history --since "2026-01-20T10:00:00Z"
```

### `jarvis history show <snapshot-id>`

Mostra detalhes de um snapshot específico.

**Exemplo:**
```bash
jarvis history show "550e8400-e29b-41d4-a716-446655440000"
```

### `jarvis history clear`

Limpa histórico antigo.

**Opções:**
- `--keep <number>`: Manter N snapshots recentes (padrão: 50)
- `--all`: Limpar todo o histórico

**Exemplo:**
```bash
jarvis history clear --keep 20
jarvis history clear --all
```

## Exemplos de Uso

### Exemplo 1: Desfazer Última Operação

```bash
# Jarvis fez mudanças indesejadas
$ jarvis "refactor this function"

# Ver o que mudou
$ jarvis history

# Desfazer
$ jarvis undo

# Verificar que mudanças foram revertidas
$ git status
```

### Exemplo 2: Navegar Histórico

```bash
# Ver histórico
$ jarvis history

# Output:
# 1. [2026-01-20 10:30] Modified src/main.rs
# 2. [2026-01-20 10:25] Created src/utils.rs
# 3. [2026-01-20 10:20] Modified src/lib.rs

# Desfazer 2 operações
$ jarvis undo 2

# Refazer 1 operação
$ jarvis redo
```

### Exemplo 3: Investigar Mudanças

```bash
# Ver detalhes de um snapshot específico
$ jarvis history show "550e8400-e29b-41d4-a716-446655440000"

# Output mostra:
# - Arquivos modificados
# - Diffs das mudanças
# - Metadados da operação
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `uuid` - IDs únicos de snapshots
- `serde` / `serde_json` - Serialização
- `tokio` - Async runtime
- `chrono` - Timestamps
- `sha2` - Cálculo de hashes
- `walkdir` - Navegação de diretórios
- `anyhow` / `thiserror` - Error handling
- `diff` ou `similar` - Cálculo de diffs

**Crates opcionais:**
- `git2` - Integração com Git
- `sqlx` - Persistência em banco de dados

### Desafios Técnicos

1. **Armazenamento de Snapshots**: Como armazenar eficientemente?
   - **Solução**: Usar storage incremental (apenas diffs)
   - Comprimir snapshots antigos
   - Limitar tamanho total do histórico

2. **Performance**: Snapshots podem ser lentos em projetos grandes
   - **Solução**: Snapshot apenas arquivos modificados
   - Usar hashing para detectar mudanças rapidamente
   - Snapshot assíncrono em background

3. **Conflitos**: O que fazer se arquivo foi modificado externamente?
   - **Solução**: Detectar conflitos antes de restaurar
   - Pedir confirmação do usuário
   - Oferecer merge manual

4. **Git Integration**: Como integrar com Git?
   - **Solução**: Criar commits Git para cada snapshot (opcional)
   - Usar Git como backup do histórico
   - Permitir sincronização bidirecional

5. **Limites de Histórico**: Quanto histórico manter?
   - **Solução**: Configurável pelo usuário
   - Limpeza automática de snapshots antigos
   - Manter pelo menos N snapshots recentes

### Performance

- **Lazy Snapshot**: Criar snapshot apenas quando necessário
- **Incremental Storage**: Armazenar apenas diferenças
- **Compression**: Comprimir snapshots antigos
- **Caching**: Cachear snapshots frequentes em memória

### Segurança

- **Validation**: Validar snapshots antes de restaurar
- **Backup**: Manter backup antes de operações destrutivas
- **Permissions**: Verificar permissões antes de modificar arquivos
- **Atomicity**: Operações devem ser atômicas (tudo ou nada)

## Roadmap de Implementação

### Fase 1: Core Infrastructure (Sprint 1)

- [ ] Definir estruturas de dados (`EditSnapshot`, `FileState`, etc.)
- [ ] Implementar `SnapshotManager` básico
- [ ] Sistema de captura de snapshots
- [ ] Comandos básicos (`undo`, `redo`)

### Fase 2: History Management (Sprint 2)

- [ ] Implementar `EditHistory`
- [ ] Persistência de histórico (SQLite)
- [ ] Comando `history` com visualização
- [ ] Navegação de histórico

### Fase 3: Advanced Features (Sprint 3)

- [ ] Diff entre snapshots
- [ ] Preview de undo/redo
- [ ] Integração com Git
- [ ] Limpeza automática de histórico

### Fase 4: Optimization (Sprint 4)

- [ ] Snapshot incremental
- [ ] Compressão de snapshots
- [ ] Performance improvements
- [ ] Cache de snapshots frequentes

### Fase 5: Integration (Sprint 5)

- [ ] Integração com sessões
- [ ] Integração com state runtime
- [ ] UI melhorada para histórico
- [ ] Export/import de histórico

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Agentic/History/EditHistory.cs` - Gerenciamento de histórico
- `Jarvis.CLI/Agentic/History/EditSnapshot.cs` - Estrutura de snapshots
- `Jarvis.CLI/Commands/UndoCommand.cs` - Comando undo
- `Jarvis.CLI/Commands/RedoCommand.cs` - Comando redo
- `Jarvis.CLI/Commands/HistoryCommand.cs` - Comando history

### Documentação Externa

- [Git Internals - Objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects)
- [Rust diff libraries](https://crates.io/crates/similar)
- [SQLite for Rust](https://docs.rs/rusqlite/latest/rusqlite/)
- [Jarvis CLI State Runtime](../state-runtime.md) - Sistema de persistência existente

---

**Status**: 📝 Planejado  
**Prioridade**: 🟡 Média  
**Última atualização**: 2026-01-20
