# Deploy do Jarvis Daemon na VPS

**Objetivo**: Publicar o daemon Jarvis em uma VPS Linux para que rode 24/7, gerando conteúdo, coletando métricas e executando propostas autonomamente.

**Sim, você precisa acessar a VPS** para configurar o ambiente, copiar os binários e criar o serviço. Este guia descreve o passo a passo.

---

## 1. Pré-requisitos

### Na sua máquina local (Windows)
- Projeto Jarvis compilado
- Rust instalado (para compilar o binário)
- Acesso SSH à VPS

### Na VPS (Linux)
- Ubuntu 20.04+ ou Debian 11+
- Mínimo 1 GB RAM (2 GB+ recomendado para LLM)
- 5 GB+ espaço em disco
- Conexão com internet estável

---

## 2. Compilar o binário

Na sua máquina local, dentro do projeto:

```powershell
cd E:\projects\ia\jarvis_cli\jarvis-rs

# Compilar daemon em release (binário otimizado)
cargo build --release --bin jarvis-daemon

# Compilar também o CLI (para gerenciar o daemon remotamente)
cargo build --release --bin jarvis
```

Os binários ficam em:
- `jarvis-rs/target/release/jarvis-daemon` (ou `.exe` no Windows)
- `jarvis-rs/target/release/jarvis` (ou `jarvis.exe`)

**Importante**: Se sua VPS for Linux, você precisa compilar para o alvo Linux. Opções:

1. **Cross-compile** (na sua máquina Windows para Linux):
   ```powershell
   rustup target add x86_64-unknown-linux-gnu
   cargo build --release --bin jarvis-daemon --target x86_64-unknown-linux-gnu
   ```
   (Pode exigir ferramentas adicionais como `mingw-w64` ou WSL.)

2. **Compilar direto na VPS** (mais simples):
   - Instale Rust na VPS: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   - Clone o projeto ou copie via `scp`/`rsync`
   - Execute `cargo build --release --bin jarvis-daemon` na VPS

---

## 3. Estrutura na VPS

Crie a estrutura de diretórios:

```bash
# Conectar na VPS
ssh usuario@seu-ip-vps

# Criar diretório do Jarvis
sudo mkdir -p /opt/jarvis
sudo chown $USER:$USER /opt/jarvis
cd /opt/jarvis

# Diretório para o usuário que rodará o daemon
mkdir -p ~/.jarvis/credentials
```

### Arquivos necessários

| Caminho | Descrição |
|---------|-----------|
| `/opt/jarvis/jarvis-daemon` | Binário do daemon |
| `/opt/jarvis/jarvis` | Binário do CLI (opcional, para gerenciar) |
| `~/.jarvis/daemon.db` | Banco SQLite (criado automaticamente) |
| `~/.jarvis/credentials/google.json` | Tokens OAuth do Google (após `auth google`) |
| `~/.jarvis/.env` ou `/opt/jarvis/.env` | Variáveis de ambiente |

---

## 4. Copiar binários para a VPS

### Opção A: SCP (da sua máquina para a VPS)

```powershell
# Da sua máquina Windows (PowerShell)
scp E:\projects\ia\jarvis_cli\jarvis-rs\target\release\jarvis-daemon usuario@seu-ip-vps:/opt/jarvis/
scp E:\projects\ia\jarvis_cli\jarvis-rs\target\release\jarvis usuario@seu-ip-vps:/opt/jarvis/

# Tornar executáveis (na VPS)
ssh usuario@seu-ip-vps "chmod +x /opt/jarvis/jarvis-daemon /opt/jarvis/jarvis"
```

### Opção B: Compilar na VPS

```bash
# Na VPS
ssh usuario@seu-ip-vps

# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Clonar ou copiar o projeto
cd /opt
git clone <url-do-repo> jarvis-src
# OU: rsync -avz /e/projects/ia/jarvis_cli usuario@vps:/opt/jarvis-src

cd /opt/jarvis-src/jarvis-rs
cargo build --release --bin jarvis-daemon --bin jarvis

# Copiar binários
cp target/release/jarvis-daemon /opt/jarvis/
cp target/release/jarvis /opt/jarvis/
```

---

## 5. Variáveis de ambiente

Crie o arquivo `/opt/jarvis/.env` (ou `~/.jarvis/.env`):

