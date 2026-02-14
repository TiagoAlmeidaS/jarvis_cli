# 🔨 Build & Run - Jarvis CLI

Guia completo de comandos para compilar e executar o Jarvis CLI com Databricks.

---

## 🚀 Comando Único (Recomendado)

### Build + Run Automático (Release)
```bash
cd /e/projects/ia/jarvis_cli
chmod +x scripts/BUILD_AND_RUN_COMPLETE.sh
./scripts/BUILD_AND_RUN_COMPLETE.sh
```

### Build + Run em Modo Debug (Compila mais rápido)
```bash
cd /e/projects/ia/jarvis_cli
./scripts/BUILD_AND_RUN_COMPLETE.sh debug
```

---

## 📋 Comandos Separados

### 1. Build Apenas

#### Build Release (Recomendado - Mais rápido em execução)
```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo build --package jarvis-cli --release
```

#### Build Debug (Mais rápido para compilar)
```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo build --package jarvis-cli
```

#### Build com Informações Detalhadas
```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo build --package jarvis-cli --release --verbose
```

### 2. Run Apenas (Após Build)

#### Run com Release Build
```bash
cd /e/projects/ia/jarvis_cli
source ./configure-credentials.sh
cd jarvis-rs
./target/release/jarvis.exe chat \
  -c model_provider=databricks \
  -m databricks-claude-opus-4-5
```

#### Run com Debug Build
```bash
cd /e/projects/ia/jarvis_cli
source ./configure-credentials.sh
cd jarvis-rs
./target/debug/jarvis.exe chat \
  -c model_provider=databricks \
  -m databricks-claude-opus-4-5
```

---

## ⚡ Comandos One-Liner

### Build Release + Run
```bash
cd /e/projects/ia/jarvis_cli && source ./configure-credentials.sh && cd jarvis-rs && cargo build --package jarvis-cli --release && ./target/release/jarvis.exe chat -c model_provider=databricks -m databricks-claude-opus-4-5
```

### Build Debug + Run
```bash
cd /e/projects/ia/jarvis_cli && source ./configure-credentials.sh && cd jarvis-rs && cargo build --package jarvis-cli && ./target/debug/jarvis.exe chat -c model_provider=databricks -m databricks-claude-opus-4-5
```

---

## 🎯 Diferentes Modelos

### Usar Claude Opus 4.5 (Padrão - Melhor para código)
```bash
./target/release/jarvis.exe chat \
  -c model_provider=databricks \
  -m databricks-claude-opus-4-5
```

### Usar Claude Haiku 4.5 (Mais rápido)
```bash
./target/release/jarvis.exe chat \
  -c model_provider=databricks \
  -m databricks-claude-haiku-4-5
```

### Usar Llama 3.1 405B (Raciocínio complexo)
```bash
./target/release/jarvis.exe chat \
  -c model_provider=databricks \
  -m databricks-meta-llama-3-1-405b
```

---

## 🔧 Comandos de Desenvolvimento

### Build Incremental (Apenas o que mudou)
```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo build --package jarvis-cli
```

### Rebuild Completo (Do zero)
```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo clean
cargo build --package jarvis-cli --release
```

### Check (Verifica erros sem compilar)
```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo check --package jarvis-cli
```

### Clippy (Linter Rust)
```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo clippy --package jarvis-cli
```

### Testes
```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo test --package jarvis-core
```

---

## 📊 Comparação: Debug vs Release

| Aspecto | Debug | Release |
|---------|-------|---------|
| **Tempo de compilação** | 2-5 min | 5-15 min |
| **Velocidade de execução** | Lento | Rápido |
| **Tamanho do binário** | ~100 MB | ~75 MB |
| **Debugging** | Sim | Limitado |
| **Uso** | Desenvolvimento | Produção |

**Recomendação**:
- Use **debug** durante desenvolvimento (muitos rebuilds)
- Use **release** para uso diário (melhor performance)

---

## 🛠️ Troubleshooting

### Erro: "binary já em uso"
```bash
# Feche todas as instâncias do Jarvis
pkill -f jarvis.exe

# Ou no Windows:
taskkill /F /IM jarvis.exe
```

### Erro: "cargo: command not found"
```bash
# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Erro: Compilação travada
```bash
# Limpar cache e tentar novamente
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo clean
rm -rf target
cargo build --package jarvis-cli --release
```

### Erro 401 OpenAI (Ainda usando OpenAI)
```bash
# Verificar se credenciais estão exportadas
echo $DATABRICKS_API_KEY
echo $DATABRICKS_BASE_URL

