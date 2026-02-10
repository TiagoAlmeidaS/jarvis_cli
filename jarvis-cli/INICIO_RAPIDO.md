# ⚡ Início Rápido - Testar Jarvis CLI

## ⚠️ IMPORTANTE: Instalar Rust Primeiro

**Se você recebeu erro "cargo não é reconhecido":**

```powershell
# Opção 1: Usar o script de instalação automática
cd jarvis-cli
.\instalar-rust.ps1

# Opção 2: Instalar manualmente
# Baixe de: https://rustup.rs/
```

**Após instalar:** FECHE e abra um NOVO terminal!

📖 **Guia completo:** Veja `INSTALAR_RUST.md`

---

## 🎯 Forma Mais Rápida de Testar

### Opção 1: Usando o Script PowerShell (Mais Fácil)

```powershell
# Navegar para jarvis-cli
cd jarvis-cli

# Executar o script de teste
.\testar-cli.ps1 help          # Ver ajuda
.\testar-cli.ps1 chat          # Iniciar chat interativo
.\testar-cli.ps1 chat "olá"     # Chat com prompt inicial
.\testar-cli.ps1 test           # Executar testes
.\testar-cli.ps1 build          # Compilar projeto
```

### Opção 2: Executar Diretamente com Cargo

```powershell
# 1. Compilar (primeira vez)
cd jarvis-rs
cargo build

# 2. Executar
cargo run --bin jarvis -- --help
cargo run --bin jarvis -- chat
```

### Opção 3: Usar o Binário Compilado

```powershell
# Após compilar, executar diretamente
cd jarvis-rs
.\target\debug\jarvis.exe --help
.\target\debug\jarvis.exe chat
```

## 📋 Pré-requisitos

- ✅ **Rust** instalado (`rustc --version`) - **OBRIGATÓRIO**
- ✅ **Cargo** (vem com Rust)
- ✅ **Node.js** (opcional, apenas para o wrapper npm)

**Verificar instalação:**
```powershell
rustc --version
cargo --version
```

## 🚀 Comandos Essenciais

```powershell
# Compilar
cargo build

# Executar CLI
cargo run --bin jarvis -- chat

# Testes
cargo test

# Verificar código
cargo clippy

# Formatar código
cargo fmt
```

## 📚 Documentação Completa

Para mais detalhes, consulte:
- **Guia Completo:** `COMO_TESTAR_TERMINAL.md`
- **Documentação Geral:** `README.md`

---

**Dica:** Se você só quer testar rapidamente, use `.\testar-cli.ps1 chat` 🚀
