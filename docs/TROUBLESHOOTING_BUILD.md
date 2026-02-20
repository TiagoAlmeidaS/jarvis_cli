# Troubleshooting: Erros de Compilação Rust

## Erro E0786: found invalid metadata files / O arquivo de paginação é muito pequeno

### Descrição
```
error[E0786]: found invalid metadata files for crate `jarvis_tui`
  = note: failed to mmap file '...': O arquivo de paginação é muito pequeno para que esta operação seja concluída. (os error 1455)
```

### Causa
Erro **1455** no Windows = `ERROR_COMMITMENT_LIMIT` — memória virtual (arquivo de paginação) insuficiente para mapear os artefatos de build.

### Solução
1. **Limpar e recompilar**:
   ```powershell
   cd jarvis-rs
   cargo clean
   cargo build -p jarvis-cli
   ```

2. **Aumentar o arquivo de paginação** (se o problema persistir):
   - Painel de Controle → Sistema → Configurações avançadas → Desempenho → Avançado → Memória virtual
   - Aumentar o tamanho do arquivo de paginação ou deixar o Windows gerenciar

3. **Fechar outros programas** para liberar memória antes de compilar.

---

## Erro: target corrompido / "Não foi possível localizar o caminho" (Windows)

### Descrição
```
error: failed to write `...\target\debug\.fingerprint\...\invoked.timestamp`
Caused by: O sistema não pode encontrar o caminho especificado. (os error 3)
```

Ou ao tentar `Remove-Item -Recurse target`:
```
Não foi possível localizar o arquivo 'lib-icu_decimal.json'
Não foi possível localizar uma parte do caminho
```

### Causa
- Diretório `target/` corrompido (arquivos deletados por antivírus, IDE ou processo concorrente)
- Caminhos muito longos no Windows (limite ~260 caracteres)
- Processo bloqueando arquivos

### Solução

1. **Fechar Cursor/IDE, terminal e qualquer processo Cargo** antes de limpar.

2. **Usar `target` em caminho curto** (evita limite de 260 caracteres):
   ```powershell
   cd E:\projects\ia\jarvis_cli\jarvis-rs
   $env:CARGO_TARGET_DIR = "E:\jv\target"
   cargo clean
   cargo build -p jarvis-cli
   ```

3. **Se `cargo clean` falhar**, deletar manualmente em etapas:
   ```powershell
   Remove-Item -Recurse -Force target\debug\deps -ErrorAction SilentlyContinue
   Remove-Item -Recurse -Force target\debug\.fingerprint -ErrorAction SilentlyContinue
   Remove-Item -Recurse -Force target\debug\build -ErrorAction SilentlyContinue
   cargo build -p jarvis-cli
   ```

4. **Último recurso**: mover o projeto para um caminho mais curto (ex: `C:\jv\jarvis_cli`) e compilar de lá.

---

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

## ICE: path with `Res::Err` but no error emitted (tonic/bytes)

### Descrição
```
error: internal compiler error: path with `Res::Err` but no error emitted
 --> ...\tonic-0.14.3\src\codec\buffer.rs:7:18
  |
7 |     buf: &'a mut BytesMut,
  |                  ^^^^^^^^
```

O compilador Rust falha ao resolver os tipos `Bytes` e `BytesMut` do crate `bytes` durante a compilação do `tonic` (usado pelo OpenTelemetry OTLP via gRPC).

### Causa
Bug conhecido no compilador Rust 1.93.0 ao compilar `tonic` 0.14.x. Relacionado a [rust-lang/rust#149233](https://github.com/rust-lang/rust/issues/149233).

### Solução
O projeto está configurado para usar **Rust 1.92.0** em `jarvis-rs/rust-toolchain.toml` para evitar esse ICE.

1. **Instalar Rust 1.92.0** (se ainda não tiver):
   ```powershell
   rustup install 1.92.0
   ```

2. **Verificar a toolchain ativa**:
   ```powershell
   cd jarvis-rs
   rustc --version
   # Deve mostrar: rustc 1.92.0 (...)
   ```

3. **Não alterar** o `rust-toolchain.toml` para 1.93.0 até que o bug seja corrigido em versões futuras do Rust.

---

## Resumo

| Erro | Causa | Ação |
|------|-------|------|
| E0463 (jarvis_core) | jarvis-core falha ao compilar ou build no diretório errado | `cargo build -p jarvis-core` para ver erro real; build em `jarvis-rs/` |
| E0282 (type inference) | Cascata do E0463 ou ambiguidade | Anotação de tipo explícita (já aplicada) |
| ICE Res::Err (tonic) | Bug no Rust 1.93.0 | Usar Rust 1.92.0 (ver rust-toolchain.toml) |
