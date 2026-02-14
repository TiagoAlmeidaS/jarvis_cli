# Sandbox Execution — Execucao Segura de Comandos

**Status**: Planejado
**Prioridade**: ALTA
**Gap**: G6
**Roadmap**: Fase 2, Step 2.3
**Ultima atualizacao**: 2026-02-13

---

## 1. Problema

O Jarvis pode executar comandos shell arbitrarios. Hoje o controle e binario:
- **Com aprovacao**: cada comando pede permissao (lento)
- **Yolo mode**: zero restricoes (perigoso)

Falta uma camada intermediaria que permita **execucao autonoma com guardrails**:
limites de recursos, restricao de diretorio, rollback de mudancas, e audit trail.

### Referencia: Claude Code / Cursor

Esses tools usam um modelo de **sandbox progressivo**:
1. Comandos read-only (ls, cat, grep) — nunca pedem permissao
2. Comandos de workspace (git add, npm install) — permitidos no diretorio
3. Comandos de sistema (rm -rf, sudo) — sempre pedem permissao
4. Network access — controlado por policy

---

## 2. Modelo de Sandbox Proposto

### 2.1 Niveis de Sandbox

```rust
pub enum SandboxLevel {
    /// Read-only: apenas leitura de arquivos e listagem.
    /// Nenhuma escrita permitida.
    ReadOnly,

    /// Workspace write: leitura livre, escrita apenas em diretórios permitidos.
    /// Acesso a rede bloqueado.
    WorkspaceWrite {
        writable_roots: Vec<PathBuf>,
    },

    /// Full workspace: escrita em workspace + acesso a rede controlado.
    /// Comandos de sistema restritos.
    FullWorkspace {
        writable_roots: Vec<PathBuf>,
        allow_network: bool,
        blocked_commands: Vec<String>,
    },

    /// Full access (yolo): sem restricoes.
    /// Usar apenas com aprovacao explicita do usuario.
    DangerFullAccess,
}
```

### 2.2 Command Classification

```rust
pub enum CommandRisk {
    /// Leitura pura: ls, cat, head, tail, find, grep, git status, git log
    Safe,

    /// Escrita em workspace: git add, git commit, npm install, cargo build
    WorkspaceWrite,

    /// Escrita potencialmente destrutiva: rm, mv com overwrite, git reset
    Destructive,

    /// Sistema: sudo, chmod, chown, systemctl, apt, pip install (global)
    System,

    /// Network: curl, wget, ssh, nc, docker
    Network,

    /// Desconhecido: qualquer comando nao classificado
    Unknown,
}

impl CommandRisk {
    /// Classify a command based on its first token and arguments.
    pub fn classify(command: &[String]) -> Self {
        let cmd = command.first().map(String::as_str).unwrap_or("");
        let args = &command[1..];

        match cmd {
            // Safe (read-only)
            "ls" | "dir" | "cat" | "head" | "tail" | "find" | "rg" | "grep"
            | "wc" | "file" | "stat" | "du" | "df" | "echo" | "pwd"
            | "which" | "where" | "type" | "env" | "printenv" => Self::Safe,

            // Git: depends on subcommand
            "git" => match args.first().map(String::as_str) {
                Some("status") | Some("log") | Some("diff") | Some("show")
                | Some("branch") | Some("remote") | Some("tag") => Self::Safe,
                Some("add") | Some("commit") | Some("checkout") | Some("switch")
                | Some("merge") | Some("rebase") | Some("stash") => Self::WorkspaceWrite,
                Some("push") | Some("pull") | Some("fetch") | Some("clone") => Self::Network,
                Some("reset") if args.iter().any(|a| a == "--hard") => Self::Destructive,
                _ => Self::WorkspaceWrite,
            },

            // Package managers (workspace-level)
            "cargo" | "npm" | "npx" | "yarn" | "pnpm" | "dotnet"
            | "pip" | "poetry" | "uv" => Self::WorkspaceWrite,

            // Destructive
            "rm" | "del" | "rmdir" => Self::Destructive,
            "mv" | "move" if args.iter().any(|a| a == "-f") => Self::Destructive,

            // System
            "sudo" | "chmod" | "chown" | "chgrp" | "systemctl"
            | "apt" | "apt-get" | "yum" | "dnf" | "brew"
            | "snap" | "flatpak" => Self::System,

            // Network
            "curl" | "wget" | "ssh" | "scp" | "rsync" | "nc"
            | "docker" | "kubectl" | "terraform" => Self::Network,

            // Shell interpreters: analyze the -c argument
            "bash" | "sh" | "zsh" | "powershell" | "pwsh" | "cmd" => {
                // If using -c, classify the inner command
                if let Some(pos) = args.iter().position(|a| a == "-c" || a == "/c") {
                    if let Some(inner) = args.get(pos + 1) {
                        let inner_parts: Vec<String> = inner
                            .split_whitespace()
                            .map(String::from)
                            .collect();
                        return Self::classify(&inner_parts);
                    }
                }
                Self::Unknown
            },

            _ => Self::Unknown,
        }
    }
}
```

