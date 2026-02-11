@echo off
REM Script de teste para validar integração RAG (Windows)

echo.
echo 🧪 Testing RAG Integration in Jarvis CLI
echo ==========================================
echo.

set JARVIS_BIN=jarvis-rs\target\release\jarvis.exe

REM Check if binary exists
if not exist "%JARVIS_BIN%" (
    echo ❌ Jarvis binary not found. Building...
    cd jarvis-rs
    cargo build --release --features qdrant,postgres
    cd ..
)

echo ✅ Jarvis binary found
echo.

REM Step 1: Check current context
echo 📝 Step 1: Checking current context...
%JARVIS_BIN% context stats

echo.
echo 📝 Step 2: Adding test documents to context...

REM Add README as a test document
if exist "README.md" (
    echo    Adding README.md...
    %JARVIS_BIN% context add README.md --tags project,docs
) else (
    echo    ⚠️  README.md not found, skipping
)

REM Add RAG documentation
if exist "docs\rag-integration-guide.md" (
    echo    Adding rag-integration-guide.md...
    %JARVIS_BIN% context add docs\rag-integration-guide.md --tags rag,docs
) else (
    echo    ⚠️  rag-integration-guide.md not found, skipping
)

REM Add some Rust code
if exist "jarvis-rs\core\src\rag\mod.rs" (
    echo    Adding RAG module...
    %JARVIS_BIN% context add jarvis-rs\core\src\rag\mod.rs --tags rust,rag,code
) else (
    echo    ⚠️  RAG module not found, skipping
)

echo.
echo ✅ Documents added to context
echo.

REM Step 3: Check stats again
echo 📊 Step 3: Checking updated context stats...
%JARVIS_BIN% context stats

echo.
echo 🔍 Step 4: Testing semantic search...
%JARVIS_BIN% context search "How does RAG work?" -n 3 --min-score 0.5

echo.
echo 🤖 Step 5: Testing RAG-enhanced chat (with context injection)...
echo.
echo Query: 'What is this project about?'
echo Expected: Should use context from indexed documents
echo.

%JARVIS_BIN% exec "What is this project about? Summarize in 2-3 sentences."

echo.
echo ==========================================
echo 🎉 RAG Integration Test Complete!
echo.
echo Next steps:
echo 1. Check if the LLM response included information from your documents
echo 2. Try more complex queries: 'How is RAG implemented?'
echo 3. Enable debug logging: set RUST_LOG=jarvis_core::rag=debug
echo.

pause
