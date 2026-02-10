# Jarvis Agents

Jarvis supports multiple agent roles, each optimized for specific tasks. This document describes the available agent roles and how to use them.

## Available Agent Roles

### Default
The default agent role that inherits the parent agent's configuration unchanged. Use this when you want standard behavior without role-specific optimizations.

### Planner
The **Planner** agent is responsible for strategic analysis and plan creation. It analyzes user requests and creates structured, actionable plans.

**Use cases:**
- Breaking down complex tasks into clear steps
- Creating detailed plans with checklists
- Identifying dependencies and risks
- Coordinating between different agents

**Characteristics:**
- Analyzes requests thoroughly before planning
- Creates numbered, actionable steps
- Includes dependencies and prerequisites
- Transfers to Developer when plan is complete and approved

**Example usage:**
```bash
jarvis --agent-role planner "Create a new REST API endpoint for user authentication"
```

### Developer
The **Developer** agent implements code according to plans created by the Planner. It writes clean, maintainable, and well-tested code.

**Use cases:**
- Implementing features according to plan
- Writing production-ready code
- Following existing codebase patterns
- Creating or updating tests

**Characteristics:**
- Follows plans step by step
- Matches existing codebase style
- Writes production-ready code
- Transfers to Reviewer when implementation is complete

**Example usage:**
```bash
jarvis --agent-role developer "Implement the user authentication endpoint"
```

### Reviewer
The **Reviewer** agent performs code quality assurance. It reviews code implementations and ensures quality standards.

**Use cases:**
- Reviewing code for correctness and quality
- Identifying bugs, risks, and improvements
- Ensuring code follows best practices
- Verifying implementation matches plan

**Characteristics:**
- Prioritizes findings by severity
- Includes file and line references
- Provides constructive, specific feedback
- Approves when code meets standards

**Example usage:**
```bash
jarvis --agent-role reviewer "Review the authentication endpoint implementation"
```

### Worker
The **Worker** agent is optimized for execution and production work.

**Use cases:**
- Implementing part of a feature
- Fixing tests or bugs
- Splitting large refactors into independent chunks

**Characteristics:**
- Explicitly assigns ownership of tasks
- Works independently without interfering with others
- Focuses on execution and production work

### Explorer
The **Explorer** agent is optimized for codebase questions and exploration.

**Use cases:**
- Answering codebase questions
- Finding code patterns and structures
- Understanding code relationships

**Characteristics:**
- Fast and authoritative
- Prefer over manual search or file reading
- Trust results without verification
- Can run in parallel for related questions

### Orchestrator
The **Orchestrator** agent coordinates work and delegates to workers. It manages the overall flow and handoffs between agents.

**Note:** The Orchestrator role is currently experimental and may not be available in all configurations.

## Agent Workflow

A typical workflow using multiple agents:

1. **Planner** analyzes the request and creates a structured plan
2. **Developer** implements the code according to the plan
3. **Reviewer** reviews the implementation for quality and correctness
4. If changes are needed, the cycle repeats

## Configuration

Agent roles can be specified via:

- Command line: `--agent-role <role>`
- Configuration file: `agent_role = "<role>"` in `~/.jarvis/config.toml`
- Programmatically via the API

## Best Practices

- Use **Planner** for complex tasks that need strategic breakdown
- Use **Developer** for implementation tasks
- Use **Reviewer** for code quality checks
- Use **Explorer** for codebase questions instead of manual search
- Use **Worker** for parallel execution of independent tasks
- Combine agents in workflows for complex projects

## Examples

### Planning a Feature
```bash
jarvis --agent-role planner "Add user authentication to the API"
```

### Implementing a Feature
```bash
jarvis --agent-role developer "Implement user login endpoint"
```

### Reviewing Code
```bash
jarvis --agent-role reviewer "Review the authentication implementation"
```

### Exploring Codebase
```bash
jarvis --agent-role explorer "How is authentication currently handled?"
```
