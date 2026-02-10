# Sistema de Testes Automático

## Visão Geral

O sistema de Testes Automático permite que o Jarvis CLI detecte, execute e analise testes automaticamente após mudanças no código. O sistema suporta múltiplos frameworks de teste e pode ser integrado com o processo de refatoração para validar mudanças.

O sistema inclui:
- **Detecção Automática**: Detecta framework de teste usado no projeto
- **Execução Automática**: Executa testes após mudanças
- **Análise de Resultados**: Analisa e reporta resultados
- **Integração com Refatoração**: Valida mudanças automaticamente

## Motivação

Problemas que o sistema resolve:

1. **Validação Automática**: Garante que mudanças não quebram testes existentes
2. **Confiança**: Permite refatoração com confiança
3. **Feedback Rápido**: Feedback imediato sobre qualidade do código
4. **CI/CD Integration**: Pode ser usado em pipelines
5. **Qualidade**: Mantém qualidade do código através de testes

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│              Auto Testing System                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Test         │───▶│ Test         │───▶│ Test         │ │
│  │ Framework    │    │ Runner       │    │ Analyzer     │ │
│  │ Detector     │    │              │    │              │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Framework    │    │ Test         │    │ Test         │ │
│  │ Adapters     │    │ Executor     │    │ Reporter     │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Detecção**:
   - Escanear projeto para detectar framework de teste
   - Verificar arquivos de configuração (Cargo.toml, package.json, pytest.ini, etc.)
   - Identificar estrutura de testes

2. **Execução**:
   - Executar comandos de teste apropriados
   - Capturar output e exit code
   - Monitorar tempo de execução

3. **Análise**:
   - Parsear resultados dos testes
   - Identificar testes que passaram/falharam
   - Calcular métricas (cobertura, tempo, etc.)

4. **Reporte**:
   - Gerar relatório de resultados
   - Destacar testes falhados
   - Sugerir correções quando apropriado

### Integrações

- **Refactoring Engine**: Valida mudanças após refatoração
- **File Watchers**: Executa testes quando arquivos mudam
- **CI/CD**: Pode ser usado em pipelines
- **LLM Gateway**: Pode usar resultados para melhorar código

## Especificação Técnica

### APIs e Interfaces

```rust
// Test framework detector trait
pub trait TestFrameworkDetector: Send + Sync {
    fn detect_framework(&self, project_path: &Path) -> Option<TestFramework>;
    fn detect_test_files(&self, project_path: &Path) -> Vec<PathBuf>;
}

// Test runner trait
pub trait TestRunner: Send + Sync {
    async fn run_tests(
        &self,
        framework: &TestFramework,
        options: TestRunOptions,
    ) -> Result<TestResults>;
    
    async fn run_specific_tests(
        &self,
        framework: &TestFramework,
        test_files: &[PathBuf],
    ) -> Result<TestResults>;
}

// Test analyzer trait
pub trait TestAnalyzer: Send + Sync {
    fn analyze_results(&self, results: &TestResults) -> TestAnalysis;
    fn suggest_fixes(&self, failures: &[TestFailure]) -> Vec<FixSuggestion>;
}
```

### Estruturas de Dados

```rust
pub enum TestFramework {
    RustCargo,
    PythonPytest,
    PythonUnittest,
    NodeJest,
    NodeMocha,
    GoTesting,
    JavaJUnit,
    DotNetXUnit,
    DotNetNUnit,
    Other(String),
}

pub struct TestFrameworkInfo {
    pub framework: TestFramework,
    pub command: Vec<String>,
    pub test_directory: PathBuf,
    pub config_file: Option<PathBuf>,
    pub detected_files: Vec<PathBuf>,
}

pub struct TestRunOptions {
    pub verbose: bool,
    pub parallel: bool,
    pub coverage: bool,
    pub filter: Option<String>,
    pub timeout: Option<Duration>,
}

pub struct TestResults {
    pub framework: TestFramework,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub duration: Duration,
    pub failures: Vec<TestFailure>,
    pub output: String,
    pub exit_code: i32,
}

pub struct TestFailure {
    pub test_name: String,
    pub file_path: Option<PathBuf>,
    pub line_number: Option<usize>,
    pub error_message: String,
    pub stack_trace: Option<String>,
}

pub struct TestAnalysis {
    pub success_rate: f32,
    pub critical_failures: Vec<TestFailure>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<FixSuggestion>,
}

pub struct FixSuggestion {
    pub test_name: String,
    pub suggestion: String,
    pub confidence: f32,
}
```

### Algoritmos

#### Detecção de Framework

1. Verificar arquivos de configuração:
   - `Cargo.toml` → Rust (cargo test)
   - `package.json` → Node.js (jest, mocha)
   - `pytest.ini` ou `setup.py` → Python (pytest)
   - `go.mod` → Go (go test)
   - `pom.xml` → Java (JUnit)
   - `.csproj` → .NET (xUnit, NUnit)

2. Verificar estrutura de diretórios:
   - `tests/` → Rust
   - `__tests__/` ou `*.test.js` → Jest
   - `test_*.py` → pytest

3. Verificar dependências:
   - `devDependencies` em package.json
   - `[dev-dependencies]` em Cargo.toml

#### Execução de Testes

1. Construir comando baseado no framework
2. Adicionar opções (verbose, parallel, etc.)
3. Executar em sandbox (se configurado)
4. Capturar stdout, stderr e exit code
5. Parsear resultados

#### Análise de Resultados

