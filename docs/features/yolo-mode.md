# Yolo Mode — Auto-Approve All Actions

**Status**: Implementado
**Prioridade**: Media
**Versao**: 1.0.0
**Ultima atualizacao**: 2026-02-13

---

## 1. Visao Geral

O "Yolo Mode" permite que o usuario ative o auto-approve de todas as acoes
do Jarvis durante uma sessao, sem precisar reiniciar o TUI ou mudar configuracoes
permanentes. Similar ao modo "Auto-approve" do Cursor.

## 2. Comandos

### /approve-all (Yolo Mode)

Ativa o modo de auto-aprovacao total para a sessao atual:
- Nenhuma acao pedira confirmacao (shell, edits, patches, etc.)
- Equivalente ao preset "Full Access" (`AskForApproval::Never` + `SandboxPolicy::DangerFullAccess`)
- Efeito apenas na sessao atual — nao altera config permanente

```
/approve-all
```

Mensagem exibida:
> Yolo mode activated! Jarvis will auto-approve all actions for this session.

### /approve-default (Restaurar padrao)

Restaura a politica de aprovacao padrao:
- Jarvis pode ler/editar no workspace, pede permissao para internet e arquivos externos
- Equivalente ao preset "Default" (`AskForApproval::OnRequest` + workspace write policy)

```
/approve-default
```

Mensagem exibida:
> Default approval mode restored. Jarvis will ask for approval on sensitive actions.

## 3. Alternativas existentes

Alem dos slash commands, existem outras formas de controlar permissoes:

| Metodo | Escopo | Permanente? |
|--------|--------|-------------|
| `/approve-all` | Sessao atual | Nao |
| `/approve-default` | Sessao atual | Nao |
| `/permissions` ou `/approvals` | Sessao atual | Nao |
| `--approval-mode never` (CLI arg) | Sessao atual | Nao |
| Trust Directory (onboarding) | Projeto | Sim |
| `config.toml` | Global | Sim |

## 4. Niveis de aprovacao disponíveis

| Nivel | Enum | Descricao |
|-------|------|-----------|
| Untrusted | `AskForApproval::UnlessTrusted` | So auto-aprova comandos seguros (ls, cat) |
| On Failure | `AskForApproval::OnFailure` | Roda em sandbox, so pede se falhar |
| On Request | `AskForApproval::OnRequest` | Modelo decide quando pedir (default) |
| Never | `AskForApproval::Never` | Nunca pede aprovacao (yolo mode) |

## 5. Approve All direto no popup de aprovacao

Alem dos slash commands, o Yolo Mode tambem pode ser ativado diretamente no popup
de aprovacao que aparece durante uma task. Quando o Jarvis pede permissao para
executar um comando ou aplicar um patch, as opcoes agora incluem:

**Para comandos (exec):**
1. `y` — Yes, proceed (aprova apenas esta acao)
2. `p` — Yes, and don't ask again for commands that start with... (exec policy)
3. `a` — Yes, and don't ask again this session (approve all) **← Yolo Mode**
4. `Esc/n` — No, and tell Jarvis what to do differently

**Para patches (file edits):**
1. `y` — Yes, proceed
2. `f` — Yes, and don't ask again for these files
3. `a` — Yes, and don't ask again this session (approve all) **← Yolo Mode**
4. `Esc/n` — No, and tell Jarvis what to do differently

Ao selecionar a opcao "approve all", a acao atual e aprovada E todas as acoes
futuras na sessao serao auto-aprovadas (mesmo efeito do `/approve-all`).

## 6. Arquivos modificados

- `tui/src/slash_command.rs` — Novos comandos `ApproveAll` e `ApproveDefault`
- `tui/src/chatwidget.rs` — Handlers `activate_approve_all_mode()` e `restore_default_approval_mode()`
- `tui/src/bottom_pane/approval_overlay.rs` — Opcao "approve all session" nos popups de exec e patch
