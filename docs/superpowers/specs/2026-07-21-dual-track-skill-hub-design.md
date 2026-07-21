# Dual-Track Skill Hub: Skills Sync Hub + iFlytek Skill Hub Design

## Goal

Split “Skill Hub” into two independent tracks that share the Skill 中心 UX but use different protocols:

| UI name | Protocol | Config |
|---------|----------|--------|
| **Skills Sync Hub** | Existing legacy client (`/api/v1/groups`, `/skills`, `.../archive`) | Existing `skillHubEndpoints` — **do not migrate or rewrite** |
| **iFlytek Skill Hub** | ClawHub-compatible API (`/.well-known/clawhub.json`, list/download under `/api/v1`) | New `iflytekSkillHubEndpoints` |

Phase 1 delivers dual-track shell + **anonymous** iFlytek discover / preview / install (`@global` PUBLIC), with explicit extension points for Token and team namespaces.

## Decisions Log

| Decision | Choice |
|----------|--------|
| Source management UX | Separate tabs/entries (not one dropdown) |
| Source tree | Fully dual-track roots |
| Existing data | Leave `skillHubEndpoints` unchanged; treat as Skills Sync Hub |
| Config shape | Separate arrays (not `protocol` on old records) |
| Traversal | Separate loops per track — no merged “one Hub client” loop |
| Stats | Separate counts: Skills Sync Hub `n`, iFlytek Skill Hub `m` |
| Phase 1 iFlytek scope | Anonymous list + preview + install to main library |
| Naming | Skills Sync Hub / iFlytek Skill Hub |

## Current Behavior

- Single `SkillHubEndpoint { id, name, baseUrl, enabled }` list.
- Hard-coded legacy HTTP in `skill_hub_client.rs` (no auth).
- Source tree / labels / startup refresh treat all hubs as one “Skill Hub”.
- Discover preview for hub skills downloads archive and reads `SKILL.md`.

## Desired Behavior

### Configuration

- **Skills Sync Hub:** keep using `AppConfig.skillHubEndpoints` as today.
- **iFlytek Skill Hub:** add `AppConfig.iflytekSkillHubEndpoints: Vec<IflytekSkillHubEndpoint>`.

Suggested shape:

```ts
IflytekSkillHubEndpoint {
  id: string
  name: string
  baseUrl: string   // e.g. https://skillhub.xkw.cn
  enabled: boolean
  // Extension (phase 1: no UI required):
  // token linked via credential store by endpointId
}
```

- Missing `iflytekSkillHubEndpoints` on disk → empty list (serde default); existing installs unaffected.
- Do not add required fields to `SkillHubEndpoint`.

### Source management UI

- Two entries/tabs: **Skills Sync Hub** | **iFlytek Skill Hub**.
- Each manages only its own endpoint list (add / enable / remove).
- Rename user-visible “Skill Hub” copy for the legacy track to **Skills Sync Hub**.

### Source tree

```
全部
├── Skills Sync Hub
│     └── {endpoint}
│           └── {group…}
└── iFlytek Skill Hub
      └── {endpoint}
            └── {namespace…}   // phase 1: at least global; structure ready for teams
GitHub / GitLab / 本地 …
```

- Selecting a Skills Sync subtree refreshes **only** that track (legacy client).
- Selecting an iFlytek subtree refreshes **only** that track (ClawHub client).
- 「全部」composes both result sets for display; it does **not** unify protocols into one fetch loop.

### Stats and labels

- Replace combined “N Skill Hubs” style summaries with:
  - Skills Sync Hub: `n`
  - iFlytek Skill Hub: `m`
- Card meta examples:
  - `Skills Sync Hub · {group}`
  - `iFlytek Skill Hub · {namespace}`

### Discover cache

- Each discoverable item must be attributable to a track (`hubKind: 'skillsSync' | 'iflytek'`, or resolvable via which endpoint table owns `hubEndpointId`).
- Refreshing one track updates only that track’s entries; must not wipe the other track’s cache entries.

### Startup refresh

