# Dockerfile para Jarvis Web API
# Otimizado para Railway.app

FROM rust:1.75-slim as builder

# Instalar dependências de build
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Criar diretório de trabalho
WORKDIR /app

# Copiar workspace completo (necessário para build correto)
COPY . .

# Build em release
RUN cd jarvis-rs && cargo build --package jarvis-web-api --release

# Imagem final minimalista
FROM debian:bookworm-slim

# Instalar apenas dependências runtime essenciais
RUN apt-get update && apt-get install -y \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiar binário
COPY --from=builder /app/jarvis-rs/target/release/jarvis-web-api /app/jarvis-web-api

# Copiar arquivos estáticos
COPY --from=builder /app/jarvis-rs/web-api/static ./static

# Criar diretório para jarvis_home
RUN mkdir -p /app/.jarvis

# Variáveis de ambiente padrão
ENV RUST_LOG=info
ENV JARVIS_HOME=/app/.jarvis

# PORT será definido pelo Railway automaticamente
# Expor porta padrão (Railway sobrescreve)
EXPOSE 3000

# Comando de execução
CMD ["/app/jarvis-web-api"]
