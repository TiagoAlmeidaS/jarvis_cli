# =============================================================================
# Jarvis CLI Environment Variables
# Adicione este conteúdo ao seu ~/.bashrc ou ~/.bash_profile
# =============================================================================

# Google OAuth
export GOOGLE_CLIENT_ID="765554684645-geg8l26m1vkukn792bfdgtm0urhe905v.apps.googleusercontent.com"

# Databricks
export DATABRICKS_API_KEY="your_databricks_api_key_here"
export DATABRICKS_BASE_URL="https://adb-926216925051160.0.azuredatabricks.net"

# OpenAI
export OPENAI_API_KEY="your_openai_api_key_here"

# OpenRouter
export OPENROUTER_API_KEY="your_openrouter_api_key_here"

# Jarvis CLI Aliases (opcional)
alias jarvis='E:/projects/ia/jarvis_cli/jarvis-rs/target/debug/jarvis.exe'
alias jarvis-build='cd E:/projects/ia/jarvis_cli/jarvis-rs && cargo build --package jarvis-cli'
alias jarvis-test='cd E:/projects/ia/jarvis_cli/jarvis-rs && cargo test --package jarvis-core'
