# TuneFusion — Requirements (Living Spec)

**Purpose:** TuneFusion is a practice app that fuses **vocals + guitar** around real songs, with **real-time feedback** and (eventually) competitive/social modes.

This is the **source of truth** for what we’re building. Keep it updated.

## 0) Glossary
- **Target track**: The reference audio (imported song).
- **Pitch trace**: Time-series of detected fundamental frequency (Hz) mapped to note names.
- **Chord timeline**: Time-aligned chord labels (e.g., G, D/F#, Em7) across the song.
- **Segment**: A time range used for looping / slow practice.

## 1) Product Goals
1. Make practicing **singing + guitar** feel like a game (SingStar / Rocksmith vibes) while still being genuinely instructional.
2. Let users practice with **their own songs** (import audio) and see **vocal notes + chord changes** in sync.
3. Provide **immediate feedback** (pitch accuracy + timing) and **progress tracking**.
4. Support **multiple users** and a **competition mode** (local first; online later).
5. Add a fun “**radio DJ**” style narration/coach mode to keep practice engaging.

## 2) Non-Goals (for MVP)
- Building a full music streaming service.
- Hosting/distributing copyrighted songs.
- Perfect, studio-grade transcription of every song.
- Automatic guitar-note transcription (single-note guitar melody extraction) in MVP.

## 3) Target Users / Use Cases
### U1) Solo practice
- Import a song.
- Practice vocals with on-screen pitch lanes.
- Practice guitar with chord prompts.
- Loop difficult sections and slow down.

### U2) Duo / household
- Separate profiles for John/Cathy/kids/guests.
- Each user has their own scores and progress.

### U3) Competition
- Two users compete on the same song.
- Real-time side-by-side feedback + final winner.

## 4) Functional Requirements

### FR-1: Song Import
- Allow importing audio files at minimum: **MP3, WAV**.
- Store metadata:
  - title (user-provided)
  - artist (optional)
  - duration
  - file path / id
  - import timestamp

### FR-2: Song Analysis (offline or background job)
When a song is imported, TuneFusion should analyze it to produce practice assets.

Minimum outputs:
- **Vocal pitch trace** (time → note/pitch)
- **Chord timeline** (time → chord label)

Nice-to-have outputs:
- Estimated tempo (BPM)
- Beat grid
- Key signature estimate
- Section markers (verse/chorus) if feasible

Implementation note (not binding): likely uses source separation + pitch detection + chord recognition.

### FR-3: Vocal Visualization (SingStar-style)
- Display a scrolling timeline of target notes:
  - pitch on Y axis
  - time on X axis
  - note duration as horizontal segments
- Highlight “current note” region.
- Provide clear visual cue when the user is sharp/flat.

### FR-4: Real-time Vocal Analysis
- Listen to microphone input during playback.
- Compute user pitch in near real-time.
- Compare to target pitch trace.
- Provide:
  - **instant feedback** (sharp/flat + how far)
  - **timing feedback** (early/late)

### FR-5: Scoring & Feedback
- During a performance, compute a running score.
- After a performance, show a breakdown:
  - overall score
  - pitch accuracy score
  - timing score
  - “best streak” (optional)
  - trouble spots (time ranges with low accuracy)
- Store results per user + per song.

### FR-6: Chord Display (Guitar mode)
- Show current chord + upcoming chord(s) aligned to the song timeline.
- Optional display modes:
  - chord names only
  - chord diagram (fretboard) for common guitar chords
- Provide a “count-in” or visual pre-roll so a player can get ready.

### FR-7: Practice Tools
- **Loop a segment** (A–B repeat).
- **Playback speed control** (e.g., 50%–100%) with pitch-corrected time-stretch if possible.
- “Jump to trouble spot” based on performance history.
- Optional: transposition (change key) for easier vocal range.

### FR-8: Multi-User Profiles
- Support multiple profiles on a single device.
- For each profile store:
  - display name
  - avatar (optional)
  - history of sessions
  - per-song best scores
  - preferences (difficulty, UI layout, DJ voice on/off)

Auth:
- MVP can be local-only profiles.
- Later: login + sync across devices.

### FR-9: Competition Mode
Modes:
- Local “two players”:
  - turn-based OR simultaneous
  - side-by-side scoring
- Output:
  - winner + detailed comparison

Online competition (stretch):
- real-time remote match (would require realtime transport + latency handling)

### FR-10: DJ-Style Narration / Coach
- Optional narration voice that:
  - introduces the session
  - calls out upcoming challenging parts
  - provides end-of-song recap
- Must be non-annoying:
  - adjustable frequency
  - mute toggle

### FR-11: Recording & Playback
- Allow user to record vocals (and optional guitar via mic) during practice.
- Allow playback of recorded take with overlay of pitch accuracy.
- Storage controls:
  - delete recordings
  - storage location/limits

### FR-12: Community / Sharing (stretch)
- Share recordings/scores (private link or friends).
- Leaderboards (global or friends).
- Content moderation requirements if public.

## 5) Data Requirements (High-Level)
Entities (minimum):
- UserProfile
- Song
- SongAnalysisAsset (pitch trace, chord timeline, tempo)
- PracticeSession
- PerformanceScore
- Recording (optional)

## 6) Non-Functional Requirements
### NFR-1: Latency
- Real-time pitch feedback should feel responsive.
  - Target: < 100ms end-to-end for pitch feedback display (best-effort).

### NFR-2: Reliability
- Analysis jobs should be resumable (don’t corrupt song library on crash).

### NFR-3: Privacy
- MVP: keep recordings/local data local by default.
- If cloud sync exists, clearly indicate what uploads.

### NFR-4: Copyright / Licensing
- Avoid redistributing copyrighted songs.
- Be explicit: user imports their own audio.
- Any future sharing features must consider copyright implications.

### NFR-5: Cross-platform (TBD)
- TBD: web app vs desktop vs mobile.

## 7) Acceptance Criteria (MVP)
A release is “MVP complete” when:
1. User can import an MP3/WAV.
2. App generates (even if imperfect) a vocal pitch trace + chord timeline.
3. During playback, app shows SingStar-like pitch lanes and detects user pitch.
4. App provides a score + post-session breakdown.
5. App supports at least 2 local user profiles.
6. Docs are up to date (README + REQUIREMENTS).

## 8) Open Questions
1. Target platform: web, desktop (Electron), mobile?
2. Do we require offline-only operation for MVP?
3. How “accurate” do chords need to be for MVP (triads only? 7ths? slash chords?)
4. Do we want separate modes: **Vocal**, **Guitar**, **Both**?
5. Does competition mode start as turn-based only?
