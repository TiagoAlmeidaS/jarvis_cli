# 📚 Documentação Jarvis CLI

Índice completo da documentação do projeto Jarvis CLI com Databricks.

---

## 🚀 Quick Start

### Executar Agora (Build + Run)
```bash
cd /e/projects/ia/jarvis_cli
chmod +x BUILD_AND_RUN_COMPLETE.sh
./BUILD_AND_RUN_COMPLETE.sh
```

### Apenas Run (Build já feito)
```bash
cd /e/projects/ia/jarvis_cli
./RUN_JARVIS.sh
```

---

## 📖 Documentação Principal

### 1. Build & Run
**Arquivo**: [`BUILD_AND_RUN.md`](./BUILD_AND_RUN.md)

Comandos completos para compilar e executar:
- ✅ Build + Run automático
- ✅ Comandos separados (build / run)
- ✅ One-liners
- ✅ Debug vs Release
- ✅ Diferentes modelos Databricks
- ✅ Troubleshooting

### 2. Configuração
**Arquivo**: [`CONFIGURACAO.md`](./CONFIGURACAO.md)

Guia de configuração do projeto:
- Google OAuth
- Databricks API
- OpenAI API
- OpenRouter API
- Arquivos de configuração
- Variáveis de ambiente

### 3. Features
**Diretório**: [`features/`](./features/)

Documentação de funcionalidades implementadas:
- RAG Context Management
- Autonomous Architecture (Phase 1, 2, 3)
- Integrações (Qdrant, Redis, SQL Server)
- Persistence Layer

---

## 🐛 Resolução de Problemas

### Bug Crítico Corrigido
**Arquivo raiz**: [`BUG_CRITICO_CORRIGIDO.md`](../BUG_CRITICO_CORRIGIDO.md)

O problema do provider Databricks e sua correção:
- Provider não lia `DATABRICKS_BASE_URL`
- Sistema fazia fallback para OpenAI
- Correção implementada
- Como verificar se está funcionando

### Troubleshooting Geral
**Arquivo raiz**: [`TROUBLESHOOTING.md`](../TROUBLESHOOTING.md)

Problemas comuns e soluções:
- Erro 401 Unauthorized
- Provider errado
- Variáveis de ambiente
- Comandos de diagnóstico

### Solução Final
**Arquivo raiz**: [`SOLUCAO_FINAL.md`](../SOLUCAO_FINAL.md)

Solução completa do problema OpenAI vs Databricks:
- Por que `-c model_provider=databricks` não bastava
- Importância de especificar o modelo
- Configuração permanente

---

## 📋 Referências Rápidas

### Comandos Corretos
**Arquivo raiz**: [`COMANDOS_CORRETOS.md`](../COMANDOS_CORRETOS.md)

Referência rápida de todos os comandos CLI:
- Flags disponíveis
- Como especificar provider
- Como especificar modelo
- Overrides de configuração

### Problema Identificado
**Arquivo raiz**: [`PROBLEMA_IDENTIFICADO.md`](../PROBLEMA_IDENTIFICADO.md)

Análise técnica do problema:
- Por que o binário estava desatualizado
- Mudanças no código
- Status das configurações
- Tempo de compilação

---

## 🔧 Scripts Disponíveis

### Scripts Principais

| Script | Descrição | Uso |
|--------|-----------|-----|
| `BUILD_AND_RUN_COMPLETE.sh` | Build + Config + Run | `./BUILD_AND_RUN_COMPLETE.sh` |
| `RUN_JARVIS.sh` | Apenas execução | `./RUN_JARVIS.sh` |
| `TEST_NOW.sh` | Teste rápido (release) | `./TEST_NOW.sh` |
| `TEST_DATABRICKS_COMPLETE.sh` | Teste com provider + modelo | `./TEST_DATABRICKS_COMPLETE.sh` |

### Scripts de Configuração

| Script | Descrição | Uso |
|--------|-----------|-----|
| `configure-credentials.sh` | Exporta variáveis (Bash) | `source ./configure-credentials.sh` |
| `configure-credentials.ps1` | Exporta variáveis (PowerShell) | `./configure-credentials.ps1` |
| `bashrc-snippet.sh` | Snippet para ~/.bashrc | `cat bashrc-snippet.sh >> ~/.bashrc` |

---

## 🎯 Modelos Databricks

| Modelo | Nome | Uso |
|--------|------|-----|
| **Claude Opus 4.5** | `databricks-claude-opus-4-5` | Coding, raciocínio complexo |
| **Claude Haiku 4.5** | `databricks-claude-haiku-4-5` | Chat rápido, tarefas simples |
| **Llama 3.1 405B** | `databricks-meta-llama-3-1-405b` | Modelo gigante, raciocínio profundo |

---

## 🔑 Credenciais Necessárias

```bash
# Google OAuth
GOOGLE_CLIENT_ID="765554684645-geg8l26m1vkukn792bfdgtm0urhe905v.apps.googleusercontent.com"

# Databricks
DATABRICKS_API_KEY="your_databricks_api_key_here"
DATABRICKS_BASE_URL="https://your-workspace.azuredatabricks.net"

# OpenAI
OPENAI_API_KEY="your_openai_api_key_here"

# OpenRouter
OPENROUTER_API_KEY="your_openrouter_api_key_here"
```

