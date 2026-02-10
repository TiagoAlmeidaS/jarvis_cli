# 🪟 Como Compilar no Windows - Guia Completo

## ✅ Boa Notícia!

Você tem **DUAS opções** e ambas funcionam! O projeto **SUPORTA Windows**, mas precisa de algumas dependências.

---

## 🎯 Opção 1: Script Oficial de Setup (MAIS FÁCIL) ⭐ RECOMENDADO

O projeto tem um script oficial que instala **TUDO** automaticamente, incluindo LLVM!

### Como Usar:

```powershell
# 1. Abrir PowerShell como Administrador
#    (Botão direito no PowerShell > "Executar como Administrador")

# 2. Navegar para o diretório jarvis-rs
cd E:\projects\ia\jarvis_cli\jarvis-rs

# 3. Executar o script oficial
powershell -ExecutionPolicy Bypass -File scripts/setup-windows.ps1
```

### O que o script faz:

✅ Instala Rust (via winget)  
✅ Instala Visual Studio Build Tools  
✅ Instala LLVM/Clang (resolve o erro atual!)  
✅ Instala CMake  
✅ Instala Git, ripgrep, just  
✅ Configura todas as variáveis de ambiente  
✅ Compila o projeto automaticamente  

**Tempo estimado:** 15-30 minutos (dependendo da sua internet)

---

## 🐧 Opção 2: Usar WSL2 (Você já tem instalado!)

Como você já tem WSL2 rodando, pode usar essa opção também.

### Como Usar:

```bash
# 1. Abrir WSL2 (Ubuntu)
wsl

# 2. Instalar Rust no WSL2
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 3. Navegar para o projeto (no WSL2, o Windows fica em /mnt/)
cd /mnt/e/projects/ia/jarvis_cli/jarvis-rs

# 4. Compilar
cargo build
```

**Vantagens:**
- ✅ Ambiente Linux completo
- ✅ Sem problemas de dependências do Windows
- ✅ Mais rápido de configurar

---

## 🤔 Qual Escolher?

### Use o Script Oficial (Opção 1) se:
- ✅ Você quer compilar diretamente no Windows
- ✅ Você quer que tudo seja configurado automaticamente
- ✅ Você não se importa em esperar a instalação

### Use WSL2 (Opção 2) se:
- ✅ Você prefere ambiente Linux
- ✅ Você quer algo mais rápido de configurar
- ✅ Você já está familiarizado com Linux

---

## 🚀 Recomendação: Script Oficial

**Recomendo usar o script oficial** porque:
1. É feito pelos desenvolvedores do projeto
2. Instala tudo automaticamente
3. Resolve o problema do LLVM que você está tendo
4. Configura tudo corretamente

---

## 📋 Passo a Passo Rápido (Script Oficial)

```powershell
# 1. Abrir PowerShell como Administrador
#    Botão direito no PowerShell > "Executar como Administrador"

# 2. Executar:
cd E:\projects\ia\jarvis_cli\jarvis-rs
powershell -ExecutionPolicy Bypass -File scripts/setup-windows.ps1

# 3. Aguardar instalação (pode demorar)

# 4. Pronto! O projeto será compilado automaticamente
```

---

## ⚠️ Notas Importantes

- O script precisa ser executado **como Administrador**
- O script usa `winget` (Windows Package Manager) - já vem no Windows 10/11
- A primeira compilação pode levar vários minutos
- Após instalar, você pode usar `cargo build` normalmente

---

## 🐛 Se Der Problema

### Erro: "winget não encontrado"
- Atualize o Windows ou instale winget manualmente

### Erro: "Permissão negada"
- Execute como Administrador

### Erro: "Script não pode ser executado"
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

---

**Quer que eu execute o script para você ou prefere fazer manualmente?** 🚀
