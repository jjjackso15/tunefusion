Agents should read this file before performing repository-wide changes.

Agents must follow docs/TASK_PROTOCOL.md for all work.

# TuneFusion Agent Guidelines

## Product Architecture

TuneFusion is a **desktop-first music practice and analysis tool**.

Technology stack:

* **Desktop framework:** Tauri
* **Frontend:** React + TypeScript
* **Backend:** Rust
* **Storage:** SQLite + filesystem artifacts

### Architectural Principles

TuneFusion follows an **artifact-first analysis architecture**.

Pipeline:

```
Import audio
→ run analysis
→ generate artifacts
→ UI and coaching layers consume artifacts
```

Key rules:

* Analysis outputs are **versioned artifacts**
* Artifacts are **never overwritten**
* New results create a new **analysis_run** with a new `pipelineVersion`

Artifacts include:

* waveform_peaks
* tempo_map
* beat_grid
* pitch_contour
* sections
* chords
* practice_loops
* score_report

### Storage Model

TuneFusion is **local-first**.

Metadata:

* SQLite database

Binary artifacts:

* Stored on disk under project directories

Example structure:

```
app_data/
  projects/
    <projectId>/
      audio/
      artifacts/
      sessions/
```

---

# Repository Layout

```
apps/desktop/        Tauri + React desktop UI
crates/audio_engine/ Low-level audio utilities
crates/analysis/     Analysis pipeline + artifact generation
packages/shared/     Shared types and interfaces
docs/                Architecture, roadmap, ADRs
```

---

# Architecture Governance

Before proposing architectural changes:

1. Read all files in `docs/adr/`
2. Check existing architecture decisions

Rules:

* Accepted ADRs **must not be changed**
* Architectural changes require **architect approval**
* Changes require creating a **new ADR**

---

# Development Priorities

Current order of importance:

1. MVP practice workflow
2. Artifact pipeline stability
3. Coaching layer
4. Competition features

---

# Agent Team Operating Model

TuneFusion uses a **multi-agent development team** coordinated by OpenClaw.

Agents:

| Agent     | Role                           |
| --------- | ------------------------------ |
| main      | orchestrator                   |
| planner   | roadmap / task planning        |
| architect | system design authority        |
| deep      | research and complex reasoning |
| coder     | feature implementation         |
| tester    | automated tests and harnesses  |
| critic    | review and QA                  |

---

# Agent Responsibilities

### main (orchestrator)

* Receives tasks from the user
* Breaks work into smaller steps
* Delegates tasks to the appropriate agents
* Integrates results
* Maintains repo consistency

---

### planner

Responsible for planning work.

Produces:

* milestones
* GitHub issues
* acceptance criteria
* task breakdowns

---

### architect

Responsible for system design.

Defines:

* module boundaries
* data models
* interfaces
* architecture notes

All architectural decisions must align with ADRs.

---

### deep

Research and technical exploration.

Responsibilities:

* evaluate algorithms
* research approaches
* validate assumptions
* provide alternatives and tradeoffs

Outputs:

* research summaries
* design suggestions
* pitfalls and constraints

---

### coder

Responsible for implementation.

Rules:

* implement **small vertical slices**
* follow repository structure
* respect architectural boundaries
* never redesign system architecture

---

### tester

Responsible for testing.

Produces:

* unit tests
* integration tests
* fixtures
* test harnesses

Every feature slice should include **minimal automated tests**.

---

### critic

Responsible for review and quality control.

Checks:

* correctness
* maintainability
* security issues
* performance risks
* ADR compliance
* artifact pipeline rules
* repository folder boundaries

---

# Development Workflow

All work follows this sequence:

```
planner
  → architect
      → deep
          → coder
              → tester
                  → critic
                      → main
```

### Step descriptions

1. **planner**

   * defines tasks and acceptance criteria

2. **architect**

   * defines structure and module placement

3. **deep**

   * researches algorithms or strategies

4. **coder**

   * implements the feature

5. **tester**

   * adds automated tests

6. **critic**

   * reviews the change and identifies issues

7. **main**

   * integrates results and updates documentation

---

# Development Rules

* Prefer **small commits and incremental PRs**
* Avoid large refactors without an ADR
* Maintain deterministic analysis results
* Do not overwrite artifacts
* Every feature should include **basic tests**

---

# Core Product Pipeline

The MVP user workflow is:

```
Import audio
→ Analyze track
→ Generate artifacts
→ Practice loops
→ Score performance
→ Persist results
```

All features should support this pipeline.

---