```bash
# === LLM (obrigatório para pipelines que usam LLM) ===
# OpenRouter (recomendado para modelos baratos)
OPENROUTER_API_KEY=sua_chave_aqui

# OU Google Gemini (free tier)
# GOOGLE_API_KEY=sua_chave_aqui

# === WordPress (para publicar artigos) ===
WORDPRESS_APP_PASSWORD=xxxx xxxx xxxx xxxx

# === Google OAuth (Search Console + AdSense) ===
# Necessário apenas para coleta de métricas reais
GOOGLE_CLIENT_ID=xxx.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=xxx

# === Telegram (notificações) ===
JARVIS_TELEGRAM_BOT_TOKEN=123456:ABC...
JARVIS_TELEGRAM_CHAT_ID=123456789
JARVIS_NOTIFY_HOUR=8

# === Opcional: caminho do banco ===
# JARVIS_DAEMON_DB=/opt/jarvis/daemon.db

# === Logs ===
RUST_LOG=jarvis_daemon=info
```

**Segurança**: Proteja o `.env`:
```bash
chmod 600 /opt/jarvis/.env
```

---

## 6. Google OAuth (uma vez)

O daemon precisa de tokens Google para Search Console e AdSense. O fluxo é **interativo** (abrir URL no navegador e colar o código).

### Opção A: Executar na VPS via SSH

```bash
ssh usuario@seu-ip-vps
cd /opt/jarvis

# Carregar .env
export $(grep -v '^#' .env | xargs)

# Executar auth (vai imprimir URL, você abre no browser local, cola o código)
./jarvis-daemon auth google \
  --client-id "$GOOGLE_CLIENT_ID" \
  --client-secret "$GOOGLE_CLIENT_SECRET"
```

O arquivo `~/.jarvis/credentials/google.json` será criado na VPS.

### Opção B: Executar localmente e copiar

Se preferir fazer o OAuth na sua máquina:

```powershell
# Na sua máquina (com jarvis ou jarvis-daemon)
cd E:\projects\ia\jarvis_cli\jarvis-rs
cargo run --bin jarvis-daemon -- auth google --client-id "..." --client-secret "..."

# O arquivo será criado em C:\Users\SeuUsuario\.jarvis\credentials\google.json
# Copie para a VPS:
scp C:\Users\SeuUsuario\.jarvis\credentials\google.json usuario@vps:~/.jarvis/credentials/
```

---

## 7. Configurar pipeline e sources

### 7.1 Criar pipeline SEO Blog

Crie um arquivo `pipeline-seo.json` na VPS:

```json
{
  "id": "seo-blog-1",
  "name": "SEO Blog Principal",
  "strategy": "seo_blog",
  "schedule_cron": "0 3 * * *",
  "llm": {
    "provider": "openrouter",
    "model": "google/gemini-2.0-flash-exp:free",
    "max_tokens": 4096,
    "temperature": 0.7
  },
  "seo": {
    "niche": "Seu nicho aqui",
    "target_audience": "Seu público",
    "language": "pt-BR",
    "tone": "informativo, profissional",
    "min_word_count": 800,
    "max_word_count": 2000
  },
  "publisher": {
    "platform": "wordpress",
    "base_url": "https://seu-blog.com",
    "auth_token_env": "WORDPRESS_APP_PASSWORD"
  }
}
```

Adicione o pipeline (usando o CLI na VPS ou localmente com `JARVIS_DAEMON_DB` apontando para o DB da VPS):

```bash
cd /opt/jarvis
./jarvis daemon pipeline add pipeline-seo.json
```

### 7.2 Adicionar fontes (RSS, sites)

```bash
# Listar pipelines
./jarvis daemon pipeline list

# Adicionar fonte RSS ao pipeline
./jarvis daemon source add seo-blog-1 -t rss --name "Blog X" https://exemplo.com/feed/

# Adicionar fonte Web
./jarvis daemon source add seo-blog-1 -t web --name "Site Y" https://exemplo.gov.br/noticias
```

### 7.3 Habilitar pipelines de métricas e análise

Os pipelines `metrics_collector` e `strategy_analyzer` são registrados automaticamente. Eles rodam conforme o schedule. Verifique com:

```bash
./jarvis daemon pipeline list
./jarvis daemon status
```

---

## 8. Serviço systemd (rodar 24/7)

Crie o arquivo de serviço:

```bash
sudo nano /etc/systemd/system/jarvis-daemon.service
```

Conteúdo:

```ini
[Unit]
Description=Jarvis Daemon - Autonomous content generation
After=network.target

[Service]
Type=simple
User=seu_usuario
Group=seu_usuario
WorkingDirectory=/opt/jarvis

# Carregar variáveis do .env
EnvironmentFile=/opt/jarvis/.env

# Caminho do banco (opcional)
Environment=JARVIS_DAEMON_DB=/opt/jarvis/daemon.db

# Executar o daemon
ExecStart=/opt/jarvis/jarvis-daemon run --tick-interval-sec 60 --max-concurrent 3

# Reiniciar em caso de falha
Restart=on-failure
RestartSec=30

# Logs
StandardOutput=journal
StandardError=journal
SyslogIdentifier=jarvis-daemon

[Install]
WantedBy=multi-user.target
```

