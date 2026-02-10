# Sistema de Skills

## Visão Geral

O Sistema de Skills permite que usuários criem, compartilhem e reutilizem funcionalidades automatizadas como "skills" (habilidades). Skills são scripts ou módulos que encapsulam conhecimento e ações específicas, permitindo que o Jarvis CLI execute tarefas complexas de forma reutilizável.

Skills podem ser:
- **Publicados** no marketplace para compartilhamento
- **Instalados** localmente via JIT (Just-In-Time) loading
- **Executados** pelo LLM como tools durante conversas
- **Validados** e testados antes da publicação
- **Cacheados** localmente para performance

## Motivação

O sistema de skills resolve vários problemas:

1. **Reutilização**: Evita reimplementar soluções comuns repetidamente
2. **Compartilhamento**: Permite que a comunidade compartilhe conhecimento útil
3. **Extensibilidade**: Permite adicionar novas capacidades sem modificar o core
4. **Descoberta**: Facilita encontrar soluções para problemas comuns
5. **Qualidade**: Sistema de validação e testes garante qualidade

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│                    Skills System                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Skill        │    │ Skill        │    │ JIT Skill    │ │
│  │ Registry     │───▶│ Executor     │───▶│ Loader       │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Skill        │    │ Skill        │    │ Skill        │ │
│  │ Validator    │    │ Cache        │    │ Marketplace  │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Busca de Skills**: Usuário busca skills no marketplace via `jarvis skills search`
2. **Instalação JIT**: Skill é baixado e cacheado automaticamente quando necessário
3. **Execução**: LLM chama skill via `execute_skill` tool durante conversa
4. **Validação**: Skills podem ser validados antes da publicação
5. **Cache**: Skills instalados são cacheados localmente em `~/.jarvis/skills/`

### Integrações

- **LLM Gateway**: Skills são expostos como tools para o LLM
- **API Client**: Comunicação com marketplace/backend
- **File System**: Cache local de skills
- **Test Framework**: Validação e testes de skills

## Especificação Técnica

### APIs e Interfaces

```rust
// Skill metadata
pub struct SkillMetadata {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub version: String,
    pub language: SkillLanguage,
    pub category: String,
    pub tags: Vec<String>,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Skill definition
pub struct Skill {
    pub metadata: SkillMetadata,
    pub code: String,
    pub dependencies: Vec<String>,
    pub parameters: Vec<SkillParameter>,
    pub tests: Vec<SkillTest>,
}

// Skill executor trait
pub trait SkillExecutor: Send + Sync {
    async fn execute(
        &self,
        skill: &Skill,
        parameters: HashMap<String, Value>,
    ) -> Result<SkillExecutionResult>;
}

// JIT Skill loader trait
pub trait JITSkillLoader: Send + Sync {
    async fn search_skills(
        &self,
        query: &str,
    ) -> Result<Vec<SkillSearchResult>>;
    
    async fn download_skill(
        &self,
        skill_id: Uuid,
    ) -> Result<Skill>;
    
    async fn get_cached_skill(
        &self,
        skill_id: Uuid,
    ) -> Result<Option<Skill>>;
    
    async fn clear_cache(
        &self,
        skill_id: Option<Uuid>,
    ) -> Result<()>;
}

// Skill registry trait
pub trait SkillRegistry: Send + Sync {
    fn register_skill(&mut self, skill: Skill) -> Result<()>;
    fn get_skill(&self, name: &str) -> Option<&Skill>;
    fn list_skills(&self) -> Vec<&Skill>;
    fn search_skills(&self, query: &str) -> Vec<&Skill>;
}
```

### Estruturas de Dados

```rust
pub enum SkillLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Shell,
    Other(String),
}

pub struct SkillParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub description: String,
    pub required: bool,
    pub default_value: Option<Value>,
}

pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

pub struct SkillTest {
    pub name: String,
    pub test_type: TestType,
    pub input: HashMap<String, Value>,
    pub expected_output: Value,
}

pub enum TestType {
    Unit,
    Integration,
    E2E,
}

pub struct SkillExecutionResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}
```