### 2.3 Decisao por nivel + risco

| Command Risk | ReadOnly | WorkspaceWrite | FullWorkspace | DangerFullAccess |
|-------------|----------|----------------|---------------|------------------|
| Safe | Executar | Executar | Executar | Executar |
| WorkspaceWrite | BLOQUEAR | Executar (se no workspace) | Executar | Executar |
| Destructive | BLOQUEAR | PEDIR APROVACAO | PEDIR APROVACAO | Executar |
| System | BLOQUEAR | BLOQUEAR | PEDIR APROVACAO | Executar |
| Network | BLOQUEAR | BLOQUEAR | Se allow_network | Executar |
| Unknown | BLOQUEAR | PEDIR APROVACAO | PEDIR APROVACAO | Executar |

---

## 3. Rollback de Mudancas

### 3.1 Snapshot antes de acoes destrutivas

Antes de executar uma acao classificada como `Destructive` ou `WorkspaceWrite`,
criar um snapshot do estado afetado:

```rust
pub struct ActionSnapshot {
    pub id: String,
    pub timestamp: i64,
    pub action: String,
    pub affected_files: Vec<FileSnapshot>,
}

pub struct FileSnapshot {
    pub path: PathBuf,
    pub content_before: Option<Vec<u8>>,  // None = file didn't exist
    pub permissions: Option<u32>,
}

impl ActionSnapshot {
    /// Rollback all changes from this snapshot.
    pub async fn rollback(&self) -> Result<RollbackSummary> {
        for file in &self.affected_files {
            match &file.content_before {
                Some(content) => {
                    // Restore original content
                    tokio::fs::write(&file.path, content).await?;
                }
                None => {
                    // File was created by the action, delete it
                    if file.path.exists() {
                        tokio::fs::remove_file(&file.path).await?;
                    }
                }
            }
        }
        Ok(RollbackSummary { files_restored: self.affected_files.len() })
    }
}
```

### 3.2 Git-based rollback (alternativa simples)

Para projetos com git, usar `git stash` como mecanismo de rollback:

```rust
pub async fn safe_execute_with_git_rollback(
    command: &[String],
    cwd: &Path,
) -> Result<CommandOutput> {
    // 1. Check if cwd is a git repo with clean state
    let status = git_status(cwd).await?;

    // 2. Stash current state (safety net)
    if !status.is_clean {
        git_stash_push(cwd, "jarvis-safety-net").await?;
    }

    // 3. Execute command
    let result = execute_command(command, cwd).await;

    // 4. If failed, rollback
    if result.is_err() || result.as_ref().unwrap().exit_code != 0 {
        git_checkout_dot(cwd).await?;  // Discard changes
        if !status.is_clean {
            git_stash_pop(cwd).await?;  // Restore original state
        }
    }

    result
}
```

---

## 4. Resource Limits

### 4.1 Limites por execucao

```rust
pub struct ExecutionLimits {
    /// Maximum execution time per command.
    pub timeout: Duration,              // default: 30s
    /// Maximum output size (stdout + stderr).
    pub max_output_bytes: usize,        // default: 1MB
    /// Maximum CPU time (on supported platforms).
    pub max_cpu_time: Option<Duration>, // default: None
    /// Maximum memory (on supported platforms).
    pub max_memory_bytes: Option<usize>, // default: None
    /// Working directory restriction.
    pub allowed_cwd: Option<PathBuf>,
}
```

### 4.2 Implementacao cross-platform

| Mecanismo | Linux | macOS | Windows |
|-----------|-------|-------|---------|
| Timeout | `tokio::time::timeout` | `tokio::time::timeout` | `tokio::time::timeout` |
| Output limit | Truncate reader | Truncate reader | Truncate reader |
| CWD restriction | Path validation | Path validation | Path validation |
| Process isolation | `unshare` / `cgroups` | `sandbox-exec` (Seatbelt) | Job objects |

Para simplificar, fase 1 usa apenas:
- `tokio::time::timeout` (cross-platform)
- Output truncation (cross-platform)
- Path validation (cross-platform)

Isolamento de processo real (cgroups, Seatbelt, job objects) e fase 3.

---

## 5. Audit Trail

### 5.1 Log de todas as execucoes

Cada comando executado e registrado com contexto completo:

```rust
pub struct ExecutionLog {
    pub id: String,
    pub timestamp: i64,
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub risk_level: CommandRisk,
    pub sandbox_level: SandboxLevel,
    pub approval: ApprovalDecision,  // Auto, UserApproved, Denied
    pub exit_code: Option<i32>,
    pub stdout_preview: String,      // primeiros 500 chars
    pub stderr_preview: String,
    pub duration_ms: u64,
    pub snapshot_id: Option<String>, // referencia ao ActionSnapshot
}
```

### 5.2 Persistencia

Usar a tabela `daemon_logs` existente para o daemon, e o session rollout
(JSONL) existente para o TUI.

Para auditoria mais detalhada, nova tabela (opcional):

