# Agent Teams

Declarative multi-agent orchestration system for Jarvis CLI.
Allows users to define teams of AI agents in a `teams.yaml` file,
where a **Lead** agent orchestrates **Teammates** to accomplish complex tasks.

Inspired by Anthropic's "Agent Teams" architecture with isolated contexts
and orchestrator-worker patterns.

## Architecture Overview

```
teams.yaml
    |
    v
[Schema & Parser] --> TeamsConfig, TeamDefinition, TeammateDefinition
    |
    v
[Validation & Loader] --> validate_teams(), load_teams()
    |
    v
[Skill Filtering] --> allowed_skills on Config, filter_skills_by_allowed()
    |
    v
[Orchestrator] --> build_lead_prompt() generates system prompt with team roster
    |
    v
[CLI Command] --> `jarvis teams run <team> <task>` launches lead via jarvis-exec
    |
    v
[Lead Agent] --> uses spawn_agent(teammate_name, team_name) to delegate
    |
    v
[Teammate Agents] --> spawned with teammate-specific config (model, role, skills)
```

## Configuration Format (`teams.yaml`)

```yaml
teams:
  backend-team:
    description: "Team for backend development tasks"
    lead:
      model: "o3"
      role: "You are a senior backend architect..."
      skills: ["rust", "database"]
      reasoning_effort: "high"
    teammates:
      rust-dev:
        model: "gpt-4.1"
        role: "You are a Rust developer..."
        skills: ["rust"]
        read_only: false
      db-expert:
        model: "o4-mini"
        role: "You are a database specialist..."
        skills: ["database", "sql"]
        reasoning_effort: "medium"
```

See `jarvis-rs/teams.yaml.example` for a complete documented example.

## Implementation Phases

### Phase 1: Schema & Parser - COMPLETE

**Status**: Done (46 tests passing)

**Files created**:

- `core/src/teams/mod.rs` - Module declarations and re-exports
- `core/src/teams/config.rs` - Data structures: `TeamsConfig`, `TeamDefinition`, `TeammateDefinition`
- `core/src/teams/resolver.rs` - `resolve_model_spec()` + `ResolvedModel` (validates model names against known providers)
- `core/src/teams/validation.rs` - `validate_teams()` + `TeamValidationError` enum
- `core/src/teams/loader.rs` - `load_teams()` + `TeamLoadOutcome` + `TeamLoadError`
- `teams.yaml.example` - Reference configuration with documentation

**Key design decisions**:

- YAML format (via `serde_yaml`)
- Model resolution validates against `built_in_model_providers()` from `model_provider_info.rs`
- Validation checks: duplicate names, empty teams, model validity, skill existence
- Loader searches CWD, home dir, and explicit paths

### Phase 2: Skill Filtering per Teammate - COMPLETE

**Status**: Done (7 filter tests passing)

**Files created**:

- `core/src/skills/filter.rs` - `filter_skills_by_allowed()`, `filter_skills_ref_by_allowed()`

**Files modified**:

- `core/src/config/mod.rs` - Added `allowed_skills: Option<Vec<String>>` to `Config` struct
- `core/src/skills/mod.rs` - Added filter module + re-exports
- `core/src/jarvis.rs` - Filters skills through `allowed_skills` at session init and per-turn
- `core/src/tools/spec.rs` - Added `teammate_name` and `team_name` params to `spawn_agent` tool
- `core/src/tools/handlers/collab.rs` - Added `find_teammate()`, `apply_teammate_to_config()`, teammate-based spawn flow

**Key design decisions**:

- **Skills are NOT tools**. Skills = SKILL.md prompt injections. Tools = shell, grep, etc.
- `allowed_skills` on Config filters which skill prompts get injected (both at session init and per-turn)
- All tools remain available to all agents regardless of skill filtering
- `teammate_name` + `team_name` parameters on `spawn_agent` take precedence over `agent_type`

### Phase 3: Team Orchestration + CLI - IN PROGRESS (code written, tests pending)

**Status**: All code written and compiles (`cargo check -p jarvis-cli` passed). Tests not yet run.

**Files created**:

- `core/src/teams/orchestrator.rs` - `build_lead_prompt(team_name, team)` + 4 unit tests
- `cli/src/teams_cmd.rs` - `TeamsCli`, `TeamsCommand` enum (~407 lines)

**Files modified**:

- `core/src/teams/mod.rs` - Added `pub mod orchestrator;` + `pub use`
- `cli/src/lib.rs` - Added `pub mod teams_cmd;`
- `cli/src/main.rs` - Added `Teams` variant to `Subcommand` enum + dispatch arm

**CLI subcommands**:

| Command                          | Description                                      |
| -------------------------------- | ------------------------------------------------ |
| `jarvis teams list`              | Lists all teams and teammates from teams.yaml    |
| `jarvis teams validate`          | Validates teams.yaml, reports errors             |
| `jarvis teams show <team>`       | Detailed view of team + lead prompt preview      |
| `jarvis teams run <team> <task>` | Runs the lead agent with generated system prompt |