### Algoritmos

#### Busca de Skills
1. Buscar no cache local primeiro
2. Se não encontrado, buscar no marketplace
3. Filtrar por query, categoria, tags
4. Ordenar por relevância (rating, downloads, recency)
5. Retornar top N resultados

#### JIT Loading
1. Verificar se skill está em cache
2. Se não, baixar do marketplace
3. Validar skill (checksum, formato)
4. Salvar em cache local
5. Retornar skill

#### Execução de Skill
1. Validar parâmetros fornecidos
2. Resolver dependências
3. Criar ambiente isolado (sandbox)
4. Executar código do skill
5. Capturar output/erros
6. Retornar resultado

## Comandos CLI

### `jarvis skills list`

Lista todas as skills publicadas no marketplace.

**Opções:**
- `--category, -c <category>`: Filtrar por categoria
- `--limit, -l <number>`: Limitar número de resultados (padrão: 20)

**Exemplo:**
```bash
jarvis skills list
jarvis skills list --category "code-analysis" --limit 10
```

### `jarvis skills show <name>`

Mostra detalhes de uma skill específica.

**Opções:**
- `--version, -v <version>`: Versão específica a mostrar

**Exemplo:**
```bash
jarvis skills show "refactor-to-async"
jarvis skills show "refactor-to-async" --version "1.2.0"
```

### `jarvis skills search <query>`

Busca skills no marketplace.

**Opções:**
- `--category, -c <category>`: Filtrar por categoria
- `--language, -l <language>`: Filtrar por linguagem
- `--limit, -n <number>`: Limitar resultados

**Exemplo:**
```bash
jarvis skills search "refactoring"
jarvis skills search "test" --language rust --limit 5
```

### `jarvis skills install <skill-id>`

Instala uma skill do marketplace (JIT loading).

**Exemplo:**
```bash
jarvis skills install "550e8400-e29b-41d4-a716-446655440000"
```

### `jarvis skills cache`

Gerencia o cache de skills.

**Subcomandos:**
- `list`: Lista skills cacheadas
- `clear [skill-id]`: Limpa cache (tudo ou skill específica)

**Exemplo:**
```bash
jarvis skills cache list
jarvis skills cache clear
jarvis skills cache clear "550e8400-e29b-41d4-a716-446655440000"
```

### `jarvis skills validate <path>`

Valida uma skill local antes da publicação.

**Exemplo:**
```bash
jarvis skills validate ./my-skill.json
```

### `jarvis skills test <skill-name>`

Executa testes de uma skill.

**Exemplo:**
```bash
jarvis skills test "my-skill"
```

### `jarvis skills publish <path>`

Publica uma skill no marketplace.

**Exemplo:**
```bash
jarvis skills publish ./my-skill.json
```

## Exemplos de Uso

### Exemplo 1: Buscar e Instalar Skill

```bash
# Buscar skills de refatoração
$ jarvis skills search "refactor"

# Instalar skill específica
$ jarvis skills install "refactor-to-async"

# Skill é automaticamente cacheada e disponível para uso
```

### Exemplo 2: Usar Skill em Conversa

Durante uma conversa com o Jarvis, o LLM pode automaticamente usar skills:

```
User: "Refatore essa função para usar async/await"

Jarvis: [Usa skill "refactor-to-async" automaticamente]
        ✓ Função refatorada com sucesso
```

### Exemplo 3: Criar e Publicar Skill

