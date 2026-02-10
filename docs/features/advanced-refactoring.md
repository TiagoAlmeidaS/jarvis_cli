# Refatoração Avançada

## Visão Geral

O sistema de Refatoração Avançada permite que o Jarvis CLI execute refatorações complexas e seguras no código usando análise estática. O sistema suporta múltiplas linguagens e tipos de refatoração, com preview de mudanças e validação pós-refatoração.

O sistema inclui:
- **Análise Estática**: Análise de AST para refatorações seguras
- **Tipos de Refatoração**: Múltiplos tipos de refatoração suportados
- **Preview**: Visualizar mudanças antes de aplicar
- **Validação**: Validar refatorações com testes
- **Rollback**: Reverter refatorações se necessário

## Motivação

Problemas que o sistema resolve:

1. **Segurança**: Refatorações baseadas em AST são mais seguras que regex
2. **Precisão**: Entende estrutura do código, não apenas texto
3. **Confiança**: Preview permite revisar antes de aplicar
4. **Validação**: Testes garantem que nada quebrou
5. **Produtividade**: Automatiza refatorações tediosas

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│              Advanced Refactoring System                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Refactoring  │───▶│ AST          │───▶│ Refactoring  │ │
│  │ Engine       │    │ Parser       │    │ Applicator   │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Refactoring  │    │ Change       │    │ Validation   │ │
│  │ Strategies   │    │ Preview      │    │ Engine       │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Análise**:
   - Código é parseado em AST
   - Estrutura é analisada
   - Oportunidades de refatoração são identificadas

2. **Geração de Mudanças**:
   - Refatoração é aplicada ao AST
   - Mudanças são calculadas
   - Preview é gerado

3. **Revisão**:
   - Preview é apresentado ao usuário
   - Usuário pode aprovar ou rejeitar
   - Validações são executadas

4. **Aplicação**:
   - Mudanças são aplicadas aos arquivos
   - Testes são executados
   - Resultado é validado

### Integrações

- **rust-analyzer**: Para análise de código Rust
- **Tree-sitter**: Para parsing de múltiplas linguagens
- **Test Runner**: Validação pós-refatoração
- **Undo/Redo**: Rollback se necessário

## Especificação Técnica

### APIs e Interfaces

```rust
// Refactoring engine trait
pub trait RefactoringEngine: Send + Sync {
    async fn analyze(
        &self,
        file_path: &Path,
        refactoring_type: RefactoringType,
    ) -> Result<RefactoringAnalysis>;
    
    async fn preview(
        &self,
        analysis: &RefactoringAnalysis,
    ) -> Result<RefactoringPreview>;
    
    async fn apply(
        &self,
        analysis: &RefactoringAnalysis,
        options: ApplyOptions,
    ) -> Result<RefactoringResult>;
}

// AST parser trait
pub trait ASTParser: Send + Sync {
    fn parse(&self, content: &str, language: Language) -> Result<AST>;
    fn can_parse(&self, language: Language) -> bool;
}
```

### Estruturas de Dados

```rust
pub enum RefactoringType {
    ExtractFunction {
        start_line: usize,
        end_line: usize,
        new_function_name: String,
    },
    ExtractVariable {
        expression: String,
        variable_name: String,
    },
    RenameSymbol {
        old_name: String,
        new_name: String,
    },
    InlineFunction {
        function_name: String,
    },
    MoveFunction {
        function_name: String,
        target_file: PathBuf,
    },
    ChangeSignature {
        function_name: String,
        new_signature: FunctionSignature,
    },
    ConvertToAsync {
        function_name: String,
    },
    AddErrorHandling {
        function_name: String,
    },
}

pub struct RefactoringAnalysis {
    pub refactoring_type: RefactoringType,
    pub file_path: PathBuf,
    pub changes: Vec<CodeChange>,
    pub affected_symbols: Vec<Symbol>,
    pub risks: Vec<Risk>,
}

pub struct CodeChange {
    pub file_path: PathBuf,
    pub change_type: ChangeType,
    pub old_content: String,
    pub new_content: String,
    pub line_range: (usize, usize),
}

pub enum ChangeType {
    Insert,
    Replace,
    Delete,
}

pub struct RefactoringPreview {
    pub analysis: RefactoringAnalysis,
    pub diff: String,
    pub affected_files: Vec<PathBuf>,
    pub estimated_impact: ImpactEstimate,
}

pub struct RefactoringResult {
    pub success: bool,
    pub changes_applied: Vec<CodeChange>,
    pub tests_passed: bool,
    pub errors: Vec<String>,
}

pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
}
```

### Algoritmos

#### Análise de AST

1. Parsear código em AST
2. Identificar símbolos (funções, variáveis, tipos)
3. Analisar referências e dependências
4. Identificar oportunidades de refatoração

#### Aplicação de Refatoração

1. Modificar AST conforme refatoração
2. Calcular mudanças (diff)
3. Validar mudanças (sintaxe, tipos)
4. Aplicar mudanças aos arquivos

## Comandos CLI

