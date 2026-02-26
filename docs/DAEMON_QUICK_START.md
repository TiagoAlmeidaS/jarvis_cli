# 🚀 Quick Start - Dashboard Web do Daemon

Guia rápido para configurar e executar o dashboard web do Jarvis Daemon em 5 minutos.

## ✅ Checklist Rápido

- [ ] Rust instalado
- [ ] `config.toml` configurado com `[api]` e `api_key`
- [ ] Projeto compilado (`cargo build --package jarvis-web-api --release`)
- [ ] Servidor executando (`./target/release/jarvis-web-api`)
- [ ] Dashboard acessível em `http://localhost:3000/daemon`

## 📝 Passo a Passo

### 1. Configurar API Key (2 minutos)

Edite ou crie `~/.jarvis/config.toml`:

```toml
[api]
api_key = "sua-chave-secreta-aqui"  # OBRIGATÓRIO
port = 3000
```

**Gerar API Key:**
```bash
# Linux/Mac
openssl rand -hex 32

# Windows PowerShell
-join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
```

### 2. Compilar (2 minutos)

```bash
cd jarvis-rs
cargo build --package jarvis-web-api --release
```

### 3. Executar (30 segundos)

```bash
# Linux/Mac
./target/release/jarvis-web-api

# Windows
.\target\release\jarvis-web-api.exe
```

### 4. Acessar Dashboard (10 segundos)

1. Abra: `http://localhost:3000/daemon`
2. Digite a API key quando solicitado
3. Pronto! 🎉

## 🔍 Verificar se Funcionou

```bash
# Health check
curl http://localhost:3000/api/health

# Deve retornar: {"status":"ok","version":"..."}
```

## ⚠️ Problemas Comuns

### "Daemon endpoints will be unavailable"
- **Causa**: Banco SQLite não encontrado
- **Solução**: O banco será criado automaticamente. Verifique permissões em `~/.jarvis/`

### "Missing Authorization header"
- **Causa**: API key não configurada
- **Solução**: Adicione `[api]` com `api_key` no `config.toml`

### Dashboard não carrega
- **Causa**: Banco vazio (normal na primeira vez)
- **Solução**: Execute o daemon pelo menos uma vez para gerar dados

## 📚 Próximos Passos

- [Guia Completo de Configuração](./DAEMON_DASHBOARD_SETUP.md)
- [Documentação da API](./DAEMON_API.md)
- [Guia de Monitoramento](./DAEMON_MONITORING.md)