1. Identificar testes que falharam
2. Extrair mensagens de erro
3. Identificar padrões comuns de falha
4. Sugerir correções baseadas em erros

## Comandos CLI

### `jarvis test`

Executa testes automaticamente.

**Opções:**
- `--framework <name>`: Forçar framework específico
- `--verbose, -v`: Output verboso
- `--parallel`: Executar testes em paralelo
- `--coverage`: Gerar relatório de cobertura
- `--filter <pattern>`: Filtrar testes por padrão
- `--watch`: Observar mudanças e re-executar

**Exemplo:**
```bash
jarvis test
jarvis test --verbose
jarvis test --coverage
jarvis test --filter "auth"
jarvis test --watch
```

### `jarvis test detect`

Detecta framework de teste no projeto.

**Exemplo:**
```bash
jarvis test detect
```

### `jarvis test analyze <results-file>`

Analisa resultados de testes de um arquivo.

**Exemplo:**
```bash
jarvis test analyze test-results.json
```

## Exemplos de Uso

### Exemplo 1: Execução Automática Após Refatoração

```bash
# Jarvis refatora código
$ jarvis refactor "extract function"

# Sistema automaticamente executa testes
$ jarvis test

# Output:
# Running cargo test...
# test auth::test_login ... ok
# test auth::test_logout ... ok
# test utils::test_format ... FAILED
# 
# 2 passed, 1 failed
```

### Exemplo 2: Detecção de Framework

```bash
$ jarvis test detect

Detected test framework: Rust (cargo test)
Test directory: ./tests
Config file: Cargo.toml
Found 15 test files
```

### Exemplo 3: Watch Mode

```bash
$ jarvis test --watch

Watching for changes...
File changed: src/auth.rs
Running tests...
✓ All tests passed
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialização
- `anyhow` / `thiserror` - Error handling
- `walkdir` - Navegação de diretórios
- `glob` - Padrões de arquivos
- `toml` - Parsear Cargo.toml
- `notify` - File watching (para watch mode)

**Crates opcionais:**
- `regex` - Para parsing de output de testes
- `which` - Para encontrar executáveis

### Desafios Técnicos

1. **Parsing de Output**: Cada framework tem formato diferente
   - **Solução**: Criar parsers específicos por framework
   - Usar regex ou parsing estruturado
   - Suportar múltiplos formatos de output

2. **Detecção Confiável**: Como detectar framework corretamente?
   - **Solução**: Usar heurísticas múltiplas
   - Verificar arquivos de config + estrutura + dependências
   - Permitir override manual

3. **Performance**: Executar testes pode ser lento
   - **Solução**: Executar em paralelo quando possível
   - Cachear resultados quando apropriado
   - Permitir executar apenas testes afetados

4. **Sandboxing**: Testes podem ter efeitos colaterais
   - **Solução**: Usar sandboxing existente do Jarvis CLI
   - Executar testes em ambiente isolado
   - Limitar recursos (CPU, memória, tempo)

5. **Multi-framework**: Projetos podem ter múltiplos frameworks
   - **Solução**: Detectar todos e executar sequencialmente
   - Permitir escolher qual executar
   - Agregar resultados

### Performance

- **Parallel Execution**: Executar testes em paralelo quando suportado
- **Incremental Testing**: Executar apenas testes afetados por mudanças
- **Caching**: Cachear resultados quando código não mudou
- **Timeout**: Limitar tempo de execução de testes

### Segurança

- **Sandboxing**: Executar testes em ambiente isolado
- **Resource Limits**: Limitar CPU, memória, tempo
- **Network**: Bloquear acesso à rede (exceto quando necessário)
- **File System**: Limitar acesso a sistema de arquivos

## Roadmap de Implementação

### Fase 1: Core Infrastructure (Sprint 1)

- [ ] Definir estruturas de dados (`TestFramework`, `TestResults`)
- [ ] Implementar `TestFrameworkDetector` básico
- [ ] Suportar Rust (cargo test)
- [ ] Comando `jarvis test detect`

### Fase 2: Test Execution (Sprint 2)

- [ ] Implementar `TestRunner`
- [ ] Adicionar suporte a Python (pytest)
- [ ] Adicionar suporte a Node.js (jest)
- [ ] Comando `jarvis test`

### Fase 3: Analysis & Reporting (Sprint 3)

- [ ] Implementar `TestAnalyzer`
- [ ] Parsing de resultados
- [ ] Geração de relatórios
- [ ] Sugestões de correção

### Fase 4: Integration (Sprint 4)

- [ ] Integração com refactoring engine
- [ ] Watch mode
- [ ] Coverage reporting
- [ ] CI/CD integration

### Fase 5: Advanced Features (Sprint 5)

- [ ] Suporte a mais frameworks
- [ ] Incremental testing
- [ ] Test generation
- [ ] Performance profiling

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Agentic/Testing/AutoTestRunner.cs` - Executor de testes
- `Jarvis.CLI/Agentic/Testing/TestFrameworkDetector.cs` - Detector de frameworks
- `Jarvis.CLI/Agentic/Testing/TestModels.cs` - Modelos de dados

### Documentação Externa

- [Rust Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Pytest Documentation](https://docs.pytest.org/)
- [Jest Documentation](https://jestjs.io/)
- [Jarvis CLI Sandboxing](../sandbox.md) - Sandboxing para execução segura

---

**Status**: 📝 Planejado  
**Prioridade**: 🟡 Média  
**Última atualização**: 2026-01-20