# Se vazias, exportar:
source /e/projects/ia/jarvis_cli/configure-credentials.sh
```

---

## 📝 Scripts Disponíveis

| Script | Descrição |
|--------|-----------|
| `scripts/BUILD_AND_RUN_COMPLETE.sh` | Build + Config + Run (completo) |
| `scripts/RUN_JARVIS.sh` | Apenas Run (assume build já feito) |
| `TEST_NOW.sh` | Teste rápido com release build |
| `TEST_DATABRICKS_COMPLETE.sh` | Teste com provider + modelo |
| `configure-credentials.sh` | Exporta variáveis de ambiente |

---

## 🎯 Workflow Recomendado

### Primeira Vez
```bash
# 1. Build completo
cd /e/projects/ia/jarvis_cli
chmod +x scripts/BUILD_AND_RUN_COMPLETE.sh
./scripts/BUILD_AND_RUN_COMPLETE.sh

# Aguardar compilação (~10-15 min)
```

### Uso Diário (Sem mudanças no código)
```bash
cd /e/projects/ia/jarvis_cli
./scripts/RUN_JARVIS.sh
```

### Desenvolvimento (Com mudanças no código)
```bash
# Build incremental debug (rápido)
cd /e/projects/ia/jarvis_cli/jarvis-rs
cargo build --package jarvis-cli

# Testar
cd ..
./scripts/RUN_JARVIS.sh

# Quando tudo funcionar, fazer build release:
cd jarvis-rs
cargo build --package jarvis-cli --release
```

---

## 🔍 Verificar Build

### Checar versão do binário
```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs
./target/release/jarvis.exe --version
```

### Ver tamanho do binário
```bash
ls -lh /e/projects/ia/jarvis_cli/jarvis-rs/target/release/jarvis.exe
```

### Ver quando foi compilado
```bash
stat /e/projects/ia/jarvis_cli/jarvis-rs/target/release/jarvis.exe
```

---

## 📚 Links Úteis

- **Documentação Rust**: https://doc.rust-lang.org/cargo/
- **Cargo Book**: https://doc.rust-lang.org/cargo/commands/cargo-build.html
- **Troubleshooting**: `E:/projects/ia/jarvis_cli/TROUBLESHOOTING.md`
- **Bug Crítico Corrigido**: `E:/projects/ia/jarvis_cli/BUG_CRITICO_CORRIGIDO.md`
- **Solução Final**: `E:/projects/ia/jarvis_cli/SOLUCAO_FINAL.md`

---

## 💡 Dicas

1. **Sempre use release** para uso normal (muito mais rápido)
2. **Configure aliases** no `.bashrc`:
   ```bash
   alias jarvis-build="cd /e/projects/ia/jarvis_cli/jarvis-rs && cargo build --package jarvis-cli --release"
   alias jarvis-run="cd /e/projects/ia/jarvis_cli && ./scripts/RUN_JARVIS.sh"
   alias jarvis-all="cd /e/projects/ia/jarvis_cli && ./scripts/BUILD_AND_RUN_COMPLETE.sh"
   ```
3. **Use cargo check** antes de build completo (mais rápido)
4. **Limpe target/** periodicamente para economizar espaço

---

## ✅ Checklist Pré-Execução

Antes de executar, certifique-se:

- [ ] Rust instalado (`rustc --version`)
- [ ] Credenciais configuradas (`echo $DATABRICKS_API_KEY`)
- [ ] Diretório correto (`pwd` mostra `.../jarvis_cli`)
- [ ] Permissão de execução nos scripts (`chmod +x *.sh`)

---

## 🎉 Comando Final Recomendado

**Para primeira execução ou após mudanças no código:**
```bash
cd /e/projects/ia/jarvis_cli && ./scripts/BUILD_AND_RUN_COMPLETE.sh
```

**Para uso diário (build já feito):**
```bash
cd /e/projects/ia/jarvis_cli && ./scripts/RUN_JARVIS.sh
```

**One-liner absoluto (tudo de uma vez):**
```bash
cd /e/projects/ia/jarvis_cli && source ./configure-credentials.sh && cd jarvis-rs && cargo build --package jarvis-cli --release && ./target/release/jarvis.exe chat -c model_provider=databricks -m databricks-claude-opus-4-5
```
