# Jarvis Daemon — Deploy com Docker

**Objetivo**: Rodar o jarvis-daemon em container Docker, isolado e fácil de publicar na VPS.

---

## 1. Pré-requisitos

- Docker e Docker Compose instalados
- Arquivo `.env` com credenciais (veja seção 2)

---

## 2. Configurar variáveis de ambiente

Crie `.env` na raiz do projeto (`jarvis_cli/`):

```bash
# Copie o exemplo
cp jarvis-rs/daemon/env.example .env

# Edite com suas credenciais
nano .env  # ou use seu editor
```

Variáveis essenciais:

| Variável | Obrigatório | Descrição |
|----------|-------------|-----------|
| `OPENROUTER_API_KEY` ou `GOOGLE_API_KEY` | Sim | Para LLM nos pipelines |
| `WORDPRESS_APP_PASSWORD` | Se usar WordPress | Application Password |
| `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` | Para Search Console/AdSense | OAuth |
| `JARVIS_TELEGRAM_BOT_TOKEN` / `JARVIS_TELEGRAM_CHAT_ID` | Opcional | Notificações |

---

## 3. Build e execução

### Opção A: Standalone (apenas daemon)

```bash
cd jarvis_cli

# Build
docker compose -f docker-compose.daemon.yml build

# Subir
docker compose -f docker-compose.daemon.yml up -d

# Logs
docker compose -f docker-compose.daemon.yml logs -f jarvis-daemon
```

### Opção B: Com serviços VPS (Qdrant, Ollama, Redis, Postgres)

```bash
cd jarvis_cli

# Build e subir tudo
docker compose -f docker-compose.vps.yml build
docker compose -f docker-compose.vps.yml up -d

# Logs do daemon
docker compose -f docker-compose.vps.yml logs -f jarvis-daemon
```

### Opção C: Build manual da imagem

```bash
cd jarvis_cli/jarvis-rs

# Build da imagem
docker build -t jarvis-daemon:latest -f daemon/Dockerfile .

# Run
docker run -d \
  --name jarvis-daemon \
  --env-file ../.env \
  -v jarvis-daemon-data:/home/jarvis/.jarvis \
  jarvis-daemon:latest
```

---

## 4. Credenciais Google (OAuth)

O daemon precisa de `~/.jarvis/credentials/google.json` para Search Console e AdSense.

### Passo 1: Gerar o arquivo (uma vez)

Na sua máquina ou na VPS (com display/browser):

```bash
# Com o binário local
jarvis-daemon auth google \
  --client-id "$GOOGLE_CLIENT_ID" \
  --client-secret "$GOOGLE_CLIENT_SECRET"
```

O arquivo será criado em `~/.jarvis/credentials/google.json`.

### Passo 2: Montar no container

Crie a pasta e copie o arquivo:

```bash
mkdir -p credentials
cp ~/.jarvis/credentials/google.json credentials/

# No docker-compose, descomente o volume:
# volumes:
#   - ./credentials:/home/jarvis/.jarvis/credentials:ro
```

Ou monte o volume manualmente:

```bash
docker run -d \
  --name jarvis-daemon \
  --env-file .env \
  -v jarvis-daemon-data:/home/jarvis/.jarvis \
  -v $(pwd)/credentials:/home/jarvis/.jarvis/credentials:ro \
  jarvis-daemon:latest
```

---

## 5. Configurar pipeline e sources

O banco SQLite fica no volume `jarvis_daemon_data`. Para usar o CLI e criar pipelines, você precisa acessar esse banco.

### Opção A: Executar CLI dentro do container

```bash
# Entrar no container
docker exec -it jarvis-daemon sh

# O daemon não inclui o CLI; use o binário local com o DB do container
# (veja Opção B)
```

### Opção B: Montar o volume no host e usar CLI local

Pare o container, monte o volume em um caminho conhecido e use o `jarvis` CLI local:

```bash
# Descobrir onde o volume está
docker volume inspect jarvis_cli_jarvis_daemon_data

# Ou use um bind mount em vez de volume nomeado no compose:
# volumes:
#   - ./data/jarvis:/home/jarvis/.jarvis
```

Depois, com o `jarvis` CLI instalado localmente:

```bash
export JARVIS_DAEMON_DB=./data/jarvis/daemon.db
jarvis daemon pipeline add pipeline-seo.json
jarvis daemon source add seo-blog-1 -t rss --name "Blog X" https://exemplo.com/feed/
```

### Opção C: Container auxiliar com CLI

Crie um Dockerfile que inclua o CLI e monte o mesmo volume. Ou use um container temporário:

```bash
# Compilar jarvis CLI e copiar para um container que monte o volume
# (avançado - requer imagem customizada)
```

**Recomendação**: Use bind mount `./data/jarvis:/home/jarvis/.jarvis` no compose para facilitar o acesso ao DB com o CLI local.

---

## 6. Estrutura de arquivos

```
jarvis_cli/
├── .env                          # Credenciais (não commitar)
├── credentials/
│   └── google.json               # OAuth (após auth google)
├── data/
│   └── jarvis/                   # Bind mount opcional para CLI
│       └── daemon.db
├── docker-compose.daemon.yml     # Standalone
├── docker-compose.vps.yml        # Daemon + Qdrant, Ollama, etc.
└── jarvis-rs/
    ├── .dockerignore
    └── daemon/
        ├── Dockerfile
        ├── env.example
        └── examples/
            └── pipeline-google-gemini.json
```

---

## 7. Comandos úteis

```bash
# Status
docker compose -f docker-compose.vps.yml ps

# Logs em tempo real
docker compose -f docker-compose.vps.yml logs -f jarvis-daemon

# Parar
docker compose -f docker-compose.vps.yml stop jarvis-daemon

# Reiniciar
docker compose -f docker-compose.vps.yml restart jarvis-daemon

# Rebuild após mudanças no código
docker compose -f docker-compose.vps.yml build jarvis-daemon --no-cache
docker compose -f docker-compose.vps.yml up -d jarvis-daemon
```

---

## 8. Troubleshooting

### Container sai logo após iniciar
- Verifique os logs: `docker compose logs jarvis-daemon`
- Confirme que `OPENROUTER_API_KEY` ou `GOOGLE_API_KEY` está no `.env`

### Pipeline não executa
- O daemon precisa de pipelines e sources no banco
- Use bind mount para o volume e configure com `jarvis daemon pipeline add` localmente

### Erro de permissão no volume
- O daemon roda como usuário `jarvis` (UID 1000)
- Se usar bind mount, ajuste: `chown -R 1000:1000 ./data/jarvis`

---

## 9. Referências

- [DAEMON_DEPLOY_VPS.md](DAEMON_DEPLOY_VPS.md) — Deploy sem Docker (systemd)
- [daemon-cli-management.md](features/daemon-cli-management.md) — Comandos do CLI

---

**Última atualização**: 2026-02-18
