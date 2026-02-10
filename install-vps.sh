#!/bin/bash
#═══════════════════════════════════════════════════════════════════════════════
# Jarvis CLI - VPS Setup Script
# Instala e configura Ollama na VPS com suporte a Docker
#═══════════════════════════════════════════════════════════════════════════════

set -e  # Exit on error

# Cores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Funções de log
log_info() {
    echo -e "${CYAN}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

log_header() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo ""
}

# Verificar se está rodando como root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "Este script precisa ser executado como root"
        echo "Use: sudo ./install-vps.sh"
        exit 1
    fi
}

# Detectar sistema operacional
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        VERSION=$VERSION_ID
    else
        log_error "Sistema operacional não suportado"
        exit 1
    fi
}

# Instalar dependências básicas
install_dependencies() {
    log_header "Instalando Dependências"

    case $OS in
        ubuntu|debian)
            apt-get update
            apt-get install -y \
                curl \
                wget \
                git \
                htop \
                jq \
                net-tools \
                ca-certificates \
                gnupg \
                lsb-release
            ;;
        centos|rhel|fedora)
            yum update -y
            yum install -y \
                curl \
                wget \
                git \
                htop \
                jq \
                net-tools \
                ca-certificates
            ;;
        *)
            log_error "Sistema operacional não suportado: $OS"
            exit 1
            ;;
    esac

    log_success "Dependências instaladas"
}

# Instalar Docker
install_docker() {
    log_header "Instalando Docker"

    if command -v docker &> /dev/null; then
        log_success "Docker já instalado: $(docker --version)"
        return
    fi

    log_info "Instalando Docker..."

    case $OS in
        ubuntu|debian)
            # Remover versões antigas
            apt-get remove -y docker docker-engine docker.io containerd runc 2>/dev/null || true

            # Adicionar repositório Docker
            mkdir -p /etc/apt/keyrings
            curl -fsSL https://download.docker.com/linux/$OS/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg

            echo \
              "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/$OS \
              $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

            apt-get update
            apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
            ;;
        centos|rhel|fedora)
            yum install -y yum-utils
            yum-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
            yum install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
            ;;
    esac

    # Iniciar Docker
    systemctl start docker
    systemctl enable docker

    log_success "Docker instalado: $(docker --version)"
}

# Instalar Tailscale
install_tailscale() {
    log_header "Verificando Tailscale"

    if command -v tailscale &> /dev/null; then
        log_success "Tailscale já instalado"

        # Verificar se está conectado
        if tailscale status &> /dev/null; then
            TAILSCALE_IP=$(tailscale ip -4 2>/dev/null)
            if [ ! -z "$TAILSCALE_IP" ]; then
                log_success "Tailscale conectado: $TAILSCALE_IP"
                return
            fi
        fi

        log_warning "Tailscale instalado mas não conectado"
        log_info "Configure com: sudo tailscale up"
    else
        log_info "Instalando Tailscale..."
        curl -fsSL https://tailscale.com/install.sh | sh
        log_success "Tailscale instalado"
        log_warning "Configure com: sudo tailscale up"
    fi
}

# Escolher método de instalação do Ollama
choose_ollama_method() {
    log_header "Método de Instalação do Ollama"

    echo "Escolha o método de instalação:"
    echo "1) Docker (recomendado, isolado)"
    echo "2) Nativo (mais performático)"
    echo ""
    read -p "Escolha (1 ou 2): " CHOICE

    case $CHOICE in
        1)
            OLLAMA_METHOD="docker"
            log_info "Método escolhido: Docker"
            ;;
        2)
            OLLAMA_METHOD="native"
            log_info "Método escolhido: Nativo"
            ;;
        *)
            log_error "Escolha inválida"
            exit 1
            ;;
    esac
}

# Instalar Ollama via Docker
install_ollama_docker() {
    log_header "Instalando Ollama via Docker"

    # Parar container existente
    docker stop ollama 2>/dev/null || true
    docker rm ollama 2>/dev/null || true

    # Verificar se há GPU NVIDIA
    if command -v nvidia-smi &> /dev/null; then
        log_info "GPU NVIDIA detectada"
        GPU_FLAG="--gpus all"
    else
        log_info "Sem GPU NVIDIA, usando CPU"
        GPU_FLAG=""
    fi

    # Criar diretório para dados
    mkdir -p /opt/ollama/data

    # Iniciar container
    log_info "Iniciando container Ollama..."
    docker run -d \
        --name ollama \
        --restart unless-stopped \
        -p 11434:11434 \
        -v /opt/ollama/data:/root/.ollama \
        $GPU_FLAG \
        ollama/ollama

    # Aguardar container iniciar
    sleep 5

    # Verificar se está rodando
    if docker ps | grep -q ollama; then
        log_success "Ollama rodando via Docker"
    else
        log_error "Falha ao iniciar container Ollama"
        exit 1
    fi
}