**`teams run` flow**:

1. Loads `teams.yaml` via `load_teams()`
2. Validates with `validate_teams()`
3. Builds lead system prompt via `build_lead_prompt()` (role + auto-generated team roster)
4. Writes prompt to temp file
5. Constructs `ExecCli` via `try_parse_from` with model/instructions/reasoning overrides
6. Calls `jarvis_exec::run_main()` to start the session
7. Lead organically uses `spawn_agent(teammate_name: "...", team_name: "...")` to delegate

**`build_lead_prompt` output example**:

```
<lead's role text>

## Your Team

You have the following teammates available for delegation:

### rust-dev
- **Model**: gpt-4.1
- **Role**: You are a Rust developer...
- **Skills**: rust
- **Read-only**: no

### Delegation Guide
Use spawn_agent with teammate_name to delegate tasks:
spawn_agent(message: "...", teammate_name: "rust-dev", team_name: "backend-team")
```

### Phase 4: Shared Memory + Hot-Reload - NOT STARTED

**Planned features**:

- SQLite-based shared memory for inter-agent task results
- File watcher for `teams.yaml` hot-reload during active sessions
- Persistent task/result store accessible across team members

## Where to Continue

### Immediate Next Steps

1. **Run tests** to confirm Phase 3 code works:

   ```bash
   cargo test -p jarvis-core --lib -- teams::
   ```

   This should run all Phase 1 (46), Phase 2 (7), and Phase 3 orchestrator (4) tests.

2. **Fix any test failures** if they occur.

3. **Manual smoke test** of the CLI commands:
   ```bash
   jarvis teams list
   jarvis teams validate
   jarvis teams show <team-name>
   jarvis teams run <team-name> "some task description"
   ```

### After Validation

4. **Consider edge cases**:
   - What happens if `teams.yaml` doesn't exist when running `teams run`?
   - Error messages when teammate_name doesn't match any teammate
   - Behavior when lead model is unavailable

5. **Phase 4 planning** - Define the shared memory schema and hot-reload mechanism.

## Key Technical Insights

### Skills vs Tools

- **Skills** = SKILL.md files whose content is injected into conversation as prompt text
- **Tools** = Shell, apply_patch, grep_files, read_file, collab, GitHub tools, MCP tools
- These are **completely separate systems** with zero integration

### Skill Injection Points (in `jarvis.rs`)

1. **Session init** (~line 279): `enabled_skills()` -> `filter_skills_by_allowed()` -> `render_skills_section()` (static system prompt section)
2. **Per-turn** (~line 3505): `skills_for_cwd()` -> `filter_skills_ref_by_allowed()` -> `collect_explicit_skill_mentions()` -> `build_skill_injections()` (dynamic injection when `$skill-name` mentioned)

### Agent Spawn Flow

```
spawn_agent tool
    -> CollabHandler
    -> SpawnAgentArgs { message, agent_type, teammate_name, team_name }
    -> find_teammate() (if teammate_name provided)
    -> apply_teammate_to_config() (sets model, role, allowed_skills, reasoning_effort, read_only)
    -> build_agent_spawn_config() (clones parent Config, overrides)
    -> AgentControl::spawn_agent(config, prompt, session_source)
```

### Config Override Chain for `teams run`

```
teams.yaml lead config
    -> build_lead_prompt() generates system prompt
    -> write to temp file
    -> ExecCli via try_parse_from:
         -c model=<lead_model>
         -c model_instructions_file=<temp_file>
         -c model_reasoning_effort=<effort> (if set)
         --full-auto
    -> jarvis_exec::run_main()
    -> Config loads model_instructions_file as base_instructions
```

## File Map

```
jarvis-rs/
  teams.yaml.example              # Reference config
  docs/features/agent-teams.md    # This document
  core/src/
    lib.rs                        # Added: pub mod teams
    teams/
      mod.rs                      # Module declarations + re-exports
      config.rs                   # TeamsConfig, TeamDefinition, TeammateDefinition
      resolver.rs                 # resolve_model_spec(), ResolvedModel
      validation.rs               # validate_teams(), TeamValidationError
      loader.rs                   # load_teams(), TeamLoadOutcome, TeamLoadError
      orchestrator.rs             # build_lead_prompt()
    config/
      mod.rs                      # Modified: added allowed_skills field
    skills/
      mod.rs                      # Modified: added filter module
      filter.rs                   # filter_skills_by_allowed(), filter_skills_ref_by_allowed()
    jarvis.rs                     # Modified: skill filtering at init + per-turn
    tools/
      spec.rs                     # Modified: teammate_name/team_name on spawn_agent
      handlers/
        collab.rs                 # Modified: find_teammate(), apply_teammate_to_config()
  cli/src/
    lib.rs                        # Modified: added pub mod teams_cmd
    main.rs                       # Modified: Teams variant in Subcommand enum
    teams_cmd.rs                  # TeamsCli, TeamsCommand (List/Validate/Show/Run)
```
