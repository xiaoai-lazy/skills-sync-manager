# Skills Sync Manager Test Checklist

## Windows verification

- [ ] Install a valid skill and confirm target entry is a junction.
  - In PowerShell: `(Get-Item C:\path\to\target\skill-name).Attributes` includes `Directory, ReparsePoint`.
- [ ] Modify `SKILL.md` in the source skill directory and confirm the target sees the change.
- [ ] Uninstall the skill from the target and confirm the source skill directory is untouched.
- [ ] Create a same-name real directory in the target and confirm reinstall is blocked with a conflict error.

## macOS verification

- [ ] Install a valid skill and confirm target entry is a symlink.
  - In Terminal: `ls -l /path/to/target/skill-name` shows `-> /path/to/main/skill-name`.
- [ ] Modify source `SKILL.md` and confirm target sees the change.
- [ ] Uninstall and confirm source is untouched.
- [ ] Create a same-name unknown symlink in the target and confirm install is blocked with a conflict error.

## Linux verification

- [ ] Same as macOS symlink behavior.

## All platforms

- [ ] Close and reopen the app; confirm settings, targets, and installation records persist.
- [ ] Delete a main skill after confirmation; confirm recorded target links are removed before the source skill is deleted.

## Final smoke test

- [ ] Create a temporary main skills directory.
- [ ] Create a valid skill with `SKILL.md` frontmatter containing `name` and `description`.
- [ ] Create an invalid skill missing `description`.
- [ ] Create two temporary target directories.
- [ ] Set the main directory in the app.
- [ ] Add both target directories.
- [ ] Install the valid skill into target A.
- [ ] Install the valid skill into target B.
- [ ] Edit source `SKILL.md`; confirm both targets reflect the change.
- [ ] Uninstall from target A; confirm source and target B remain intact.
- [ ] Create same-name real directory in target A; confirm reinstall is blocked.
- [ ] Delete the main skill after confirmation; confirm recorded links are cleaned.