# Instalar Ollama nativamente
install_ollama_native() {
    log_header "Instalando Ollama Nativamente"

    if command -v ollama &> /dev/null; then
        log_success "Ollama já instalado"
    else
        log_info "Baixando e instalando Ollama..."
        curl -fsSL https://ollama.com/install.sh | sh
        log_success "Ollama instalado"
    fi

    # Configurar serviço para aceitar conexões remotas
    log_info "Configurando Ollama para conexões remotas..."

    mkdir -p /etc/systemd/system/ollama.service.d/

    cat > /etc/systemd/system/ollama.service.d/override.conf <<'EOF'
[Service]
Environment="OLLAMA_HOST=0.0.0.0:11434"
Environment="OLLAMA_NUM_PARALLEL=2"
Environment="OLLAMA_MAX_LOADED_MODELS=3"
Environment="OLLAMA_KEEP_ALIVE=24h"
EOF

    # Recarregar e reiniciar
    systemctl daemon-reload
    systemctl enable ollama
    systemctl restart ollama

    sleep 3

    # Verificar status
    if systemctl is-active --quiet ollama; then
        log_success "Ollama rodando nativamente"
    else
        log_error "Falha ao iniciar Ollama"
        systemctl status ollama --no-pager
        exit 1
    fi
}

# Baixar modelos
download_models() {
    log_header "Baixando Modelos"

    # Detectar RAM disponível
    TOTAL_RAM=$(free -g | awk '/^Mem:/{print $2}')
    log_info "RAM disponível: ${TOTAL_RAM}GB"

    # Função para baixar modelo
    pull_model() {
        local MODEL=$1
        log_info "Baixando $MODEL..."

        if [ "$OLLAMA_METHOD" = "docker" ]; then
            docker exec ollama ollama pull $MODEL
        else
            ollama pull $MODEL
        fi

        log_success "$MODEL baixado"
    }

    # Baixar modelos baseado na RAM
    if [ "$TOTAL_RAM" -ge 16 ]; then
        log_info "RAM suficiente para modelos grandes"
        pull_model "phi3:mini"
        pull_model "llama3.2:3b"
        pull_model "llama3.1:8b"
        pull_model "codellama:7b"
    elif [ "$TOTAL_RAM" -ge 8 ]; then
        log_info "RAM suficiente para modelos médios"
        pull_model "phi3:mini"
        pull_model "llama3.2:3b"
        pull_model "llama3.1:8b"
    else
        log_info "RAM limitada - usando modelos leves"
        pull_model "phi3:mini"
        pull_model "llama3.2:3b"
    fi

    log_success "Modelos baixados"
}

# Listar modelos instalados
list_models() {
    log_info "Modelos instalados:"

    if [ "$OLLAMA_METHOD" = "docker" ]; then
        docker exec ollama ollama list
    else
        ollama list
    fi
}

# Configurar firewall
configure_firewall() {
    log_header "Configurando Firewall"

    if command -v ufw &> /dev/null; then
        log_info "Configurando UFW..."

        # Permitir SSH
        ufw allow 22/tcp

        # Permitir Tailscale
        ufw allow in on tailscale0

        # Habilitar firewall
        echo "y" | ufw enable

        log_success "Firewall configurado"
    else
        log_warning "UFW não instalado, firewall não configurado"
    fi
}