### `jarvis refactor <type> <target>`

Executa uma refatoração.

**Tipos:**
- `extract-function`: Extrair função
- `extract-variable`: Extrair variável
- `rename`: Renomear símbolo
- `inline-function`: Inline função
- `move-function`: Mover função
- `change-signature`: Mudar assinatura
- `to-async`: Converter para async
- `add-error-handling`: Adicionar tratamento de erros

**Opções:**
- `--preview`: Apenas mostrar preview sem aplicar
- `--dry-run`: Validar sem aplicar
- `--skip-tests`: Pular testes após refatoração

**Exemplo:**
```bash
jarvis refactor extract-function src/main.rs:10:20 --name "process_data"
jarvis refactor rename src/utils.rs --old "old_name" --new "new_name"
jarvis refactor to-async src/api.rs --function "fetch_data" --preview
```

## Exemplos de Uso

### Exemplo 1: Extrair Função

```bash
$ jarvis refactor extract-function src/main.rs:10:25 --name "validate_input"

Preview:
- Extrair linhas 10-25 em nova função `validate_input`
- Adicionar chamada à função no lugar original
- Atualizar imports se necessário

Aplicar? (y/n): y
✓ Refatoração aplicada
✓ Testes passaram
```

### Exemplo 2: Renomear Símbolo

```bash
$ jarvis refactor rename src/auth.rs --old "login" --new "authenticate"

Análise:
- Encontradas 15 referências
- 3 arquivos afetados
- Nenhum risco detectado

Aplicar? (y/n): y
✓ Refatoração aplicada
✓ Todas referências atualizadas
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `syn` / `quote` - Para parsing Rust
- `tree-sitter` - Para parsing de outras linguagens
- `diff` ou `similar` - Para cálculo de diffs
- `serde` / `serde_json` - Serialização
- `anyhow` / `thiserror` - Error handling

**Integrações:**
- `rust-analyzer` - Para análise Rust (via LSP)
- Test Runner para validação
- Undo/Redo para rollback

### Desafios Técnicos

1. **Parsing**: Como parsear múltiplas linguagens?
   - **Solução**: Usar tree-sitter para linguagens suportadas
   - Usar syn para Rust (mais preciso)
   - Fallback para análise textual quando necessário

2. **Análise de Referências**: Como encontrar todas referências?
   - **Solução**: Usar LSP (Language Server Protocol)
   - Integrar com rust-analyzer para Rust
   - Análise estática para outras linguagens

3. **Validação**: Como garantir que refatoração é segura?
   - **Solução**: Validar sintaxe após mudanças
   - Executar testes
   - Verificar tipos (quando possível)

4. **Multi-file**: Como lidar com refatorações que afetam múltiplos arquivos?
   - **Solução**: Analisar projeto inteiro
   - Rastrear dependências entre arquivos
   - Aplicar mudanças atomicamente

### Performance

- **Incremental Parsing**: Re-parsear apenas arquivos modificados
- **Caching**: Cachear ASTs quando possível
- **Parallel Analysis**: Analisar múltiplos arquivos em paralelo

### Segurança

- **Preview**: Sempre mostrar preview antes de aplicar
- **Validation**: Validar mudanças antes de aplicar
- **Rollback**: Permitir reverter refatorações
- **Backup**: Criar backup antes de refatorações grandes

## Roadmap de Implementação

### Fase 1: Core Infrastructure (Sprint 1)

- [ ] Definir estruturas de dados (`RefactoringType`, `RefactoringAnalysis`)
- [ ] Implementar `ASTParser` básico (Rust via syn)
- [ ] Implementar `RefactoringEngine` básico
- [ ] Comando `jarvis refactor` básico

### Fase 2: Refactoring Types (Sprint 2)

- [ ] Implementar extract-function
- [ ] Implementar rename
- [ ] Implementar extract-variable
- [ ] Preview de mudanças

### Fase 3: Advanced Features (Sprint 3)

- [ ] Suporte a múltiplas linguagens (tree-sitter)
- [ ] Análise de referências (LSP)
- [ ] Validação pós-refatoração
- [ ] Integração com test runner

### Fase 4: Multi-file Refactoring (Sprint 4)

- [ ] Análise de projeto completo
- [ ] Refatorações multi-arquivo
- [ ] Aplicação atômica
- [ ] Rollback avançado

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Agentic/Refactoring/RefactoringEngine.cs` - Engine principal
- `Jarvis.CLI/Agentic/Refactoring/RoslynRefactorer.cs` - Refactorer usando Roslyn
- `Jarvis.CLI/Agentic/Refactoring/RegexRefactorer.cs` - Refactorer usando regex

### Documentação Externa

- [Syn (Rust AST)](https://docs.rs/syn/latest/syn/)
- [Tree-sitter](https://tree-sitter.github.io/tree-sitter/)
- [rust-analyzer](https://rust-analyzer.github.io/)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

---

**Status**: 📝 Planejado  
**Prioridade**: 🟢 Baixa  
**Última atualização**: 2026-01-20
