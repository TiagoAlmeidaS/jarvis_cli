# 🔧 Instalar Rust no Windows

## Método 1: Instalação Automática (Recomendado)

### Passo 1: Baixar o Instalador

```powershell
# Baixar o instalador do rustup
Invoke-WebRequest https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
```

### Passo 2: Executar o Instalador

```powershell
# Executar o instalador
.\rustup-init.exe
```

**Durante a instalação:**
- Pressione `1` para instalação padrão (recomendado)
- O instalador irá:
  - Instalar Rust e Cargo
  - Adicionar ao PATH do sistema
  - Configurar o ambiente

### Passo 3: Reiniciar o Terminal

**IMPORTANTE:** Feche e abra novamente o PowerShell/terminal para que as mudanças no PATH tenham efeito.

### Passo 4: Verificar Instalação

```powershell
rustc --version
cargo --version
```

## Método 2: Instalação Manual via Site

1. Acesse: https://rustup.rs/
2. Baixe o instalador para Windows
3. Execute `rustup-init.exe`
4. Siga as instruções na tela
5. Reinicie o terminal

## Método 3: Usar Chocolatey (se você tem Chocolatey)

```powershell
choco install rust
```

## ⚠️ Após Instalar

### 1. Reiniciar o Terminal

**CRÍTICO:** Feche completamente o PowerShell e abra novamente para que o PATH seja atualizado.

### 2. Verificar Instalação

```powershell
# Verificar versão do Rust
rustc --version

# Verificar versão do Cargo
cargo --version

# Verificar instalação completa
rustup show
```

### 3. Instalar Componentes Adicionais (Recomendado)

```powershell
# Instalar rustfmt (formatador de código)
rustup component add rustfmt

# Instalar clippy (verificador de código)
rustup component add clippy
```

## 🎯 Testar Após Instalação

```powershell
# Navegar para o projeto
cd E:\projects\ia\jarvis_cli\jarvis-rs

# Compilar o projeto
cargo build

# Executar o CLI
cargo run --bin jarvis -- --help
```

## 🐛 Problemas Comuns

### Erro: "cargo não é reconhecido" após instalação

**Solução:**
1. Feche completamente o PowerShell
2. Abra um novo PowerShell
3. Verifique: `cargo --version`

Se ainda não funcionar:

```powershell
# Verificar se está no PATH
$env:PATH -split ';' | Select-String -Pattern 'cargo'

# Adicionar manualmente ao PATH da sessão atual
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

### Erro durante instalação

- Certifique-se de ter permissões de administrador
- Verifique sua conexão com a internet
- Tente executar como administrador

### Verificar instalação existente

```powershell
# Verificar se rustup está instalado
Test-Path "$env:USERPROFILE\.cargo\bin\rustup.exe"

# Se existir, adicionar ao PATH da sessão atual
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

## 📚 Recursos Adicionais

- **Site oficial:** https://www.rust-lang.org/
- **Documentação:** https://doc.rust-lang.org/
- **Rustup:** https://rustup.rs/

---

**Dica:** Após instalar, sempre reinicie o terminal antes de usar o cargo! 🔄
