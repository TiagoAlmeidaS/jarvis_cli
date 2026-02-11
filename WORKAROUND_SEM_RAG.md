# 🔧 Workaround: Como fazer Jarvis entender o projeto SEM RAG

## 📊 Status Atual

**Descoberta**: O sistema RAG está **apenas parcialmente implementado**.

### ✅ Infraestrutura OK
- Qdrant rodando na VPS ✅ (`http://100.98.213.86:6333`)
- Config apontando corretamente via Tailscale ✅
- Redis e PostgreSQL disponíveis ✅
- Comandos `context` funcionam ✅

### ❌ Backend não implementado
- Documentos não persistem no Qdrant ❌
- Nenhuma coleção criada ❌
- Embeddings não são gerados ❌
- Sistema RAG não integrado ao chat ❌

**Conclusão**: Como documentado em `docs/features/rag-context-management.md`:
> **Status**: 📝 Planejado
> Sistema RAG ainda em desenvolvimento

---

## 🎯 Soluções Práticas (Sem RAG)

### **Opção 1: Prompt Explícito com Instruções** ⭐ Recomendado

Cole este prompt no Jarvis:

```
Por favor, analise este projeto Jarvis CLI:

1. Leia os seguintes arquivos principais:
   - README.md (raiz do projeto)
   - jarvis-rs/README.md
   - jarvis-rs/Cargo.toml
   - jarvis-rs/cli/Cargo.toml
   - jarvis-rs/cli/src/main.rs
   - jarvis-rs/core/src/lib.rs

2. Depois de ler, me explique:
   - Qual é o propósito deste projeto?
   - Qual a arquitetura principal?
   - Quais são os principais componentes?
   - Como ele funciona?
   - Quais tecnologias usa?

Use as ferramentas Read, Glob e Grep para explorar o código-fonte.
Não faça busca na web - use apenas os arquivos locais.
```

O Jarvis **TEM** as ferramentas necessárias:
- ✅ `Read` - Ler arquivos
- ✅ `Glob` - Buscar arquivos por padrão
- ✅ `Grep` - Buscar conteúdo em arquivos
- ✅ `Bash` - Executar comandos

Ele só precisa ser **instruído explicitamente** a usá-las!

---

### **Opção 2: Usar `exec` com arquivos anexados**

```bash
cd jarvis-rs

./target/debug/jarvis.exe exec \
  "Analise este projeto e me dê uma visão geral completa" \
  --attach ../README.md \
  --attach ./README.md \
  --attach ./Cargo.toml
```

*(Nota: Verifique se `--attach` é suportado com `jarvis.exe exec --help`)*

---

### **Opção 3: Modo Interativo com Contexto de Diretório**

```bash
cd jarvis-rs

# Iniciar no diretório correto
./target/debug/jarvis.exe -C . chat

# No chat, perguntar:
# "Por favor, leia o README.md e me explique este projeto"
```

O Jarvis vai ter acesso ao working directory e pode ler arquivos livremente.

---

### **Opção 4: Skills Personalizadas** (Avançado)

Criar uma skill `analyze-project.toml`:

```toml
[skill]
name = "analyze-project"
description = "Analisa o projeto Jarvis CLI atual"
version = "1.0.0"

[skill.prompts]
initial = """
Você é um assistente especializado em análise de código Rust.

Tarefas:
1. Use Glob para encontrar todos os Cargo.toml
2. Use Read para ler README.md
3. Use Grep para encontrar arquivos principais (.rs)
4. Analise a estrutura do projeto
5. Forneça uma visão geral completa

Não use busca na web. Use apenas ferramentas locais.
"""

[skill.tools]
allowed = ["Read", "Glob", "Grep", "Bash"]
```

Depois:
```bash
jarvis skills add analyze-project.toml
jarvis analyze-project
```

---

## 🔍 Por que o Jarvis busca na web ao invés de ler localmente?

### Comportamento Atual
Quando você pergunta "Consegue analisar seu projeto?", o Jarvis:
1. ❌ Não tem contexto RAG (porque não está implementado)
2. ❌ Não sabe que deve ler arquivos locais (falta instrução)
3. ✅ Usa tool `web_search` para buscar na internet
4. ❌ Encontra "J.A.R.V.I.S." do Iron Man (coincidência de nome)

### Como o Claude Code faz diferente?

**Claude Code**:
- ✅ **Auto-indexa** o diretório de trabalho no startup
- ✅ **Prioriza ferramentas locais** (Read, Grep) antes de web search
- ✅ **Mantém contexto** do projeto na conversa
- ✅ **Sistema RAG integrado** por padrão

**Jarvis (estado atual)**:
- ⚠️ Sistema RAG **planejado mas não implementado**
- ⚠️ **Não auto-indexa** (precisa instrução manual)
- ⚠️ **Não prioriza local** (web search é default)
- ✅ **Tem as ferramentas** mas precisa ser instruído

---

## 💡 Recomendação Imediata

**Para análise de projeto:**

1. **Use Opção 1** (prompt explícito) - Mais confiável
2. Seja **específico** sobre quais arquivos ler
3. **Instrua explicitamente** para não usar web search
4. Peça para usar **ferramentas locais** (Read, Glob, Grep)

**Exemplo de conversa eficaz:**

```
User: "Ignore web search. Use apenas Read e Glob.
       Leia README.md, jarvis-rs/README.md e Cargo.toml.
       Depois me explique o que é este projeto."

Jarvis: [Usa Read tool]
        [Lê os arquivos]
        [Analisa baseado no conteúdo local]
        "Este é o Jarvis CLI, um assistente de IA..."
```

---

## 📅 Roadmap RAG

De acordo com `docs/features/rag-context-management.md`:

### Fase 2: Vector Store (Sprint 2) - **EM PROGRESSO**
- [ ] Implementar integração com Qdrant
- [ ] Implementar fallback in-memory
- [ ] Integrar geração de embeddings ← **Faltando**
- [ ] Implementar busca semântica ← **Faltando**

### Fase 3: Context Compression (Sprint 3) - **NÃO INICIADO**
### Fase 4: Development Context Tracking (Sprint 4) - **NÃO INICIADO**

**Previsão**: RAG 100% funcional provavelmente em 1-2 meses de desenvolvimento.

---

## 🎯 Conclusão

**Enquanto RAG não está pronto:**

✅ **Use prompts explícitos** instruindo Jarvis a ler arquivos locais
✅ **Especifique quais arquivos** você quer que ele analise
✅ **Instrua para NÃO usar web search**
✅ **Mencione as ferramentas** que ele deve usar (Read, Glob, Grep)

**Quando RAG estiver pronto:**

✅ Auto-indexação do projeto
✅ Contexto persistente entre sessões
✅ Busca semântica automática
✅ Priorização de contexto local sobre web

O Jarvis **já tem as capacidades** - só precisa de orientação explícita até o RAG estar integrado! 🎯
