# 🚂 Deploy no Railway - Guia Completo

Railway é uma excelente escolha para deploy rápido e simples:
- ✅ **Deploy automático** via Git
- ✅ **HTTPS automático** (sem configuração)
- ✅ **Dashboard intuitivo**
- ✅ **$5 créditos/mês** no plano gratuito
- ✅ **Sem configuração complexa** de servidor
- ✅ **Logs em tempo real**

---

## 📋 Pré-requisitos

1. **Conta no Railway**
   - Acesse: https://railway.app
   - Faça login com GitHub

2. **Repositório Git**
   - Código no GitHub/GitLab
   - Railway conecta automaticamente

3. **Railway CLI** (Opcional, mas recomendado)
   ```powershell
   # Windows (PowerShell)
   iwr https://railway.app/install.ps1 | iex
   
   # Linux/Mac
   curl -fsSL https://railway.app/install.sh | sh
   ```

---

## 🚀 Deploy Rápido (3 Passos)

### **Passo 1: Preparar Repositório**

Certifique-se de que o código está commitado:

```bash
git add .
git commit -m "Add Railway deployment config"
git push
```

### **Passo 2: Criar Projeto no Railway**

#### Opção A: Via Dashboard (Mais Fácil)

1. Acesse https://dashboard.railway.app
2. Clique em **"New Project"**
3. Selecione **"Deploy from GitHub repo"**
4. Escolha seu repositório
5. Railway detectará automaticamente o `railway.json`

#### Opção B: Via CLI

```bash
# Login
railway login

# No diretório do projeto
cd jarvis-rs/web-api

# Inicializar projeto
railway init

# Linkar com projeto existente (se já criou no dashboard)
# railway link
```

### **Passo 3: Configurar Variáveis de Ambiente**

#### Via Dashboard:

1. No projeto Railway, clique em **"Variables"**
2. Adicione as seguintes variáveis:

| Variável | Valor | Descrição |
|----------|-------|-----------|
| `JARVIS_API_KEY` | `sua-api-key-gerada` | **OBRIGATÓRIO** - API Key para autenticação |
| `RUST_LOG` | `info` | Nível de logging (opcional) |
| `JARVIS_HOME` | `/app/.jarvis` | Diretório do Jarvis (opcional) |

#### Via CLI:

```bash
railway variables set JARVIS_API_KEY="sua-api-key-aqui"
railway variables set RUST_LOG=info
```

**Gerar API Key:**
```powershell
# Windows PowerShell
-join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})

# Linux/Mac
openssl rand -hex 32
```

### **Passo 4: Deploy**

Railway fará deploy automaticamente quando você:
- Fizer push para o repositório (se configurado)
- Ou manualmente via dashboard/cli

**Via CLI:**
```bash
railway up
```

**Via Dashboard:**
- Clique em **"Deploy"** ou aguarde deploy automático

---

## 🔧 Configuração Detalhada

### **railway.json**

O arquivo `railway.json` já está configurado:

```json
{
  "$schema": "https://railway.app/railway.schema.json",
  "build": {
    "builder": "DOCKERFILE",
    "dockerfilePath": "jarvis-rs/web-api/Dockerfile",
    "dockerContext": "jarvis-rs"
  },
  "deploy": {
    "startCommand": "/app/jarvis-web-api",
    "healthcheckPath": "/api/health",
    "healthcheckTimeout": 100,
    "restartPolicyType": "ON_FAILURE",
    "restartPolicyMaxRetries": 10
  }
}
```

### **Dockerfile**

O Dockerfile está otimizado para Railway:
- Build multi-stage (menor imagem final)
- Suporta variável `PORT` (Railway define automaticamente)
- Health check configurado

### **Variáveis de Ambiente Importantes**

Railway define automaticamente:
- `PORT` - Porta do servidor (não precisa definir)
- `RAILWAY_ENVIRONMENT` - Ambiente atual

Você precisa definir:
- `JARVIS_API_KEY` - **OBRIGATÓRIO**

Opcionais:
- `RUST_LOG` - Nível de logging
- `JARVIS_HOME` - Diretório do Jarvis

---

## 📊 Monitoramento

### **Ver Logs**

**Via Dashboard:**
1. Abra o projeto no Railway
2. Clique em **"Deployments"**
3. Selecione o deployment
4. Veja logs em tempo real

**Via CLI:**
```bash
railway logs
railway logs --follow  # Seguir logs em tempo real
```

### **Ver Métricas**

No dashboard Railway:
- CPU usage
- Memory usage
- Network traffic
- Request count

### **Health Check**

Railway verifica automaticamente:
- Endpoint: `/api/health`
- Timeout: 100ms
- Se falhar, Railway reinicia o serviço

---

## 🔒 Segurança

### **API Key**

⚠️ **IMPORTANTE**: Nunca commite a API key no Git!

Use apenas variáveis de ambiente no Railway.

### **HTTPS**

Railway fornece HTTPS automático:
- Certificado SSL automático
- Domínio: `seu-projeto.up.railway.app`
- Domínio customizado: Configurável no dashboard

### **Rate Limiting**

