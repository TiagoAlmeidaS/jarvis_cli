# 🚂 Deploy no Railway - Resumo Rápido

## ⚡ Deploy em 5 Minutos

### 1. Gerar API Key

```powershell
# Windows PowerShell
-join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
```

```bash
# Linux/Mac
openssl rand -hex 32
```

**⚠️ Salve esta key!**

### 2. Push para GitHub

```bash
git add .
git commit -m "Add Railway deployment"
git push
```

### 3. Criar Projeto no Railway

1. Acesse: https://dashboard.railway.app
2. **New Project** → **Deploy from GitHub repo**
3. Selecione seu repositório
4. Railway detectará automaticamente o `railway.json`

### 4. Configurar API Key

No dashboard Railway:
1. Clique em **Variables**
2. Adicione: `JARVIS_API_KEY` = `sua-key-gerada`
3. (Opcional) Adicione: `RUST_LOG` = `info`

### 5. Deploy

Railway fará deploy automaticamente!

Aguarde 5-10 minutos para o primeiro build.

### 6. Testar

```bash
# Substitua pela URL do seu projeto
curl https://seu-projeto.up.railway.app/api/health
```

## ✅ Pronto!

Sua API estará em: `https://seu-projeto.up.railway.app`

---

## 📚 Documentação Completa

- **Guia Completo**: [DEPLOY_RAILWAY.md](./DEPLOY_RAILWAY.md)
- **Quick Start**: [QUICK_START_RAILWAY.md](./QUICK_START_RAILWAY.md)
- **API Docs**: [README.md](./README.md)

---

## 🔧 Comandos Úteis

```bash
# Instalar Railway CLI
# Windows:
iwr https://railway.app/install.ps1 | iex

# Linux/Mac:
curl -fsSL https://railway.app/install.sh | sh

# Login
railway login

# Ver logs
railway logs

# Ver variáveis
railway variables

# Adicionar variável
railway variables set JARVIS_API_KEY="sua-key"
```

---

## 🆘 Problemas?

Veja [DEPLOY_RAILWAY.md](./DEPLOY_RAILWAY.md) seção "Troubleshooting"
