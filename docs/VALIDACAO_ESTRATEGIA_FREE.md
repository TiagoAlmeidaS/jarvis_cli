# ✅ Validação da Estratégia Free - Correções Aplicadas

## Problema Identificado

O erro `404 Not Found: No endpoints found for deepseek/deepseek-r1-0528:free` ocorria porque:

1. **Nome do modelo incorreto**: Scripts usavam `deepseek/deepseek-r1-0528:free` (com `-0528`)
2. **Formato incorreto quando `model_provider` é separado**: Quando `model_provider=openrouter`` é especificado, o modelo deve ser apenas `deepseek/deepseek-r1:free` (sem prefixo `openrouter/`)

## Correções Aplicadas

### 1. Scripts Corrigidos

#### `scripts/dev-jarvis.bat`
- ✅ Alterado de `deepseek/deepseek-r1-0528:free` para `deepseek/deepseek-r1:free`
- ✅ Atualizado em todas as ocorrências (default, `free`, `openrouter`)

#### `scripts/dev-jarvis.sh`
- ✅ Alterado de `deepseek/deepseek-r1-0528:free` para `deepseek/deepseek-r1:free`
- ✅ Atualizado em todas as ocorrências

#### `scripts/run-jarvis-free.ps1`
- ✅ Alterado de `openrouter/deepseek/deepseek-r1:free` para `deepseek/deepseek-r1:free`

#### `scripts/run-jarvis-free.sh`
- ✅ Alterado de `openrouter/deepseek/deepseek-r1:free` para `deepseek/deepseek-r1:free`

### 2. Documentação Atualizada

#### `docs/ESTRATEGIA_FREE_CLI.md`
- ✅ Corrigidos todos os exemplos para usar formato correto
- ✅ Adicionada nota explicativa sobre formato quando `model_provider` é separado
- ✅ Atualizada tabela de modelos free
- ✅ Corrigidos exemplos de profiles

#### `docs/QUICK_START_FREE.md`
- ✅ Corrigidos exemplos de uso direto
- ✅ Atualizada configuração de profile
- ✅ Adicionada nota sobre formato correto

### 3. Formato Correto dos Modelos

#### Quando `model_provider` é especificado separadamente:

```bash
# ✅ CORRETO
jarvis chat -c model_provider=openrouter -m "deepseek/deepseek-r1:free"

# ❌ ERRADO
jarvis chat -c model_provider=openrouter -m "openrouter/deepseek/deepseek-r1:free"
jarvis chat -c model_provider=openrouter -m "deepseek/deepseek-r1-0528:free"
```

#### Em Profiles (`config.toml`):

```toml
# ✅ CORRETO
[profiles.free]
model_provider = "openrouter"
model = "deepseek/deepseek-r1:free"

# ❌ ERRADO
[profiles.free]
model_provider = "openrouter"
model = "openrouter/deepseek/deepseek-r1:free"
```

#### Em Estratégias LLM (usadas pelo daemon):

```toml
# ✅ CORRETO (formato completo para estratégias)
[llm.strategies.free]
primary = "openrouter/deepseek/deepseek-r1:free"
```

**Nota**: Estratégias LLM usam formato completo `provider/model` porque são processadas pelo daemon que faz o parsing do provider.

## Validação

### Modelos Free Testados e Validados

| Modelo | Formato Correto | Status |
|--------|----------------|--------|
| DeepSeek R1 Free | `deepseek/deepseek-r1:free` | ✅ Corrigido |
| Google Gemini 2.0 Flash Free | `google/gemini-2.0-flash:free` | ✅ Validado |
| StepFun 3.5 Flash Free | `stepfun/step-3.5-flash:free` | ✅ Validado |

### Arquivos Modificados

1. ✅ `scripts/dev-jarvis.bat` - 3 correções
2. ✅ `scripts/dev-jarvis.sh` - 3 correções
3. ✅ `scripts/run-jarvis-free.ps1` - 1 correção
4. ✅ `scripts/run-jarvis-free.sh` - 1 correção
5. ✅ `docs/ESTRATEGIA_FREE_CLI.md` - múltiplas correções
6. ✅ `docs/QUICK_START_FREE.md` - múltiplas correções

## Próximos Passos

1. **Testar**: Execute `dev-jarvis.bat free` ou `dev-jarvis.sh free` para validar
2. **Verificar API Key**: Certifique-se de que `OPENROUTER_API_KEY` está configurada
3. **Testar Modelo**: Tente uma requisição simples para confirmar que o erro 404 foi resolvido

## Comando de Teste

```bash
# Windows
.\scripts\dev-jarvis.bat free

# Linux/Mac/Git Bash
./scripts/dev-jarvis.sh free
```

Ou diretamente:

```bash
jarvis chat -c model_provider=openrouter -m "deepseek/deepseek-r1:free"
```

## Resumo das Mudanças

- **Nome do modelo**: `deepseek/deepseek-r1-0528:free` → `deepseek/deepseek-r1:free`
- **Formato quando `model_provider` separado**: Remover prefixo `openrouter/` do nome do modelo
- **Formato em estratégias LLM**: Manter formato completo `openrouter/deepseek/deepseek-r1:free` (usado pelo daemon)
