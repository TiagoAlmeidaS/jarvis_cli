@echo off
REM End-to-End Test Suite for RAG Integration (Windows)
setlocal enabledelayedexpansion

set JARVIS_BIN=jarvis-rs\target\release\jarvis.exe
set TEST_DIR=%TEMP%\jarvis-rag-test-%RANDOM%
set TEST_DOCS_DIR=%TEST_DIR%\docs

set TESTS_PASSED=0
set TESTS_FAILED=0
set TESTS_TOTAL=0

echo.
echo ========================================
echo RAG End-to-End Test Suite
echo ========================================
echo.
echo Test directory: %TEST_DIR%
echo Jarvis binary: %JARVIS_BIN%
echo.

REM Create test directory
mkdir "%TEST_DIR%" 2>NUL
mkdir "%TEST_DOCS_DIR%" 2>NUL

REM Create test documents
echo Creating test documents...

REM Document 1: README
(
echo # Jarvis CLI
echo.
echo Jarvis is an AI-powered coding assistant built in Rust.
echo.
echo ## Features
echo.
echo - RAG ^(Retrieval Augmented Generation^) for context-aware responses
echo - Qdrant vector storage at http://100.98.213.86:6333
echo - Ollama embeddings using nomic-embed-text model ^(768 dimensions^)
echo - PostgreSQL for document persistence
echo - Interactive TUI and non-interactive exec modes
echo.
echo ## Authentication
echo.
echo The project implements JWT-based authentication using the AuthManager component.
echo Users can authenticate via OAuth device code or API key.
) > "%TEST_DOCS_DIR%\README.md"

REM Document 2: RAG Architecture
(
echo # RAG System Architecture
echo.
echo ## Components
echo.
echo ### 1. Vector Store
echo - Primary: Qdrant ^(production^)
echo - Fallback: InMemory ^(testing/development^)
echo - Similarity: Cosine distance
echo - Dimensions: 768 ^(nomic-embed-text^)
echo.
echo ### 2. Embedding Generator
echo - Service: Ollama
echo - Model: nomic-embed-text
echo - Endpoint: http://100.98.213.86:11434
echo - Batch support: Yes
) > "%TEST_DOCS_DIR%\rag-architecture.md"

REM Document 3: Code Example
(
echo // Example: RAG Context Injection
echo.
echo use jarvis_core::rag::{create_rag_injector, inject_rag_context, RagContextConfig};
echo.
echo async fn process_message^(message: ^&str^) -^> String {
echo     // Initialize RAG injector
echo     let injector = create_rag_injector^(^).await;
echo.
echo     // Configure RAG
echo     let config = RagContextConfig {
echo         max_chunks: 5,
echo         min_score: 0.7,
echo         enabled: true,
echo     };
echo.
echo     // Inject context
echo     inject_rag_context^(message, ^&injector, ^&config^)
echo         .await
echo         .unwrap_or_else^(^|_^| message.to_string^(^)^)
echo }
) > "%TEST_DOCS_DIR%\example.rs"

echo Created 3 test documents
echo.

REM ========== TESTS ==========

echo ========== Running Tests ==========
echo.

REM Test 1: Binary exists
echo Test: Binary exists
if exist "%JARVIS_BIN%" (
    echo [PASS] Binary exists
    set /a TESTS_PASSED+=1
) else (
    echo [FAIL] Binary not found at %JARVIS_BIN%
    echo Build it with: cd jarvis-rs ^&^& cargo build --release --features qdrant,postgres
    set /a TESTS_FAILED+=1
    goto :summary
)
set /a TESTS_TOTAL+=1

REM Test 2: Context add
echo Test: Adding document to context
%JARVIS_BIN% context add "%TEST_DOCS_DIR%\README.md" --tags test,readme 2>&1 | findstr /C:"indexed successfully" >NUL
if %ERRORLEVEL% == 0 (
    echo [PASS] Context add command
    set /a TESTS_PASSED+=1
) else (
    echo [FAIL] Context add command
    set /a TESTS_FAILED+=1
)
set /a TESTS_TOTAL+=1

REM Test 3: Context list
echo Test: Listing documents
%JARVIS_BIN% context list 2>&1 | findstr /C:"Total documents" >NUL
if %ERRORLEVEL% == 0 (
    echo [PASS] Context list command
    set /a TESTS_PASSED+=1
) else (
    echo [FAIL] Context list command
    set /a TESTS_FAILED+=1
)
set /a TESTS_TOTAL+=1

REM Test 4: Context stats
echo Test: Getting context stats
%JARVIS_BIN% context stats 2>&1 | findstr /C:"Total Documents" >NUL
if %ERRORLEVEL% == 0 (
    echo [PASS] Context stats command
    set /a TESTS_PASSED+=1
) else (
    echo [FAIL] Context stats command
    set /a TESTS_FAILED+=1
)
set /a TESTS_TOTAL+=1

REM Test 5: Context search
echo Test: Semantic search
%JARVIS_BIN% context search "RAG authentication" -n 3 2>&1 | findstr /C:"Searching context" >NUL
if %ERRORLEVEL% == 0 (
    echo [PASS] Context search command
    set /a TESTS_PASSED+=1
) else (
    echo [FAIL] Context search command
    set /a TESTS_FAILED+=1
)
set /a TESTS_TOTAL+=1

REM Test 6: Add multiple documents
echo Test: Adding multiple documents
set DOCS_ADDED=0
for %%f in ("%TEST_DOCS_DIR%\*.md") do (
    %JARVIS_BIN% context add "%%f" --tags test 2>&1 | findstr /C:"indexed successfully" >NUL
    if !ERRORLEVEL! == 0 (
        set /a DOCS_ADDED+=1
    )
)
if !DOCS_ADDED! GEQ 2 (
    echo [PASS] Add multiple documents ^(!DOCS_ADDED! added^)
    set /a TESTS_PASSED+=1
) else (
    echo [FAIL] Add multiple documents
    set /a TESTS_FAILED+=1
)
set /a TESTS_TOTAL+=1

REM Test 7: JSON output
echo Test: JSON output format
%JARVIS_BIN% context stats -o json 2>&1 | findstr /C:"total_documents" >NUL
if %ERRORLEVEL% == 0 (
    echo [PASS] JSON output format
    set /a TESTS_PASSED+=1
) else (
    echo [FAIL] JSON output format
    set /a TESTS_FAILED+=1
)
set /a TESTS_TOTAL+=1

REM Test 8: Search with filters
echo Test: Search with score filter
%JARVIS_BIN% context search "Ollama embeddings" --min-score 0.5 -n 5 2>&1 | findstr /C:"Searching" >NUL
if %ERRORLEVEL% == 0 (
    echo [PASS] Search with filters
    set /a TESTS_PASSED+=1
) else (
    echo [FAIL] Search with filters
    set /a TESTS_FAILED+=1
)
set /a TESTS_TOTAL+=1

:summary
REM Print summary
echo.
echo ========================================
echo Test Summary
echo ========================================
echo.
echo Total tests:  %TESTS_TOTAL%
echo Passed:       %TESTS_PASSED%
echo Failed:       %TESTS_FAILED%
echo.

if %TESTS_FAILED% == 0 (
    echo [SUCCESS] All tests passed!
    set EXIT_CODE=0
) else (
    echo [FAILED] Some tests failed
    set EXIT_CODE=1
)

REM Cleanup
echo.
echo Cleaning up test environment...
rd /s /q "%TEST_DIR%" 2>NUL

echo.
echo Tests complete.
pause
exit /b %EXIT_CODE%
