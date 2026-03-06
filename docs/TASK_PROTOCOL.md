# TuneFusion Task Protocol

This document defines how the TuneFusion agent team executes work.
All agents MUST follow this protocol.

---

## Prime Directive

Deliver progress as **small, verifiable slices**.

A “slice” is an end-to-end increment that:
- changes as few files as possible
- compiles (or is structurally valid if pre-build)
- includes at least one automated test OR a deterministic verification step
- does not introduce architectural drift

If a request is large, it must be decomposed into slices.

---

## Standard Workflow

All work follows:

planner → architect → deep → coder → tester → critic → main

Notes:
- main orchestrates and integrates
- architect is the authority on boundaries and ADR compliance
- critic is the authority on “should we merge this?”

---

## Roles and Outputs (Contract)

### planner output (required)
Produces a **Task Packet**:

- Goal (1–2 sentences)
- Non-goals (explicit exclusions)
- Scope (what changes)
- Acceptance Criteria (checklist)
- Risks / unknowns
- Slice Plan (ordered list of small slices)
- Suggested agent assignments

Planner must keep slices small enough to complete in one iteration.

---

### architect output (required if any structure changes)
Produces an **Implementation Blueprint**:

- Which modules/crates/packages change (explicit)
- File placement guidance
- Public interfaces / types to add or modify
- Data contracts (DB schema / artifact formats)
- ADR impact statement:
  - “No ADR changes” OR
  - “New ADR required: <title>”

Architect should reject:
- cross-boundary dependencies
- “quick hacks” that break artifact rules
- implicit refactors

---

### deep output (required if uncertainty exists)
Produces a **Research Note**:

- Recommended approach
- 1–3 alternatives + tradeoffs
- Edge cases
- Performance considerations
- Pitfalls to avoid
- Test ideas

If web research is required, deep should include links/citations in the summary.

---

### coder output (required)
Produces an **Implementation Slice**:

- smallest end-to-end change possible
- keeps within assigned boundaries
- avoids refactors unless explicitly approved
- updates docs only if needed

Coder must not:
- change architecture decisions without a new ADR
- overwrite artifacts
- bypass interface boundaries

---

### tester output (required)
Produces **Proof**:

- at least one automated test (preferred)
- or a deterministic verification script
- or a CI-friendly harness step

Tester must confirm:
- artifact outputs are stable and versioned
- new code paths are covered at least minimally

---

### critic output (required)
Produces a **Merge Review**:

- pass/fail recommendation
- list of issues (severity-tagged)
- required fixes vs optional improvements
- follow-up issues to open

Critic MUST check:

- ADR compliance
- artifact pipeline rules
- folder boundaries
- security pitfalls (file writes, injection, unsafe parsing)
- concurrency/locking risks (especially analysis runs + SQLite)

---

### main output (required)
Produces **Integration**:

- resolves conflicts
- ensures repo is consistent
- ensures docs/roadmap updated as needed
- opens GitHub issues for follow-ups
- summarizes what shipped in this slice

---

## Slice Size Rules

A slice must be bounded to one of these:

- add one artifact type end-to-end
- add one UI screen with stubbed backend calls
- add one backend API + one minimal UI caller
- add one DB table + one caller path + one test

If it exceeds that, split it.

---

## Acceptance Criteria Format (required)

Acceptance criteria must be written as a checklist:

- [ ] Condition 1
- [ ] Condition 2
- [ ] Condition 3

Criteria must be testable or at least deterministically verifiable.

---

## Stop Conditions (when to halt and escalate)

Agents must stop and escalate to main if:

- an architectural decision is required
- a change would violate ADRs
- output artifacts would need overwriting
- a cross-boundary dependency is necessary
- tests cannot be made deterministic

Escalation message must include:
- what blocked you
- what you tried
- 2–3 options to proceed

---

## Artifact Pipeline Rules (hard requirements)

- Never overwrite artifacts.
- Every analysis run creates a new run folder/id.
- Record `pipelineVersion` for every run.
- Prefer deterministic processing (same input => same output).
- Realtime processing is optional and must not replace deterministic artifacts.

---

## Git / PR Discipline

- Prefer small commits.
- Avoid “mega commits”.
- Each slice should be PR-able.
- If large refactor is needed: require an ADR or DECISION note first.

---

## Definitions

**Task Packet**: planner’s structured breakdown for a piece of work.  
**Implementation Blueprint**: architect’s constraints + placement + contracts.  
**Research Note**: deep’s research + pitfalls + alternatives.  
**Slice**: smallest shippable, testable increment.

---