# 🚀 Guia de Uso: dev-jarvis

Scripts para desenvolvimento rápido do Jarvis CLI com diferentes providers e modelos.

## 📍 Localização dos Scripts

Os scripts estão em `scripts/` (raiz do projeto), **NÃO** em `jarvis-rs/scripts/`:

```
jarvis_cli/
├── scripts/
│   ├── dev-jarvis.bat    ← Windows (PowerShell/CMD)
│   └── dev-jarvis.sh     ← Git Bash/Linux/Mac
└── jarvis-rs/
    └── ...
```

## 🪟 Windows (PowerShell/CMD)

### Executar da raiz do projeto:

```powershell
# Navegar para raiz do projeto
cd E:\projects\ia\jarvis_cli

# Executar script
.\scripts\dev-jarvis.bat free
```

### Ou executar de qualquer lugar:

```powershell
# Se estiver em jarvis-rs/scripts
cd E:\projects\ia\jarvis_cli
.\scripts\dev-jarvis.bat free
```

## 🐧 Git Bash / Linux / Mac

### Opção 1: Usar o script shell (recomendado)

```bash
# Navegar para raiz do projeto
cd /e/projects/ia/jarvis_cli

# Dar permissão de execução (primeira vez)
chmod +x scripts/dev-jarvis.sh

# Executar
./scripts/dev-jarvis.sh free
```

### Opção 2: Usar o .bat via cmd.exe (Git Bash)

```bash
# Navegar para raiz do projeto
cd /e/projects/ia/jarvis_cli

# Executar via cmd.exe
cmd.exe //c scripts\\dev-jarvis.bat free
```

## 📋 Exemplos de Uso

### Estratégia Free (Padrão)

```bash
# Windows PowerShell
.\scripts\dev-jarvis.bat free

# Git Bash
./scripts/dev-jarvis.sh free
```

### Free com modelo customizado

```bash
# Windows
.\scripts\dev-jarvis.bat free stepfun/step-3.5-flash:free

# Git Bash
./scripts/dev-jarvis.sh free stepfun/step-3.5-flash:free
```

### Google AI Studio Free

```bash
# Windows
.\scripts\dev-jarvis.bat free_google

# Git Bash
./scripts/dev-jarvis.sh free_google
```

### Release Build

```bash
# Windows
.\scripts\dev-jarvis.bat --release free

# Git Bash
./scripts/dev-jarvis.sh --release free
```

## ⚠️ Problemas Comuns

### Erro: "No such file or directory" (Git Bash)

**Causa**: Tentando executar `.bat` diretamente no Git Bash.

**Solução**: Use o script `.sh` ou execute via `cmd.exe`:
```bash
cmd.exe //c scripts\\dev-jarvis.bat free
```

### Erro: "Script não encontrado"

**Causa**: Está no diretório errado.

**Solução**: Navegue para a raiz do projeto:
```bash
cd /e/projects/ia/jarvis_cli  # Git Bash
cd E:\projects\ia\jarvis_cli  # PowerShell
```

### Erro: "Permission denied" (Linux/Mac)

**Causa**: Script sem permissão de execução.

**Solução**:
```bash
chmod +x scripts/dev-jarvis.sh
```

## 🔑 Configuração de API Keys

### OpenRouter (Free Strategy)

```bash
# Windows PowerShell
$env:OPENROUTER_API_KEY = "sk-or-v1-..."

# Git Bash / Linux / Mac
export OPENROUTER_API_KEY="sk-or-v1-..."
```

### Google AI Studio

```bash
# Windows PowerShell
$env:GOOGLE_API_KEY = "..."

# Git Bash / Linux / Mac
export GOOGLE_API_KEY="..."
```

## 📚 Mais Informações

- [Guia Completo da Estratégia Free](../../docs/ESTRATEGIA_FREE_CLI.md)
- [Quick Start Free](../../docs/QUICK_START_FREE.md)
