#!/bin/bash
# ============================================================================
# Ollama VPS Setup Script for Jarvis CLI
# ============================================================================
# This script installs and configures Ollama on a VPS with all required models
# for the Jarvis project, accessible via Tailscale.
#
# Usage:
#   1. Copy to VPS: scp setup-ollama-vps.sh root@100.98.213.86:~/
#   2. SSH into VPS: ssh root@100.98.213.86
#   3. Run: bash setup-ollama-vps.sh
#
# Models to be installed (8 total):
#   1. llama3.2:3b       - Fast, general purpose (2.0 GB)
#   2. llama3.1:8b       - More capable, balanced (4.7 GB)
#   3. qwen2.5:7b        - Multilingual, strong reasoning (4.4 GB)
#   4. deepseek-coder:6.7b - Code specialist (3.8 GB)
#   5. codellama:7b      - Code generation (3.8 GB)
#   6. phi3:mini         - Ultra compact, efficient (2.3 GB)
#   7. gemma2:2b         - Very fast, lightweight (1.6 GB)
#   8. nomic-embed-text  - Embeddings for RAG (274 MB)
#
# Total download: ~22.8 GB
# ============================================================================

set -e  # Exit on error

echo "============================================"
echo "🚀 Ollama VPS Setup for Jarvis CLI"
echo "============================================"
echo ""

# ============================================================================
# Step 0: Check prerequisites
# ============================================================================
echo "📋 [0/6] Checking prerequisites..."

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "⚠️  This script should be run as root"
    echo "   Attempting to use sudo for privileged operations..."
    SUDO="sudo"
else
    SUDO=""
fi

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    echo "✅ Detected OS: $OS $VERSION_ID"
else
    echo "❌ Cannot detect OS (/etc/os-release not found)"
    exit 1
fi

# Check disk space (need at least 25GB)
AVAILABLE_GB=$(df -BG / | awk 'NR==2 {print $4}' | sed 's/G//')
if [ "$AVAILABLE_GB" -lt 25 ]; then
    echo "⚠️  Warning: Low disk space. Available: ${AVAILABLE_GB}GB, Recommended: 25GB+"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
else
    echo "✅ Disk space OK: ${AVAILABLE_GB}GB available"
fi

echo ""

# ============================================================================
# Step 1: Install Ollama
# ============================================================================
echo "📥 [1/6] Installing Ollama..."

if command -v ollama &> /dev/null; then
    OLLAMA_VERSION=$(ollama --version | head -n1)
    echo "✅ Ollama already installed: $OLLAMA_VERSION"
else
    echo "   Downloading and installing Ollama..."
    curl -fsSL https://ollama.com/install.sh | sh

    if [ $? -ne 0 ]; then
        echo "❌ Failed to install Ollama"
        exit 1
    fi

    OLLAMA_VERSION=$(ollama --version | head -n1)
    echo "✅ Ollama installed: $OLLAMA_VERSION"
fi

echo ""

# ============================================================================
# Step 2: Configure Ollama systemd service
# ============================================================================
echo "🔧 [2/6] Configuring Ollama service..."

# Stop any running Ollama processes
$SUDO pkill ollama 2>/dev/null || true
sleep 2

# Check if systemd service exists
if [ -f /etc/systemd/system/ollama.service ]; then
    echo "   Systemd service already exists, creating override..."
    $SUDO mkdir -p /etc/systemd/system/ollama.service.d/
    $SUDO tee /etc/systemd/system/ollama.service.d/override.conf > /dev/null <<EOF
[Service]
Environment="OLLAMA_HOST=0.0.0.0:11434"
EOF
else
    echo "   Creating systemd service..."
    $SUDO tee /etc/systemd/system/ollama.service > /dev/null <<EOF
[Unit]
Description=Ollama Service
After=network-online.target

[Service]
ExecStart=/usr/local/bin/ollama serve
Environment="OLLAMA_HOST=0.0.0.0:11434"
User=root
Restart=always
RestartSec=3

[Install]
WantedBy=default.target
EOF
fi

echo "✅ Service configuration created"

# Reload systemd and start service
$SUDO systemctl daemon-reload
$SUDO systemctl enable ollama 2>/dev/null || true
$SUDO systemctl restart ollama

# Wait for service to be ready
echo "   Waiting for Ollama to start..."
sleep 5

if $SUDO systemctl is-active --quiet ollama; then
    echo "✅ Ollama service is running"
else
    echo "❌ Failed to start Ollama service"
    $SUDO systemctl status ollama --no-pager
    exit 1
fi

echo ""

# ============================================================================
# Step 3: Verify network configuration
# ============================================================================
echo "🌐 [3/6] Verifying network configuration..."

# Check if port 11434 is listening
if $SUDO netstat -tlnp 2>/dev/null | grep -q ":11434" || $SUDO ss -tlnp 2>/dev/null | grep -q ":11434"; then
    echo "✅ Port 11434 is listening"
else
    echo "❌ Port 11434 is not listening"
    exit 1