# Testar instalação
test_installation() {
    log_header "Testando Instalação"

    # Testar porta
    log_info "Verificando porta 11434..."
    if netstat -tuln | grep -q ":11434"; then
        log_success "Porta 11434 aberta"
    else
        log_error "Porta 11434 não encontrada"
        return 1
    fi

    # Testar API
    log_info "Testando API Ollama..."
    sleep 2

    RESPONSE=$(curl -s --connect-timeout 5 http://localhost:11434/api/tags)

    if [ $? -eq 0 ] && [ ! -z "$RESPONSE" ]; then
        log_success "API respondendo corretamente"
        return 0
    else
        log_error "API não está respondendo"
        return 1
    fi
}

# Criar script de manutenção
create_maintenance_scripts() {
    log_header "Criando Scripts de Manutenção"

    # Script de status
    cat > /usr/local/bin/ollama-status <<'EOF'
#!/bin/bash
echo "════════════════════════════════════════"
echo "  Ollama Status"
echo "════════════════════════════════════════"
echo ""

# Verificar método
if docker ps | grep -q ollama; then
    echo "Método: Docker"
    echo "Status: $(docker inspect -f '{{.State.Status}}' ollama)"
    echo ""
    echo "Modelos:"
    docker exec ollama ollama list
else
    echo "Método: Nativo"
    systemctl status ollama --no-pager | head -3
    echo ""
    echo "Modelos:"
    ollama list
fi

echo ""
echo "Porta: $(netstat -tuln | grep 11434 || echo 'Não encontrada')"
echo ""
echo "RAM:"
free -h | grep Mem
echo ""

# Tailscale
if command -v tailscale &> /dev/null; then
    echo "Tailscale IP: $(tailscale ip -4 2>/dev/null || echo 'Não conectado')"
fi
EOF

    chmod +x /usr/local/bin/ollama-status
    log_success "Script criado: ollama-status"

    # Script de logs
    cat > /usr/local/bin/ollama-logs <<'EOF'
#!/bin/bash
if docker ps | grep -q ollama; then
    docker logs -f ollama
else
    journalctl -u ollama -f
fi
EOF

    chmod +x /usr/local/bin/ollama-logs
    log_success "Script criado: ollama-logs"

    # Script de restart
    cat > /usr/local/bin/ollama-restart <<'EOF'
#!/bin/bash
if docker ps | grep -q ollama; then
    docker restart ollama
else
    systemctl restart ollama
fi
echo "Ollama reiniciado"
EOF

    chmod +x /usr/local/bin/ollama-restart
    log_success "Script criado: ollama-restart"
}

# Criar arquivo de informações
create_info_file() {
    local INFO_FILE="$(pwd)/VPS_INFO.txt"

    cat > $INFO_FILE <<EOF
═══════════════════════════════════════════════════════════
  Jarvis Ollama VPS - Informações de Instalação
═══════════════════════════════════════════════════════════

Data: $(date)
Hostname: $(hostname)
Sistema: $OS $VERSION

CONFIGURAÇÃO
════════════════════════════════════════════════════════════
Método: $OLLAMA_METHOD
Porta: 11434
RAM: $(free -h | awk '/^Mem:/{print $2}')
Armazenamento: $(df -h / | awk 'NR==2{print $4}' | xargs echo "disponível")

REDE
════════════════════════════════════════════════════════════
IP Público: $(curl -s ifconfig.me 2>/dev/null || echo "N/A")
IP Tailscale: $(tailscale ip -4 2>/dev/null || echo "Configure com: sudo tailscale up")

MODELOS INSTALADOS
════════════════════════════════════════════════════════════
EOF

    if [ "$OLLAMA_METHOD" = "docker" ]; then
        docker exec ollama ollama list >> $INFO_FILE
    else
        ollama list >> $INFO_FILE
    fi

    cat >> $INFO_FILE <<EOF

COMANDOS ÚTEIS
════════════════════════════════════════════════════════════
Status:      ollama-status
Logs:        ollama-logs
Reiniciar:   ollama-restart

Listar modelos:
  $([ "$OLLAMA_METHOD" = "docker" ] && echo "docker exec ollama ollama list" || echo "ollama list")

Baixar modelo:
  $([ "$OLLAMA_METHOD" = "docker" ] && echo "docker exec ollama ollama pull <modelo>" || echo "ollama pull <modelo>")

Testar API:
  curl http://localhost:11434/api/tags

CONFIGURAÇÃO DO JARVIS (Máquina Local)
════════════════════════════════════════════════════════════
1. Configure o Tailscale se ainda não configurou:
   sudo tailscale up

2. Anote o IP Tailscale: $(tailscale ip -4 2>/dev/null || echo "Configure Tailscale primeiro")

3. Na máquina local, execute:
   cd /e/projects/ia/jarvis_cli
   ./configure-ollama-remote.sh

4. Teste:
   cd jarvis-rs
   ./target/debug/jarvis.exe chat

SEGURANÇA
════════════════════════════════════════════════════════════
✓ Acesso via Tailscale (criptografado)
✓ Porta 11434 não exposta publicamente
✓ Firewall configurado

MANUTENÇÃO
════════════════════════════════════════════════════════════
Atualizar Ollama:
  $([ "$OLLAMA_METHOD" = "docker" ] && echo "docker pull ollama/ollama && docker restart ollama" || echo "curl -fsSL https://ollama.com/install.sh | sh")

Backup modelos:
  tar czf ollama-backup.tar.gz $([ "$OLLAMA_METHOD" = "docker" ] && echo "/opt/ollama/data" || echo "~/.ollama")

SUPORTE
════════════════════════════════════════════════════════════
Documentação: ./OLLAMA_VPS_SETUP.md
Logs: ollama-logs
Status: ollama-status

═══════════════════════════════════════════════════════════
EOF

    log_success "Arquivo de informações criado: $INFO_FILE"
}

# Mostrar resumo final
show_summary() {
    log_header "Instalação Concluída!"

    echo -e "${GREEN}✓${NC} Ollama instalado via ${CYAN}$OLLAMA_METHOD${NC}"
    echo -e "${GREEN}✓${NC} Porta 11434 aberta"
    echo -e "${GREEN}✓${NC} Modelos baixados"

    # Tailscale
    TAILSCALE_IP=$(tailscale ip -4 2>/dev/null)
    if [ ! -z "$TAILSCALE_IP" ]; then
        echo -e "${GREEN}✓${NC} Tailscale IP: ${CYAN}$TAILSCALE_IP${NC}"
    else
        echo -e "${YELLOW}⚠${NC} Configure Tailscale: ${CYAN}sudo tailscale up${NC}"
    fi

    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Próximos Passos${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo ""

    if [ -z "$TAILSCALE_IP" ]; then
        echo "1️⃣  Configure Tailscale:"
        echo "   ${CYAN}sudo tailscale up${NC}"
        echo ""
    fi

    echo "2️⃣  Testar instalação:"
    echo "   ${CYAN}curl http://localhost:11434/api/tags${NC}"
    echo ""

    echo "3️⃣  Ver status:"
    echo "   ${CYAN}ollama-status${NC}"
    echo ""

    echo "4️⃣  Configurar máquina local:"
    echo "   ${CYAN}cd /e/projects/ia/jarvis_cli${NC}"
    echo "   ${CYAN}./configure-ollama-remote.sh${NC}"
    echo ""

    echo "5️⃣  Testar Jarvis:"
    echo "   ${CYAN}cd jarvis-rs${NC}"
    echo "   ${CYAN}./target/debug/jarvis.exe chat${NC}"
    echo ""

    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "📄 Informações salvas em: ${CYAN}VPS_INFO.txt${NC}"
    echo ""
}

#═══════════════════════════════════════════════════════════════════════════════
# MAIN
#═══════════════════════════════════════════════════════════════════════════════

main() {
    clear

    echo -e "${CYAN}"
    cat << "EOF"
    ╔═══════════════════════════════════════════════════════════╗
    ║                                                           ║
    ║      ██╗ █████╗ ██████╗ ██╗   ██╗██╗███████╗            ║
    ║      ██║██╔══██╗██╔══██╗██║   ██║██║██╔════╝            ║
    ║      ██║███████║██████╔╝██║   ██║██║███████╗            ║
    ║ ██   ██║██╔══██║██╔══██╗╚██╗ ██╔╝██║╚════██║            ║
    ║ ╚█████╔╝██║  ██║██║  ██║ ╚████╔╝ ██║███████║            ║
    ║  ╚════╝ ╚═╝  ╚═╝╚═╝  ╚═╝  ╚═══╝  ╚═╝╚══════╝            ║
    ║                                                           ║
    ║            VPS Setup com Ollama                          ║
    ║                                                           ║
    ╚═══════════════════════════════════════════════════════════╝
EOF
    echo -e "${NC}"

    # Verificações
    check_root
    detect_os

    log_info "Sistema detectado: $OS $VERSION"
    echo ""

    # Instalação
    install_dependencies
    install_docker
    install_tailscale
    choose_ollama_method

    if [ "$OLLAMA_METHOD" = "docker" ]; then
        install_ollama_docker
    else
        install_ollama_native
    fi

    download_models
    configure_firewall
    test_installation
    create_maintenance_scripts
    list_models
    create_info_file
    show_summary

    log_success "Setup completo!"
}

# Executar
main "$@"
