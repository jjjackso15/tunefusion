# TuneFusion: AI Handoff Protocol

This document provides instructions for AI agents collaborating on this project.

## Guiding Principles

0.  **Guardrails:** Read and obey `docs/GUARDRAILS.md` (desktop-first, artifact-first, versioned pipeline; no in-place overwrites).
1.  **Documentation First:** Before implementing any change, ensure the relevant documentation in the `/docs` directory is updated or created.
2.  **Source of Truth:** The `docs/REQUIREMENTS.md` file is the single source of truth. Do not infer requirements from conversations.
3.  **Small, Atomic Commits:** Keep commits small and focused on a single logical change.
4.  **PRs for Everything:** All code changes must be submitted via Pull Requests.

## Getting Started
1. **Prerequisites:** Ensure you have the Rust toolchain, Node.js (v18+), and `pnpm` installed.
2. **Clone & Install:**
   ```bash
   git clone <repo-url>
   cd tunefusion
   pnpm install
   ```
3. **Environment Check:** Run `cargo check` and `pnpm --dir apps/desktop exec tauri info` to verify your environment is ready.
4. **Review Docs:** Read `docs/GUARDRAILS.md` and `docs/ARCHITECTURE.md` to understand the artifact-first philosophy.

## Common Workflows

### Running the App
- **Development Mode:** `pnpm --dir apps/desktop exec tauri dev`
- **Build Production:** `pnpm --dir apps/desktop exec tauri build`

### Testing
- **Rust Tests:** `cargo test` (runs all workspace tests).
- **Analysis Crate:** `cargo test -p analysis` (focused algorithm testing).

### Adding a New Analysis Artifact
To add a new analysis type (e.g., `tempo_map`):
1. Define the data structures in `crates/analysis/src/<module>.rs`.
2. Add a new variant to `ArtifactPayload` in `crates/analysis/src/lib.rs`.
3. Implement the analysis function that returns an `ArtifactEnvelope`.
4. Expose a Tauri command in `apps/desktop/src-tauri/src/main.rs`.
5. Update the React frontend in `apps/desktop/src/App.tsx` to call the new command and render the results.

## PR Protocol
- **Branch Naming:** `feature/<name>`, `fix/<name>`, or `docs/<name>`.
- **Atomic Changes:** One PR per logical "slice" (see `docs/TASK_PROTOCOL.md`).
- **Validation:** Every PR must include either a new test case or a documented manual verification step.
