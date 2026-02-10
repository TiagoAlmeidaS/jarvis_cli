# 🪟 Opções para Compilar no Windows

## 📋 Situação Atual

A documentação oficial indica que o projeto suporta:
- ✅ **macOS 12+**
- ✅ **Ubuntu 20.04+/Debian 10+**
- ✅ **Windows 11 via WSL2** ⚠️

## 🎯 Duas Opções Disponíveis

### Opção 1: Usar WSL2 (Recomendado pela Documentação) ⭐

**Vantagens:**
- ✅ É a forma oficialmente suportada
- ✅ Ambiente Linux completo
- ✅ Sem problemas de dependências do Windows
- ✅ Mais fácil de configurar

**Desvantagens:**
- ⚠️ Precisa instalar WSL2
- ⚠️ Ambiente Linux dentro do Windows

#### Como Configurar WSL2:

```powershell
# 1. Instalar WSL2 (como Administrador)
wsl --install

# Ou se já tiver WSL, atualizar para WSL2:
wsl --set-default-version 2

# 2. Reiniciar o computador

# 3. Abrir Ubuntu/WSL2 e instalar Rust:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 4. Compilar no WSL2:
cd /mnt/e/projects/ia/jarvis_cli/jarvis-rs
cargo build
```

---

### Opção 2: Compilar Nativamente no Windows (Mais Trabalhoso)

**Vantagens:**
- ✅ Executa diretamente no Windows
- ✅ Sem precisar de WSL2

**Desvantagens:**
- ⚠️ Precisa instalar várias dependências (LLVM, CMake, etc.)
- ⚠️ Pode ter mais problemas
- ⚠️ Não é oficialmente suportado

#### Dependências Necessárias:

1. **LLVM/Clang** (para bindgen)
   ```powershell
   choco install llvm
   # ou baixar de: https://github.com/llvm/llvm-project/releases
   ```

2. **CMake** (já parece estar instalado)
   ```powershell
   choco install cmake
   ```

3. **Visual Studio Build Tools** (já parece estar instalado)

4. **Configurar Variáveis de Ambiente:**
   ```powershell
   $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
   $env:PATH += ";C:\Program Files\LLVM\bin"
   ```

---

## 🤔 Qual Escolher?

### Use WSL2 se:
- ✅ Você quer seguir a documentação oficial
- ✅ Você não se importa em usar Linux dentro do Windows
- ✅ Você quer menos problemas de configuração
- ✅ Você tem Windows 11

### Use Windows Nativo se:
- ✅ Você precisa executar diretamente no Windows
- ✅ Você já tem todas as dependências instaladas
- ✅ Você não quer usar WSL2
- ✅ Você está disposto a resolver problemas de dependências

---

## 🚀 Recomendação

**Para a maioria dos casos, recomendo usar WSL2** porque:
1. É a forma oficialmente suportada
2. É mais fácil de configurar
3. Tem menos problemas de dependências
4. O projeto foi testado nesse ambiente

---

## 📝 Próximos Passos

### Se escolher WSL2:

1. Instalar WSL2: `wsl --install`
2. Reiniciar computador
3. Abrir Ubuntu/WSL2
4. Instalar Rust no WSL2
5. Compilar no WSL2

### Se escolher Windows Nativo:

1. Instalar LLVM: `choco install llvm`
2. Configurar `LIBCLANG_PATH`
3. Tentar compilar novamente: `cargo build`

---

**Qual opção você prefere?** Posso ajudar a configurar qualquer uma das duas! 🚀
