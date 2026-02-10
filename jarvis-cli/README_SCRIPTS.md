# 📜 Scripts PowerShell para Jarvis CLI

## 🚀 Scripts Disponíveis

### 1. `configurar-rust.ps1` ⭐ **EXECUTE PRIMEIRO**

Adiciona Rust ao PATH do sistema permanentemente.

```powershell
.\configurar-rust.ps1
```

**O que faz:**
- Adiciona `C:\Users\<seu-usuario>\.cargo\bin` ao PATH do usuário
- Funciona permanentemente (não precisa executar toda vez)
- **IMPORTANTE:** Após executar, feche e abra um NOVO terminal

---

### 2. `iniciar-build.ps1` ⭐ **PARA COMPILAR**

Adiciona Rust ao PATH e inicia a compilação do projeto.

```powershell
.\iniciar-build.ps1
```

**O que faz:**
- Adiciona Rust ao PATH da sessão atual
- Navega para `jarvis-rs`
- Executa `cargo build`
- Mostra próximos passos após compilar

---

### 3. `testar-cli.ps1` ⭐ **PARA TESTAR**

Testa o CLI após compilar.

```powershell
.\testar-cli.ps1 help          # Ver ajuda
.\testar-cli.ps1 chat          # Iniciar chat
.\testar-cli.ps1 chat "olá"    # Chat com prompt
.\testar-cli.ps1 test           # Executar testes
.\testar-cli.ps1 build          # Compilar
```

**O que faz:**
- Adiciona Rust ao PATH automaticamente se necessário
- Compila o projeto se necessário
- Executa comandos do CLI

---

### 4. `adicionar-rust-path.ps1`

Adiciona Rust ao PATH apenas da sessão atual (temporário).

```powershell
.\adicionar-rust-path.ps1
```

**Uso:** Quando você precisa usar Rust mas não quer reiniciar o terminal.

---

### 5. `instalar-rust.ps1`

Instala Rust no Windows (se ainda não tiver instalado).

```powershell
.\instalar-rust.ps1
```

---

## 🎯 Fluxo Recomendado

### Primeira Vez (Configuração)

```powershell
# 1. Configurar Rust no PATH permanentemente
.\configurar-rust.ps1

# 2. FECHAR e abrir um NOVO terminal

# 3. Verificar se funcionou
cargo --version

# 4. Compilar o projeto
.\iniciar-build.ps1
```

### Uso Diário

```powershell
# Opção 1: Usar o script de teste (mais fácil)
.\testar-cli.ps1 chat

# Opção 2: Compilar manualmente
cd jarvis-rs
cargo build
cargo run --bin jarvis -- chat
```

## ⚠️ Problemas Comuns

### Erro: "cargo não é reconhecido"

**Solução 1:** Execute o script de configuração:
```powershell
.\configurar-rust.ps1
# Depois feche e abra um novo terminal
```

**Solução 2:** Adicionar manualmente nesta sessão:
```powershell
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
cargo --version
```

### Erro: "Script não pode ser executado"

**Solução:** Alterar política de execução:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Rust não encontrado após instalar

**Solução:**
1. Verificar se está instalado: `Test-Path "$env:USERPROFILE\.cargo\bin\cargo.exe"`
2. Se estiver, executar: `.\configurar-rust.ps1`
3. Reiniciar terminal

## 📋 Checklist

- [ ] Rust instalado (`rustc --version`)
- [ ] Rust configurado no PATH (`.\configurar-rust.ps1`)
- [ ] Terminal reiniciado após configuração
- [ ] Projeto compilado (`.\iniciar-build.ps1`)
- [ ] CLI testado (`.\testar-cli.ps1 help`)

---

**Dica:** Execute `.\configurar-rust.ps1` uma vez e nunca mais terá problemas com PATH! 🎉
