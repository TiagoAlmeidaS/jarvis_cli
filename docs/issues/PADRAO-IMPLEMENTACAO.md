# Padrão de implementação para issues (autonomia Jarvis)

Este documento define o **template recomendado** para issues do repositório relacionadas à autonomia do Jarvis. Use-o ao criar ou descrever issues para manter consistência e rastreabilidade.

---

## Estrutura do corpo da issue

Cada issue deve conter, quando aplicável, as seções abaixo. Não é obrigatório preencher todas; use conforme o tipo (feature, doc, teste).

| Seção | Obrigatório | Descrição |
|-------|------------|-----------|
| **Objetivo** | Sim | Uma ou duas frases sobre o que se quer alcançar com a issue. |
| **Descrição** | Sim | Contexto, problema ou necessidade que a issue endereça. |
| **Implementação / Escopo** | Recomendado | Passos ou itens concretos a implementar (bullets ou checklist). |
| **Critério de aceite** | Sim | Condições mensuráveis para considerar a issue concluída. |
| **Referências** | Recomendado | Links para docs (architecture, features), código ou issues relacionadas. |
| **Labels sugeridas** | Opcional | Tags sugeridas para triagem (ex.: `feature`, `daemon`, `autonomy`). |

---

## Template em Markdown (copiar e colar)

```markdown
## Objetivo
<!-- O que queremos alcançar com esta issue? -->

## Descrição
<!-- Contexto, problema ou necessidade. -->

## Implementação / Escopo
- [ ] Item 1
- [ ] Item 2
- [ ] (opcional) Item 3

## Critério de aceite
- [ ] Critério 1
- [ ] Critério 2

## Referências
- Doc: [nome](caminho)
- Código: `caminho/arquivo.rs` (se aplicável)

## Labels sugeridas
`tipo` (ex.: feature, documentation, testing) | `área` (ex.: daemon, core, cli) | `autonomy`
```

---

## Recomendações

1. **Objetivo curto**: uma linha quando possível; facilita leitura no board.
2. **Critério de aceite testável**: evite "melhorar X"; prefira "X faz Y em cenário Z".
3. **Acoplamento**: sempre que a issue depender de outro componente (daemon, core, config), cite-o em Referências ou Descrição. Regra do projeto: *daemon orquestra, core executa*; não duplicar scanner/executor.
4. **Doc**: se a mudança afetar comportamento de usuário ou deploy, inclua em Critério de aceite "Doc atualizada em X".
5. **Ordem**: issues de autonomia seguem o [Caminho até a autonomia](README.md#caminho-até-a-autonomia); mencione a ordem ou o conjunto (ex.: "Fase 1 — Board configurável") no corpo quando fizer sentido.

---

## Onde encontrar as issues sugeridas

As issues já redigidas segundo este padrão estão nos arquivos desta pasta:

- [daemon-google-free-tier.md](daemon-google-free-tier.md)
- [daemon-integration-tests-google.md](daemon-integration-tests-google.md)
- [autonomy-loop-gaps.md](autonomy-loop-gaps.md)
- [board-renda-phases.md](board-renda-phases.md)
- [autonomy-growth-self-learning.md](autonomy-growth-self-learning.md)
- [operacao-observabilidade.md](operacao-observabilidade.md)

Cada bloco nesses arquivos pode ser copiado para o GitHub/GitLab; as criadas via script ou API devem usar o corpo no formato acima.

---

**Última atualização:** 2026-03-11
