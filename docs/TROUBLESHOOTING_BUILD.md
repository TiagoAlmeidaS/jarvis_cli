# Troubleshooting: Erros de Compilação Rust

## Erro E0463: can't find crate for `jarvis_core`

### Descrição
Vários crates (ollama, arg0, lmstudio, backend-client, login) reportam:
```
error[E0463]: can't find crate for `jarvis_core`
```

### Causa provável
O erro **não indica** que a dependência está faltando no `Cargo.toml`. Todos os crates afetados já declaram `jarvis-core = { workspace = true }` corretamente.

A causa mais provável é que **o crate `jarvis-core` (pasta `core/`) falha ao compilar primeiro**. Quando uma dependência falha, os crates que dependem dela recebem "can't find crate" porque a dependência não está disponível.

### Verificações

1. **Diretório correto**: Sempre execute o build a partir de `jarvis-rs/`:
   ```powershell
   cd E:\projects\ia\jarvis_cli\jarvis-rs
   cargo build
   ```

2. **Compilar o core isoladamente** para ver o erro real:
   ```powershell
   cargo build -p jarvis-core
   ```
   Se houver erro aqui, esse é o problema raiz.

3. **Limpar e recompilar** (se houver cache corrompido):
   ```powershell
   cargo clean
   cargo build
   ```

4. **Verificar bloqueio de artefatos**: Se outro processo Cargo estiver rodando, aguarde ou finalize-o. A mensagem "Blocking waiting for file lock on artifact directory" indica isso.

---

## Erro E0282: type annotations needed

### Descrição
```
error[E0282]: type annotations needed
   --> arg0\src\lib.rs:135:51
    |
135 |         && let Ok(iter) = dotenvy::from_path_iter(jarvis_home.join(".env"))
    |                                                   ^^^^^^^^^^^ cannot infer type
```

### Causa
Quando `jarvis_core` não compila (ou há ambiguidade), o compilador não consegue inferir o tipo de retorno de `find_jarvis_home()`, e portanto o tipo de `jarvis_home`.

### Correção aplicada
Foi adicionada anotação de tipo explícita em `arg0/src/lib.rs`:
```rust
if let Ok(jarvis_home): Result<PathBuf, _> = jarvis_core::config::find_jarvis_home()
    && let Ok(iter) = dotenvy::from_path_iter(jarvis_home.join(".env"))
```

---

## Resumo

| Erro | Causa | Ação |
|------|-------|------|
| E0463 (jarvis_core) | jarvis-core falha ao compilar ou build no diretório errado | `cargo build -p jarvis-core` para ver erro real; build em `jarvis-rs/` |
| E0282 (type inference) | Cascata do E0463 ou ambiguidade | Anotação de tipo explícita (já aplicada) |