- Prefer eventual split switches: `startupRefresh.skillHub` (Skills Sync) vs `startupRefresh.iflytekSkillHub`.
- Phase 1 may keep a single switch that only refreshes Skills Sync, and add iFlytek switch (or default off) as a small follow-up in the same feature if cheap; design requires the **config shape to allow** independent toggles.

## Phase 1: iFlytek anonymous path

| Capability | Behavior |
|------------|----------|
| Add endpoint | name + baseUrl (e.g. `https://skillhub.xkw.cn`) |
| Discover | `GET {baseUrl}/api/v1/skills` → map `items[]` to `DiscoverableSkill` |
| Preview | `GET .../skills/{namespace}/{slug}/download` (zip) → extract `SKILL.md` (same pattern as legacy hub preview) |
| Install | download zip → extract to main library → write `SkillRecord` with namespace/slug/version |
| Auth | **No** `Authorization` header when token unset |
| Namespace | Phase 1 targets `@global` PUBLIC; tree may show `global` node |

List/detail APIs return **metadata only**; full package is always via download (expected).

### Mapping

| App concept | Skills Sync Hub | iFlytek |
|-------------|-----------------|---------|
| Endpoint table | `skillHubEndpoints` | `iflytekSkillHubEndpoints` |
| Grouping | `group` | `namespace` (phase 1: `global`) |
| Skill id | `hubSkillId` | `slug` |
| Version | hash-oriented today | store `latestVersion` / resolved version on record |
| storageKey | `hub/{endpointId}/{group}/{id}` | `hub/{endpointId}/{namespace}/{slug}` |

`source` may remain `skillhub`; distinguish tracks with `hubKind` (or endpoint-table membership).

## Out of scope (phase 1)

- API Token UI / credential persistence (extension point only)
- Team namespace browse/download
- Publish / reupload to iFlytek
- Device Flow / OAuth in-app
- Calling `@astron-team/skillhub` CLI from the app

## Extension points (team space + Token)

1. Optional token per iFlytek endpoint in credential store (keyed by `endpointId` / registry URL).
2. “Test connection” via `GET /api/v1/whoami` with `Authorization: Bearer …`.
3. With valid token: list/filter non-`global` namespaces; authenticated download.
4. **Critical:** if token is configured but invalid, do **not** send a bad Bearer on public reads (server may 401 and not fall back to anonymous). Validate with whoami; omit header when unset.
5. Scopes (platform): `skill:read` / `skill:publish` / … — document for later publish work.
6. Publish/reupload can later reuse Skills Sync Hub “dirty → reupload” UX against iFlytek publish API.

## Architecture

```
SourceManageDrawer
  ├─ Tab: Skills Sync Hub  → skillHubEndpoints CRUD
  └─ Tab: iFlytek Skill Hub → iflytekSkillHubEndpoints CRUD

SourceTree
  ├─ skillsSync root → legacy refresh only
  └─ iflytek root    → clawhub refresh only

Rust
  ├─ skill_hub_client.rs          (legacy, unchanged contract)
  └─ iflytek_skill_hub_client.rs  (ClawHub list/download/preview helpers)
```

Commands stay separate or take an explicit `hubKind` so callers never accidentally hit the wrong protocol.

## Error handling

| Case | Behavior |
|------|----------|
| iFlytek list/download network error | Track-local toast/warning; other track unaffected |
| Preview/install when zip missing `SKILL.md` | Same validation errors as legacy |
| 401/403 on anonymous global download | Toast; suggest token (future) / visibility |
| Legacy endpoints | Unchanged behavior |

## Testing

- Config: old JSON without `iflytekSkillHubEndpoints` loads; Skills Sync Hub unchanged.
- Source tree: two roots; refresh under one track does not clear the other.
- Stats: separate counts.
- iFlytek: map list item → discoverable; preview uses download; install writes record under `hub/{id}/global/{slug}`.
- No Authorization header on phase-1 iFlytek HTTP calls.
- Labels use Skills Sync Hub / iFlytek Skill Hub copy.

## Reference

- Registry guide: `https://skillhub.xkw.cn/registry/skill.md`
- Discovery: `GET /.well-known/clawhub.json` → `{"apiBase":"/api/v1"}`
- Anonymous verified on xkw: list skills, namespaces, `.../global/{slug}/download` (zip)