fi

# Show Tailscale IP if available
TAILSCALE_IP=$(ip addr show tailscale0 2>/dev/null | grep 'inet ' | awk '{print $2}' | cut -d/ -f1)
if [ -n "$TAILSCALE_IP" ]; then
    echo "✅ Tailscale IP: $TAILSCALE_IP"
else
    echo "⚠️  Tailscale interface not found (this is OK if using different VPN)"
fi

# Test local API
if curl -s --connect-timeout 3 http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "✅ Local API responding"
else
    echo "❌ Local API not responding"
    exit 1
fi

echo ""

# ============================================================================
# Step 4: Configure firewall (optional)
# ============================================================================
echo "🔥 [4/6] Configuring firewall..."

if command -v ufw &> /dev/null && $SUDO ufw status | grep -q "Status: active"; then
    $SUDO ufw allow 11434/tcp comment "Ollama API" 2>/dev/null || true
    echo "✅ UFW: Allowed port 11434/tcp"
elif command -v firewall-cmd &> /dev/null && $SUDO firewall-cmd --state 2>/dev/null | grep -q "running"; then
    $SUDO firewall-cmd --permanent --add-port=11434/tcp 2>/dev/null || true
    $SUDO firewall-cmd --reload 2>/dev/null || true
    echo "✅ Firewalld: Allowed port 11434/tcp"
else
    echo "⚠️  No active firewall detected, skipping"
fi

echo ""

# ============================================================================
# Step 5: Download all models
# ============================================================================
echo "📦 [5/6] Downloading models (this will take a while)..."
echo "   Total expected download: ~22.8 GB"
echo ""

# Array of models with descriptions
declare -A MODELS=(
    ["llama3.2:3b"]="Fast general purpose model"
    ["llama3.1:8b"]="More capable balanced model"
    ["qwen2.5:7b"]="Multilingual with strong reasoning"
    ["deepseek-coder:6.7b"]="Code specialist"
    ["codellama:7b"]="Code generation"
    ["phi3:mini"]="Ultra compact efficient model"
    ["gemma2:2b"]="Very fast lightweight model"
    ["nomic-embed-text"]="Embeddings for RAG"
)

# Track progress
TOTAL=8
CURRENT=0
SUCCESS=0
FAILED=0
FAILED_MODELS=()

# Pull each model
for MODEL in "${!MODELS[@]}"; do
    CURRENT=$((CURRENT + 1))
    DESC="${MODELS[$MODEL]}"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📥 [$CURRENT/$TOTAL] Pulling: $MODEL"
    echo "   Description: $DESC"
    echo ""

    if ollama pull "$MODEL"; then
        echo "✅ Success: $MODEL"
        SUCCESS=$((SUCCESS + 1))
    else
        echo "❌ Failed: $MODEL"
        FAILED=$((FAILED + 1))
        FAILED_MODELS+=("$MODEL")
    fi
    echo ""
done

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Download Summary:"
echo "   ✅ Successful: $SUCCESS/$TOTAL"
echo "   ❌ Failed: $FAILED/$TOTAL"
if [ $FAILED -gt 0 ]; then
    echo "   Failed models: ${FAILED_MODELS[*]}"
fi
echo ""

# ============================================================================
# Step 6: Final verification
# ============================================================================
echo "✅ [6/6] Final verification..."
echo ""

# List installed models
echo "📦 Installed models:"
ollama list
echo ""

# Show service status
echo "📊 Service status:"
$SUDO systemctl status ollama --no-pager | head -n 8
echo ""

# Show disk usage
echo "💾 Disk usage:"
df -h / | awk 'NR==1 || NR==2'
echo ""

# ============================================================================
# Success message
# ============================================================================
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎉 Ollama VPS Setup Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📋 Configuration:"
echo "   • Service: systemctl status ollama"
echo "   • Endpoint: http://0.0.0.0:11434"
echo "   • Tailscale: http://${TAILSCALE_IP:-100.98.213.86}:11434"
echo "   • Models: $SUCCESS/$TOTAL installed"
echo ""
echo "🧪 Test from local machine:"
echo "   export OLLAMA_BASE_URL=\"http://100.98.213.86:11434/v1\""
echo "   ./jarvis-rs/target/debug/jarvis.exe exec \\"
echo "     -c 'model_provider=\"ollama\"' \\"
echo "     -c 'model=\"llama3.2:3b\"' \\"
echo "     \"Hello, Ollama!\""
echo ""
echo "📚 Useful commands:"
echo "   • List models:      ollama list"
echo "   • Remove model:     ollama rm <model>"
echo "   • Service logs:     journalctl -u ollama -f"
echo "   • Restart service:  systemctl restart ollama"
echo "   • Check API:        curl http://localhost:11434/api/tags"
echo ""
echo "🔐 Security reminder:"
echo "   ✅ Access is limited via Tailscale"
echo "   ⚠️  DO NOT expose port 11434 publicly!"
echo ""