---

## 📂 Estrutura de Arquivos

```
E:/projects/ia/jarvis_cli/
├── docs/
│   ├── README.md                    # Este arquivo
│   ├── BUILD_AND_RUN.md            # Comandos build & run
│   ├── CONFIGURACAO.md             # Guia de configuração
│   └── features/                    # Documentação de features
├── jarvis-rs/                       # Código fonte Rust
│   ├── cli/                        # CLI principal
│   ├── core/                       # Core do Jarvis
│   ├── tui/                        # Terminal UI
│   └── target/                     # Binários compilados
│       ├── debug/jarvis.exe        # Build debug
│       └── release/jarvis.exe      # Build release
├── .env                            # Variáveis de ambiente (raiz)
├── config.toml                     # Configuração local (opcional)
├── BUILD_AND_RUN_COMPLETE.sh      # Script principal
├── RUN_JARVIS.sh                  # Script de execução
├── configure-credentials.sh        # Config credenciais (Bash)
├── configure-credentials.ps1       # Config credenciais (PowerShell)
├── TROUBLESHOOTING.md             # Troubleshooting
├── BUG_CRITICO_CORRIGIDO.md      # Análise do bug
├── SOLUCAO_FINAL.md              # Solução completa
└── COMANDOS_CORRETOS.md          # Referência de comandos
```

---

## 🎓 Tutoriais

### Primeiro Uso

1. **Clonar/baixar o projeto**
2. **Configurar credenciais**:
   ```bash
   cd /e/projects/ia/jarvis_cli
   source ./configure-credentials.sh
   ```
3. **Build + Run**:
   ```bash
   chmod +x BUILD_AND_RUN_COMPLETE.sh
   ./BUILD_AND_RUN_COMPLETE.sh
   ```
4. **Aguardar compilação** (~10-15 min primeira vez)
5. **Usar Jarvis**!

### Uso Diário

```bash
cd /e/projects/ia/jarvis_cli
./RUN_JARVIS.sh
```

### Desenvolvimento

```bash
# 1. Fazer mudanças no código
# 2. Build incremental
cd jarvis-rs
cargo build --package jarvis-cli

# 3. Testar
cd ..
./RUN_JARVIS.sh

# 4. Se funcionar, build release
cd jarvis-rs
cargo build --package jarvis-cli --release
```

---

## ❓ FAQ

### Como compilar mais rápido?
Use build debug: `cargo build --package jarvis-cli` (sem `--release`)

### Como usar outro modelo?
Adicione `-m <modelo>` ao comando:
```bash
./target/release/jarvis.exe chat -c model_provider=databricks -m databricks-claude-haiku-4-5
```

### Como voltar para OpenAI?
```bash
./target/release/jarvis.exe chat -c model_provider=openai -m gpt-4
```

### Erro 401 ainda aparece?
1. Verifique credenciais: `echo $DATABRICKS_API_KEY`
2. Recompile: `cargo build --package jarvis-cli --release`
3. Use script completo: `./BUILD_AND_RUN_COMPLETE.sh`

### Como limpar cache de compilação?
```bash
cd jarvis-rs
cargo clean
```

---

## 🔗 Links Úteis

- **Rust Installation**: https://rustup.rs/
- **Cargo Book**: https://doc.rust-lang.org/cargo/
- **Databricks Docs**: https://docs.databricks.com/
- **Claude API**: https://docs.anthropic.com/

---

## 📝 Histórico de Mudanças

### 2026-02-09
- ✅ Correção do bug crítico: `create_databricks_provider` agora lê `DATABRICKS_BASE_URL`
- ✅ Implementação do Google OAuth (substituindo ChatGPT)
- ✅ Databricks configurado como provider padrão
- ✅ Documentação completa criada
- ✅ Scripts de build e run criados

---

## 🤝 Contribuindo

Para contribuir com o projeto:
1. Faça mudanças no código
2. Compile: `cargo build --package jarvis-cli`
3. Teste: `cargo test --package jarvis-core`
4. Documente mudanças neste README

---

## 📄 Licença

Projeto baseado no Claude Code da Anthropic.

---

## 🆘 Suporte

Se encontrar problemas:
1. Consulte [`TROUBLESHOOTING.md`](../TROUBLESHOOTING.md)
2. Verifique [`BUG_CRITICO_CORRIGIDO.md`](../BUG_CRITICO_CORRIGIDO.md)
3. Leia [`SOLUCAO_FINAL.md`](../SOLUCAO_FINAL.md)
4. Execute diagnóstico:
   ```bash
   echo "DATABRICKS_API_KEY: ${DATABRICKS_API_KEY:0:10}..."
   echo "DATABRICKS_BASE_URL: $DATABRICKS_BASE_URL"
   ./target/release/jarvis.exe --version
   ```

---

**Última atualização**: 2026-02-09
**Versão**: 1.0.0