Substitua `seu_usuario` pelo usuário Linux que vai rodar o daemon.

Ative e inicie:

```bash
sudo systemctl daemon-reload
sudo systemctl enable jarvis-daemon
sudo systemctl start jarvis-daemon
sudo systemctl status jarvis-daemon
```

### Comandos úteis

```bash
# Ver status
sudo systemctl status jarvis-daemon

# Ver logs em tempo real
sudo journalctl -u jarvis-daemon -f

# Parar
sudo systemctl stop jarvis-daemon

# Reiniciar
sudo systemctl restart jarvis-daemon
```

---

## 9. Verificação

### 9.1 Testar manualmente (antes do systemd)

```bash
cd /opt/jarvis
export $(grep -v '^#' .env | xargs)
./jarvis-daemon run --tick-interval-sec 60 --max-concurrent 3
```

Pressione Ctrl+C para parar. Se rodar sem erros, o systemd deve funcionar.

### 9.2 Verificar via CLI

```bash
./jarvis daemon status
./jarvis daemon pipeline list
./jarvis daemon dashboard
```

### 9.3 Verificar goals (bootstrap automático)

Na primeira execução, o daemon cria goals padrão. Verifique:

```bash
./jarvis daemon goals list
./jarvis daemon goals progress
```

---

## 10. Monitoramento e manutenção

### Logs

```bash
# Logs do systemd
sudo journalctl -u jarvis-daemon -n 100 --no-pager

# Logs em tempo real
sudo journalctl -u jarvis-daemon -f
```

### Backup do banco

```bash
# Backup manual
cp ~/.jarvis/daemon.db ~/backups/daemon-$(date +%Y%m%d).db

# Ou se JARVIS_DAEMON_DB=/opt/jarvis/daemon.db
cp /opt/jarvis/daemon.db /opt/jarvis/backups/daemon-$(date +%Y%m%d).db
```

### Atualizar o daemon

```bash
# Parar o serviço
sudo systemctl stop jarvis-daemon

# Copiar novo binário (ou recompilar)
# scp novo-jarvis-daemon usuario@vps:/opt/jarvis/

# Iniciar
sudo systemctl start jarvis-daemon
```

---

## 11. Resumo do fluxo

```
1. Compilar jarvis-daemon (local ou na VPS)
2. Copiar para /opt/jarvis/
3. Criar .env com credenciais
4. Executar auth google (uma vez)
5. Criar pipeline (pipeline add) e sources
6. Testar manualmente: ./jarvis-daemon run
7. Criar serviço systemd
8. systemctl enable && systemctl start jarvis-daemon
9. Monitorar: jarvis daemon status, journalctl -u jarvis-daemon -f
```

---

## 12. Troubleshooting

### Daemon não inicia
- Verifique `journalctl -u jarvis-daemon -n 50`
- Confirme que o `.env` existe e as variáveis estão corretas
- Confirme que `OPENROUTER_API_KEY` ou `GOOGLE_API_KEY` está definida

### Pipeline não executa
- `jarvis daemon pipeline list` — pipeline está `enabled`?
- `jarvis daemon pipeline config <id>` — config está correta?
- Há sources adicionadas? `jarvis daemon source list <pipeline_id>`

### WordPress falha ao publicar
- Verifique `WORDPRESS_APP_PASSWORD` (Application Password do WordPress)
- Confirme que `base_url` no pipeline está correto (ex: `https://seu-blog.com`)

### Google APIs falham
- Execute `jarvis-daemon auth google` novamente
- Confirme que `~/.jarvis/credentials/google.json` existe na VPS

---

## 13. Alternativa: Deploy com Docker

Se preferir ambiente containerizado, use o Docker:

```bash
cd jarvis_cli
docker compose -f docker-compose.vps.yml up -d
```

Veja [DAEMON_DOCKER.md](DAEMON_DOCKER.md) para o guia completo.

---

## 14. Documentação relacionada

- [DAEMON_DOCKER.md](DAEMON_DOCKER.md) — Deploy com Docker
- [daemon-cli-management.md](features/daemon-cli-management.md) — Todos os comandos do CLI
- [daemon-automation.md](features/daemon-automation.md) — Especificação do daemon
- [ANALISE_AUTONOMIA_JARVIS.md](ANALISE_AUTONOMIA_JARVIS.md) — Status e próximos passos

---

**Última atualização**: 2026-02-18
