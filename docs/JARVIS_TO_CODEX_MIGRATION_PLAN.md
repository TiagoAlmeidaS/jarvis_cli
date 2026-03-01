# Plano de Migração: Integração Jarvis no Codex

## Análise do Estado Atual

### Estrutura Atual do Projeto

```
jarvis_cli/
├── codex-rs/           # Código upstream (54 crates) - atualizado
├── jarvis-rs/          # Suas customizações (55 crates) - renomeadas
├── jarvis-cli/         # CLI específica do Jarvis
├── codex-cli/         # CLI do Codex
├── docs/              # Documentação
└── ...
```

### Crates Únicas do Jarvis (não existem no upstream)

Estas são suas funcionalidades customizadas que **precisam ser preservadas**:

| Crate                            | Descrição                               | Prioridade |
| -------------------------------- | --------------------------------------- | ---------- |
| `daemon`                         | PublisherDaemon - pipelines de conteúdo | **ALTA**   |
| `daemon-common`                  | DB e modelos do daemon                  | **ALTA**   |
| `messaging`                      | Integrações de mensageria               | **ALTA**   |
| `telegram`                       | Bot Telegram                            | **ALTA**   |
| `whatsapp`                       | Bot WhatsApp                            | **ALTA**   |
| `web-api`                        | API web do Jarvis                       | **ALTA**   |
| `jarvis-api`                     | API Jarvis específica                   | **ALTA**   |
| `jarvis-client`                  | Cliente Jarvis                          | **ALTA**   |
| `jarvis-backend-openapi-models`  | Modelos OpenAPI                         | **MÉDIA**  |
| `jarvis-experimental-api-macros` | Macros experimentais                    | **BAIXA**  |
| `github`                         | Integração GitHub                       | **MÉDIA**  |
| `common`                         | Utilitários comuns                      | **ALTA**   |

### Crates Duplicadas (existem em ambos)

Cerca de 40 crates existem em ambos os diretórios mas com nomes diferentes:

- `codex-core` vs `jarvis-core`
- `codex-cli` vs `jarvis-cli`
- etc.

---

## Estratégia de Migração Proposta

### Princípios Fundamentais

1. ** NÃO sobrescrever código upstream** - Manter codex-rs/ intacto
2. **Adicionar sem renomear** - Criar novas crates que estendem o Codex
3. **Dependência, não duplicação** - Suas crates dependem do codex-core, não copiam
4. **Updates automáticos** - Facilitar sync com upstream

### Estrutura Proposta

```
codex-rs/                           # Upstream (não modificar)
├── core/                           # codex-core (upstream)
├── cli/                            # codex-cli (upstream)
└── ...

codex-rs-jarvis/                    # NOVO: Suas customizações
├── Cargo.toml                      # Workspace separado
├── jarvis-daemon/                  # Seu daemon
├── jarvis-messaging/               # Sistema de mensagens
├── jarvis-telegram/                # Integração Telegram
├── jarvis-whatsapp/                # Integração WhatsApp
├── jarvis-web-api/                 # API web
├── jarvis-common/                  # Utilitários comuns
└── ...
```

### Por que workspace separado?

1. **Updates automáticos** - Apenas codex-rs/ precisa ser atualizado
2. **Sem conflitos de merge** - Seu código nunca sobrescreve upstream
3. **Dependência clara** - jarvis-xxx depende de codex-core
4. **Fácil manutenção** - Separar concerns

---

## Plano de Execução

### Fase 1: Análise e Mapeamento

- [ ] Mapear todas as dependências de jarvis-core
- [ ] Identificar quais features são customizadas vs upstream
- [ ] Documentar APIs públicas usadas

### Fase 2: Criar Workspace Jarvis

- [ ] Criar diretório `codex-rs-jarvis/`
- [ ] Criar Cargo.toml workspace
- [ ] Criar crates vazias base

### Fase 3: Migrar Crates Únicas

Para cada crate única (daemon, messaging, telegram, etc.):

1. Criar nova crate em `codex-rs-jarvis/`
2. Adicionar dependência para `codex-core` (não copiar código)
3. Implementar suas features sobre o codex-core
4. Mover código específico para a nova crate

### Fase 4: Atualizar Dependências

- [ ] Alterar `jarvis-core` → `codex-core` em todas as crates
- [ ] Atualizar paths de importação
- [ ] Testar compilação

### Fase 5: Cleanup

- [ ] Remover jarvis-rs/ duplicado
- [ ] Atualizar README
- [ ] Atualizar CI/CD
- [ ] Atualizar documentação

---

## Mapeamento de Dependências

### Dependencies que precisam ser criadas

```toml
# codex-rs-jarvis/Cargo.toml

[workspace]
members = [
    "jarvis-daemon",
    "jarvis-messaging",
    "jarvis-telegram",
    "jarvis-whatsapp",
    "jarvis-web-api",
    "jarvis-common",
    # ... outras
]

[workspace.dependencies]
# Herdar todas as dependências do codex-rs
codex-core = { path = "../codex-rs/core" }
codex-cli = { path = "../codex-rs/cli" }
# ... outras
```

### Exemplo de crate migrada

```toml
# codex-rs-jarvis/jarvis-daemon/Cargo.toml
[package]
name = "jarvis-daemon"
version.workspace = true

[dependencies]
codex-core = { workspace = true }
# Suas dependências específicas
tokio = { workspace = true }
# ...
```

---

## Cronograma Sugerido

| Fase | Duração | Atividade               |
| ---- | ------- | ----------------------- |
| 1    | 1 dia   | Análise e mapeamento    |
| 2    | 1 dia   | Criar workspace         |
| 3    | 3 dias  | Migrar crates uma a uma |
| 4    | 1 dia   | Atualizar dependências  |
| 5    | 1 dia   | Cleanup e docs          |

**Total estimado: ~7 dias**

---

## Riscos e Mitigações

| Risco                 | Mitigação                                 |
| --------------------- | ----------------------------------------- |
| Perder funcionalidade | Testar cada crate após migração           |
| Quebrar dependências  | Manter jarvis-rs/ como backup até validar |
| Conflitos futuros     | Usar workspace separado                   |

---

## Próximos Passos Imediatos

1. **Confirmar estratégia** - Você approves esta abordagem?
2. **Mapear dependências** - Analisar jarvis-core dependencies
3. **Criar workspace base** - Setup inicial do codex-rs-jarvis/

---

_Documento criado em: Março 2026_
