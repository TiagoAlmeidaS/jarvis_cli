# Problemas Encontrados e Correções Necessárias

## 📋 Resumo

Este documento lista todas as referências ao projeto OpenAI Codex que precisam ser atualizadas para Jarvis no repositório `TiagoAlmeidaS/jarvis-cli`.

## ✅ Já Corrigido

1. **package.json** - URL do repositório atualizada para `github.com/TiagoAlmeidaS/jarvis_cli`

## ⚠️ Pendente de Correção

### 1. `scripts/install_native_deps.py`

**Linha 2:** Comentário ainda menciona "Codex"
```python
"""Install Codex native binaries (Rust CLI plus ripgrep helpers)."""
```
**Deve ser:** `"""Install Jarvis native binaries (Rust CLI plus ripgrep helpers)."""`

**Linha 22:** Variável `CODEX_CLI_ROOT`
```python
CODEX_CLI_ROOT = SCRIPT_DIR.parent
```
**Deve ser:** `jarvis-cli_ROOT`

**Linha 23:** URL do workflow ainda aponta para OpenAI
```python
DEFAULT_WORKFLOW_URL = "https://github.com/openai/codex/actions/runs/17952349351"
```
**Deve ser:** `https://github.com/TiagoAlmeidaS/jarvis-cli/actions/runs/...`

**Linha 47-66:** Referências a componentes "codex-*"
- `"codex"` → `"jarvis"`
- `"codex-responses-api-proxy"` → `"jarvis-responses-api-proxy"`
- `"codex-windows-sandbox-setup"` → `"jarvis-windows-sandbox-setup"`
- `"codex-command-runner"` → `"jarvis-command-runner"`

**Linha 123:** Descrição do parser
```python
parser = argparse.ArgumentParser(description="Install native Codex binaries.")
```
**Deve ser:** `"Install native Jarvis binaries."`

**Linha 270:** Repositório GitHub
```python
"openai/codex",
```
**Deve ser:** `"TiagoAlmeidaS/jarvis-cli"`

### 2. `scripts/build_npm_package.py`

**Linha 19-30:** Referências a pacotes "codex-*"
- `"codex"` → `"jarvis"`
- `"codex-responses-api-proxy"` → `"jarvis-responses-api-proxy"`
- `"codex-sdk"` → `"jarvis-sdk"`

**Linha 36:** Descrição do parser
```python
parser = argparse.ArgumentParser(description="Build or stage the Codex CLI npm package.")
```
**Deve ser:** `"Build or stage the Jarvis CLI npm package."`

**Linha 39-40:** Choices e default
```python
choices=("codex", "codex-responses-api-proxy", "codex-sdk"),
default="codex",
```
**Deve ser:** `choices=("jarvis", "jarvis-responses-api-proxy", "jarvis-sdk")`, `default="jarvis"`

### 3. `Dockerfile`

**Linha 41:** Comentário
```dockerfile
# Install codex
```
**Deve ser:** `# Install jarvis`

**Linha 42:** Nome do arquivo
```dockerfile
COPY dist/codex.tgz codex.tgz
```
**Deve ser:** `COPY dist/jarvis.tgz jarvis.tgz` (ou verificar nome real)

**Linha 50:** Variável de ambiente
```dockerfile
ENV CODEX_UNSAFE_ALLOW_NO_SANDBOX=1
```
**Deve ser:** `ENV JARVIS_UNSAFE_ALLOW_NO_SANDBOX=1` (ou verificar nome real usado no código Rust)

### 4. `README.md`

Todo o README ainda menciona "Codex" e precisa ser atualizado para "Jarvis".

### 5. Arquivos de Configuração

**Questão:** As configurações (`config.toml`) devem estar dentro de `jarvis-cli`?

**Resposta:** Não necessariamente. O arquivo `config.toml` deve estar em:
- `~/.jarvis/config.toml` (configuração do usuário)
- Ou no diretório do projeto como `.jarvis/config.toml`

O `jarvis-cli` é apenas o wrapper npm que chama o binário Rust. As configurações são gerenciadas pelo binário Rust, não pelo wrapper npm.

## 📝 Notas Importantes

1. **Nomes de binários:** Verificar se os binários Rust compilados ainda usam nomes "codex-*" ou já foram renomeados para "jarvis-*"

2. **Variáveis de ambiente:** Verificar se o código Rust usa `CODEX_*` ou `JARVIS_*` para variáveis de ambiente

3. **Workflow URLs:** As URLs de workflows do GitHub Actions precisam ser atualizadas para apontar para o novo repositório

4. **Arquivo `codex.js`:** Existe um arquivo `bin/codex.js` que parece ser idêntico ao `jarvis.js`. Pode ser removido ou mantido para compatibilidade.

## 🔍 Verificações Necessárias

1. Verificar nomes reais dos binários compilados em `jarvis-rs/`
2. Verificar variáveis de ambiente usadas no código Rust
3. Verificar se há outros arquivos que referenciam "codex"
4. Verificar documentação que precisa ser atualizada