```sql
CREATE TABLE IF NOT EXISTS execution_audit (
    id              TEXT PRIMARY KEY,
    session_id      TEXT,               -- sessao TUI ou job ID daemon
    command         TEXT NOT NULL,       -- comando executado
    cwd             TEXT NOT NULL,
    risk_level      TEXT NOT NULL,
    sandbox_level   TEXT NOT NULL,
    approval        TEXT NOT NULL,       -- auto, user_approved, denied
    exit_code       INTEGER,
    stdout_preview  TEXT,
    stderr_preview  TEXT,
    duration_ms     INTEGER,
    snapshot_id     TEXT,
    created_at      INTEGER NOT NULL
);
```

---

## 6. CLI de auditoria

```
jarvis audit list [--last 24h] [--risk high]     # Ultimas execucoes
jarvis audit show <id>                             # Detalhes de uma execucao
jarvis audit rollback <snapshot-id>                # Reverter um snapshot
jarvis audit stats                                 # Estatisticas (comandos/dia, risco)
```

---

## 7. Integracao com AgentLoop e ToolDispatcher

O sandbox e **transparente** para o AgentLoop:

```rust
// No ToolDispatcher:
pub async fn dispatch(&self, tool_name: &str, args: &str) -> Result<ToolResult> {
    let tool = self.registry.get(tool_name)?;
    let ctx = ToolContext {
        cwd: self.cwd.clone(),
        sandbox_policy: self.sandbox_policy.clone(),
        // ...
    };

    // Classify risk.
    let risk = if tool_name == "shell" {
        let command = parse_shell_args(args)?;
        CommandRisk::classify(&command)
    } else {
        tool.risk_level().into()
    };

    // Check sandbox policy.
    match self.sandbox.check_permission(risk) {
        Permission::Allowed => {},
        Permission::NeedsApproval => {
            // Send approval request, wait for response
            let decision = self.request_approval(tool_name, args, risk).await?;
            if decision == ApprovalDecision::Denied {
                return Ok(ToolResult::denied());
            }
        },
        Permission::Blocked => {
            return Ok(ToolResult::blocked(
                &format!("Command blocked by sandbox policy (risk: {risk:?})")
            ));
        },
    }

    // Create snapshot if destructive.
    let snapshot = if risk == CommandRisk::Destructive {
        Some(self.create_snapshot(tool_name, args).await?)
    } else {
        None
    };

    // Execute.
    let result = tool.execute(serde_json::from_str(args)?, &ctx).await;

    // Log to audit trail.
    self.log_execution(tool_name, args, &result, risk, snapshot.as_ref()).await;

    result
}
```

---

## 8. Testes

| Teste | Tipo | Descricao |
|-------|------|-----------|
| `test_command_classification` | Unitario | Classifica comandos corretamente |
| `test_sandbox_readonly_blocks_write` | Unitario | ReadOnly bloqueia escrita |
| `test_sandbox_workspace_allows_in_root` | Unitario | WorkspaceWrite permite em root |
| `test_sandbox_workspace_blocks_outside` | Unitario | WorkspaceWrite bloqueia fora do root |
| `test_sandbox_destructive_needs_approval` | Unitario | Destructive pede aprovacao |
| `test_snapshot_creation` | Unitario | Snapshot salva estado antes |
| `test_snapshot_rollback` | Unitario | Rollback restaura estado |
| `test_git_rollback` | Integracao | Git stash/checkout funciona |
| `test_timeout_kills_process` | Unitario | Timeout mata processo |
| `test_output_truncation` | Unitario | Output grande e truncado |
| `test_audit_trail_logged` | Unitario | Execucao registrada no audit |
| `test_shell_interpreter_classification` | Unitario | `bash -c "rm -rf"` classifica como Destructive |

---

## 9. Arquivos a criar/modificar

| Arquivo | Acao | Descricao |
|---------|------|-----------|
| `core/src/sandbox/mod.rs` | **Criar** | SandboxLevel, CommandRisk, classification |
| `core/src/sandbox/classifier.rs` | **Criar** | CommandRisk::classify() |
| `core/src/sandbox/snapshot.rs` | **Criar** | ActionSnapshot + rollback |
| `core/src/sandbox/limits.rs` | **Criar** | ExecutionLimits |
| `core/src/sandbox/audit.rs` | **Criar** | ExecutionLog + persistence |
| `core/src/tools/dispatcher.rs` | Modificar | Integrar sandbox check |
| `tui/src/bottom_pane/approval_overlay.rs` | Modificar | Mostrar risk level no popup |
| `cli/src/audit_cmd.rs` | **Criar** | Subcomando `jarvis audit` |

---

## 10. Estimativa

- **Complexidade**: Media-Alta
- **Tempo estimado**: 2-3 dias (fase basica: classification + path validation + timeout)
- **Fase avancada** (snapshots, rollback, cgroups): +3-5 dias
- **Risco**: Baixo para fase basica, Medio para isolamento de processo
- **Prerequisito**: Nenhum (pode ser implementado em paralelo com Tool Calling)
