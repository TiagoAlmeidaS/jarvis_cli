# Estratégia de Manutenção Codex-Jarvis

## Problema

O projeto tem duas cópias do código:

- `codex-rs/` - upstream (atualizado automaticamente)
- `jarvis-rs/` - suas customizações (fork manual)

Atualizar o upstream requer trabalho manual para evitar sobrescrever suas customizações.

---

## Estratégia Recomendada: Fork com Updates Manuais

### Estrutura Mantida

```
jarvis_cli/
├── codex-rs/          # Upstream - NÃO MODIFICAR
├── jarvis-rs/         # Suas customizações
└── ...
```

### Como Atualizar o Codex

**Passo 1: Backup**

```bash
git checkout homolog
git pull origin homolog
git backup jarvis-rs/
```

**Passo 2: Remover codex-rs antigo**

```bash
rm -rf codex-rs
```

**Passo 3: Adicionar upstream como remote**

```bash
git remote add upstream git@github.com:openai/codex.git
git fetch upstream
```

**Passo 4: Baixar codex-rs do upstream**

```bash
# Criar branch para update
git checkout -b chore/update-codex

# Baixar apenas codex-rs do upstream
git checkout upstream/main -- codex-rs/
git commit -m "chore: sync codex-rs from upstream"
```

**Passo 5: Resolver conflitos (se houver)**

```bash
# Se houver conflitos em codex-rs, resolver manualmente
# Suas customizações em jarvis-rs NÃO serão afetadas
```

**Passo 6: Testar**

```bash
cd codex-rs && cargo check
```

**Passo 7: Merge para homolog**

```bash
git checkout homolog
git merge chore/update-codex
git push origin homolog
```

---

## Alternativa: Usar [patch] no Cargo.toml

Se quiser que jarvis-rs use codex-core como base:

```toml
# jarvis-rs/Cargo.toml (se existir)

[patch.crates-io]
codex-core = { path = "../codex-rs/core" }
codex-cli = { path = "../codex-rs/cli" }
# ... outras crates
```

---

## Cronograma de Updates

| Frequência  | Ação                              |
| ----------- | --------------------------------- |
| Semanal     | Verificar novas releases do Codex |
| Mensal      | Executar sync completo            |
| Por demanda | Bug fixes críticos                |

---

## Checklist de Sync

- [ ] Backup do estado atual
- [ ] Remover codex-rs/
- [ ] Baixar codex-rs/ do upstream
- [ ] Testar build
- [ ] Verificar jarvis-rs/ continua funcionando
- [ ] Commit e push para homolog
- [ ] Testar em ambiente de staging

---

## Riscos

| Risco                | Mitigação                       |
| -------------------- | ------------------------------- |
| Perder customizações | Backup antes de qualquer update |
| Conflitos de merge   | Testar em branch separada       |
| Break build          | Sempre testar após sync         |

---

_Documento atualizado: Março 2026_