```bash
# Criar skill localmente
$ cat > my-skill.json << EOF
{
  "name": "add-error-handling",
  "description": "Adiciona tratamento de erros a funções Rust",
  "language": "rust",
  "code": "...",
  "parameters": [...],
  "tests": [...]
}
EOF

# Validar skill
$ jarvis skills validate my-skill.json

# Testar skill
$ jarvis skills test my-skill.json

# Publicar skill
$ jarvis skills publish my-skill.json
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `uuid` - Para IDs únicos de skills
- `serde` / `serde_json` - Serialização de skills
- `tokio` - Async runtime
- `reqwest` - Cliente HTTP para marketplace
- `anyhow` / `thiserror` - Error handling
- `async-trait` - Traits assíncronas
- `chrono` - Timestamps
- `walkdir` - Navegação de diretórios (para cache)

**Dependências opcionais:**
- `sandbox` - Para execução isolada de skills
- `validator` - Para validação de parâmetros

### Desafios Técnicos

1. **Execução Segura**: Skills podem executar código arbitrário
   - **Solução**: Usar sandboxing existente do Jarvis CLI
   - Executar skills em ambiente isolado
   - Limitar acesso a sistema de arquivos e rede

2. **Gerenciamento de Dependências**: Skills podem ter dependências
   - **Solução**: Resolver dependências antes da execução
   - Usar gerenciadores de pacotes apropriados (cargo, pip, npm)

3. **Cache Invalidation**: Quando atualizar cache?
   - **Solução**: Verificar versão no marketplace periodicamente
   - Permitir atualização manual via `jarvis skills cache clear`

4. **Multi-language Support**: Suportar múltiplas linguagens
   - **Solução**: Criar abstração de executor por linguagem
   - Usar interpretadores/runtimes apropriados

### Performance

- **Cache Local**: Reduz latência de execução
- **Lazy Loading**: Carregar skills apenas quando necessário
- **Parallel Execution**: Executar múltiplas skills em paralelo quando possível
- **Indexação**: Manter índice de skills para busca rápida

### Segurança

- **Validação**: Validar formato e conteúdo de skills antes da execução
- **Sandboxing**: Executar skills em ambiente isolado
- **Checksums**: Verificar integridade de skills baixadas
- **Permissions**: Skills devem declarar permissões necessárias
- **Code Review**: Revisar skills antes de publicar no marketplace

## Roadmap de Implementação

### Fase 1: Core Infrastructure (Sprint 1)

- [ ] Definir estruturas de dados (`Skill`, `SkillMetadata`)
- [ ] Implementar `SkillRegistry` básico
- [ ] Criar sistema de cache local (`~/.jarvis/skills/`)
- [ ] Implementar comandos básicos (`list`, `show`, `search`)

### Fase 2: JIT Loading (Sprint 2)

- [ ] Implementar `JITSkillLoader`
- [ ] Integrar com API/marketplace
- [ ] Implementar download e cache de skills
- [ ] Adicionar comandos `install` e `cache`

### Fase 3: Execution Engine (Sprint 3)

- [ ] Implementar `SkillExecutor`
- [ ] Integrar com sandboxing existente
- [ ] Suportar múltiplas linguagens (Rust, Python, Shell)
- [ ] Expor skills como tools para LLM

### Fase 4: Validation & Testing (Sprint 4)

- [ ] Implementar `SkillValidator`
- [ ] Sistema de testes de skills
- [ ] Comandos `validate` e `test`
- [ ] Integração com CI/CD

### Fase 5: Marketplace Integration (Sprint 5)

- [ ] Implementar publicação de skills
- [ ] Sistema de rating e reviews
- [ ] Versionamento de skills
- [ ] Analytics de uso

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Commands/SkillsCommand.cs` - Comandos CLI de skills
- `Jarvis.CLI/Skills/JITSkillLoader.cs` - Implementação JIT loading
- `Jarvis.CLI/Tools/SkillExecuteTool.cs` - Tool de execução de skills
- `Jarvis.CLI/Commands/SkillsJITCommands.cs` - Comandos JIT específicos

### Documentação Externa

- [Rust async traits](https://rust-lang.github.io/async-book/07_workarounds/05_async_in_traits.html)
- [Sandboxing in Rust](https://docs.rs/sandbox/latest/sandbox/)
- [Jarvis CLI Sandboxing](../sandbox.md) - Documentação de sandboxing existente

---

**Status**: 📝 Planejado  
**Prioridade**: 🔴 Alta  
**Última atualização**: 2026-01-20