Railway tem rate limiting básico, mas você pode adicionar:
- Rate limiting por IP (futuro)
- Rate limiting por API key (futuro)

---

## 🧪 Testar Deploy

### **1. Verificar Health Check**

```bash
# Substitua pela URL do seu projeto
curl https://jarvis-web-api.up.railway.app/api/health
```

Deve retornar:
```json
{"status":"ok","version":"0.0.0"}
```

### **2. Testar Chat**

```powershell
# Windows PowerShell
$headers = @{
    "Authorization" = "Bearer SUA_API_KEY"
    "Content-Type" = "application/json"
}
$body = @{
    prompt = "Olá, Jarvis!"
} | ConvertTo-Json

Invoke-RestMethod -Uri "https://seu-projeto.up.railway.app/api/chat" -Method Post -Headers $headers -Body $body
```

```bash
# Linux/Mac
curl -X POST https://seu-projeto.up.railway.app/api/chat \
  -H "Authorization: Bearer SUA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Olá, Jarvis!"}'
```

### **3. Testar Interface Web**

Acesse no navegador:
```
https://seu-projeto.up.railway.app/
```

---

## 🔄 Atualizar Deploy

### **Deploy Automático**

Se configurado, Railway faz deploy automaticamente quando você faz push:

```bash
git add .
git commit -m "Atualização"
git push
```

Railway detecta e faz deploy automaticamente.

### **Deploy Manual**

**Via Dashboard:**
1. Clique em **"Redeploy"** no deployment atual

**Via CLI:**
```bash
railway up
```

---

## 🐛 Troubleshooting

### **Erro: "API not configured"**

**Causa**: `JARVIS_API_KEY` não está definida

**Solução**:
1. Vá em **Variables** no dashboard
2. Adicione `JARVIS_API_KEY` com um valor
3. Faça redeploy

### **Erro: "Failed to bind port"**

**Causa**: Aplicação não está usando a variável `PORT`

**Solução**: Já está configurado no código para usar `PORT` automaticamente

### **Erro: "Build failed"**

**Causa**: Dependências ou compilação falhou

**Solução**:
1. Verifique logs do build no dashboard
2. Certifique-se de que o Dockerfile está correto
3. Verifique se todas as dependências estão no `Cargo.toml`

### **App não inicia**

**Solução**:
1. Verifique logs: `railway logs`
2. Verifique variáveis de ambiente
3. Teste localmente primeiro

### **Health check falhando**

**Causa**: Endpoint `/api/health` não está respondendo

**Solução**:
1. Verifique se o servidor está rodando
2. Verifique logs para erros
3. Teste manualmente: `curl https://seu-app/api/health`

---

## 💰 Custos e Limites

### **Plano Gratuito (Hobby)**

- ✅ $5 créditos/mês
- ✅ Deploy ilimitado
- ✅ HTTPS automático
- ✅ 512MB RAM por serviço
- ✅ 1GB storage
- ⚠️ Créditos podem acabar com uso intenso

### **Monitoramento de Uso**

No dashboard Railway:
- Veja créditos restantes
- Veja uso de recursos
- Configure alertas (opcional)

---

## 🎯 Próximos Passos

Após deploy bem-sucedido:

1. **Testar acesso remoto**
   - Do celular
   - De outro computador
   - Interface web funcionando

2. **Configurar domínio customizado** (opcional)
   - No dashboard Railway
   - Adicionar domínio
   - Configurar DNS

3. **Monitorar uso**
   - Verificar créditos
   - Acompanhar logs
   - Verificar métricas

4. **Otimizar** (se necessário)
   - Ajustar recursos
   - Configurar auto-sleep (se disponível)
   - Otimizar build time

---

## 📚 Comandos Úteis

```bash
# Login
railway login

# Ver projetos
railway list

# Ver status
railway status

# Ver logs
railway logs
railway logs --follow

# Ver variáveis
railway variables

# Adicionar variável
railway variables set KEY=value

# Deploy
railway up

# Abrir no navegador
railway open
```

---

## ✅ Checklist de Deploy

- [ ] Conta Railway criada
- [ ] Repositório no GitHub
- [ ] Código commitado e pushed
- [ ] Projeto criado no Railway
- [ ] Repositório conectado
- [ ] `JARVIS_API_KEY` configurada
- [ ] Deploy iniciado
- [ ] Build concluído com sucesso
- [ ] Health check funcionando
- [ ] API respondendo
- [ ] Interface web acessível
- [ ] Testado acesso remoto

---

## 🔗 Links Úteis

- [Railway Dashboard](https://dashboard.railway.app)
- [Railway Docs](https://docs.railway.app)
- [Documentação da API](./README.md)
- [Status do Plano](./PLANO_STATUS.md)

---

## 💡 Dicas

1. **Use Railway CLI** para facilitar gerenciamento
2. **Monitore créditos** para não exceder o limite
3. **Configure alertas** no dashboard (opcional)
4. **Teste localmente primeiro** antes de fazer deploy
5. **Mantenha API key segura** - nunca no código

---

## 🆘 Suporte

- Railway Support: https://railway.app/help
- Railway Discord: https://discord.gg/railway
- Logs do projeto: Dashboard → Deployments → Logs
