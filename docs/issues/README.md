# Pasta de issues (central)

Esta pasta concentra **issues sugeridas** para o projeto, em formato de documento. Cada arquivo descreve um conjunto de issues (título, descrição, critério de aceite) para você criar no GitHub, GitLab ou outro tracker.

**Uso:** abra o arquivo do tema desejado, copie cada bloco de issue e crie a issue no repositório. Para o formato recomendado (Objetivo, Descrição, Implementação, Critério de aceite), veja [PADRAO-IMPLEMENTACAO.md](PADRAO-IMPLEMENTACAO.md).

**Criar todas no GitHub:** com [GitHub CLI](https://cli.github.com/) autenticado, execute na raiz do repo: `powershell -ExecutionPolicy Bypass -File scripts/create-autonomy-issues.ps1`. O script cria as 28 issues com corpo no padrão acima e labels (documentation, enhancement, daemon, autonomy, board).

**Corrigir caracteres especiais (encoding):** se as issues aparecerem com caracteres quebrados (ex.: Ã§ em vez de ç), os corpos foram gerados com encoding errado no PowerShell. Use os arquivos UTF-8 em `docs/issues/bodies/` (27.md a 54.md) e execute: `powershell -ExecutionPolicy Bypass -File scripts/update-autonomy-issues-bodies.ps1`. Esse script atualiza as issues #27–#54 com o conteúdo em UTF-8. Pode ser reexecutado a qualquer momento.

---

## Conjuntos de issues

| Arquivo | Tema | Resumo |
|---------|------|--------|
| [daemon-google-free-tier.md](daemon-google-free-tier.md) | Daemon só com Google (free tier) | 6 issues para o daemon funcionar por completo usando apenas Gemini (Google AI Studio), sem OpenRouter. |
| [daemon-integration-tests-google.md](daemon-integration-tests-google.md) | Testes de integração (daemon + Google) | 1 issue: testes de integração do fluxo do daemon com Google (Gemini). |
| [autonomy-loop-gaps.md](autonomy-loop-gaps.md) | Fechar loop autônomo (G1–G6) | Issues para validar/completar G1 (Proposal Executor), G2 (Goal System), G3 (Real Data), G4 (Tool Calling), G5 (Agentic Loop), G6 (Sandbox). |
| [board-renda-phases.md](board-renda-phases.md) | Fases Board e Renda | 4 fases: Board configurável, loop autônomo, métricas/custo, porta para renda. Daemon orquestra; core executa. |
| [autonomy-growth-self-learning.md](autonomy-growth-self-learning.md) | Crescimento e auto-aprendizado | Dados reais no metrics/goals, A/B e prompt_optimizer, integração core/autonomous ao daemon, doc do feedback loop. |
| [operacao-observabilidade.md](operacao-observabilidade.md) | Operação e observabilidade | Health check, logs e correlação job/pipeline, métricas e alertas para o daemon em produção. |
| [daemon-telegram-notifications.md](daemon-telegram-notifications.md) | Notificações Telegram do daemon | Documentar e configurar resumo diário e alertas via Telegram (JARVIS_TELEGRAM_*). |
| [jarvis-wallet-revenue.md](jarvis-wallet-revenue.md) | Carteira do Jarvis (fundo de receita) | Ledger/atribuição de receita gerada pelo sistema (AdSense, afiliados); doc de direção futura. |

---

## Caminho até a autonomia

Ordem sugerida para implementar os conjuntos acima até o objetivo final (Jarvis executar, observar, decidir, atuar e aprender, com board e caminho para renda):

1. **Já existente:** [daemon-google-free-tier](daemon-google-free-tier.md), [daemon-integration-tests-google](daemon-integration-tests-google.md) — daemon rodando e testado com Google.
2. [autonomy-loop-gaps](autonomy-loop-gaps.md) — fechar o loop observe → decide → act (base da autonomia).
3. [board-renda-phases](board-renda-phases.md) (Fases 1 e 2) — board configurável e daemon resolvendo issues sozinho.
4. [board-renda-phases](board-renda-phases.md) (Fases 3 e 4) — métricas/custo e porta para renda.
5. [autonomy-growth-self-learning](autonomy-growth-self-learning.md) — aprendizado e melhoria contínua (dados reais, A/B, core/daemon).
6. [operacao-observabilidade](operacao-observabilidade.md) — operação e observabilidade em produção.

Referências de arquitetura: [autonomy-roadmap](../architecture/autonomy-roadmap.md), [evolução Board e Renda](../architecture/evolucao-board-e-renda-levantamento.md).

---

## Por que centralizar aqui

- **Documentação versionada**: as issues ficam no repo junto com o código e as docs.
- **Referência única**: qualquer um pode ver o que está planejado e em que ordem implementar.
- **Fácil de expandir**: novos conjuntos viram novos arquivos nesta pasta (ex.: `board-configurável.md`, `loop-autonomo.md`).

---

**Última atualização:** 2026-03-12
