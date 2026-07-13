# Disable Startup Skill Refresh Design

> Date: 2026-07-13
> Status: Approved direction, pending written-spec review

## Goal

Stop automatic Skill discovery and Skill update checks during application startup. Startup must load and display the existing local state and runtime cache without contacting configured Skill repositories or Skill Hub endpoints.

## Scope

- Remove the startup effect that calls `discoverSkills` and `checkSkillUpdates` after `getAppState` succeeds.
- Continue hydrating discoverable Skills and pending updates from the cached values returned in `AppState`.
- Preserve user-triggered discovery, update checks, installation, and update actions.
- Preserve the existing automatic application-version check (`checkAppUpdate`).
- Add or update frontend tests to prove startup does not call the two Skill remote APIs.

## Non-Goals

- No Rust command or update-service refactor.
- No change to repository cache formats, runtime-cache formats, or config migration.
- No settings toggle for automatic Skill checks.
- No delayed, scheduled, or background Skill refresh.
- No code copied or merged from `codex/update-system-refactor`.

## Behavior

On startup, the application calls `getAppState`, renders the cached Skill discovery list and pending update list, and remains offline with respect to Skill sources. Empty caches remain empty rather than being interpreted as a completed remote check.

Remote Skill work starts only from an explicit user action already present in the UI, such as refreshing the Skill Hub or checking updates. Existing error handling for those actions remains unchanged.

The application updater remains independent and may still check the configured application update endpoint after startup.

## Implementation Boundary

Remove the background-discovery and background-update dependencies from `useAppBootstrap` and its caller. Keep the corresponding functions in `useSkillHub` because user-triggered UI flows still use them. Avoid renaming APIs or reorganizing hooks beyond what is required to remove the startup behavior.

## Testing

- Replace the test that expects background discovery and update checks on startup with assertions that neither API is called.
- Verify cached discoverable Skills and cached pending updates still render after startup.
- Run the complete frontend test suite and production build.
- Rust behavior is unchanged; run the Rust suite as a regression check, while treating the known credential-store parallel-test isolation failure separately if it recurs and passes in isolation.

## Success Criteria

- Starting the application never calls `discover_skills` or `check_skill_updates` without user action.
- Cached Skill Hub state remains visible immediately after startup.
- Manual refresh and update workflows continue to work.
- Automatic application update checking continues to work.
- Frontend tests and build pass, with no new Rust regressions.
