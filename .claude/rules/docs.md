# Docs Rules (always loaded)

A doc that disagrees with the code is a bug in the same change. Update in the same commit:

| Change                                                                                                           | Update                                                             |
| ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| New crate, package, module boundary or data flow                                                                 | `docs/architecture.md`                                             |
| New dependency                                                                                                   | `docs/adr/0001-repository-and-stack.md` → "Dependencies"           |
| A decision that is expensive to reverse                                                                          | A new ADR in `docs/adr/` (`/new-adr`)                              |
| New command, capability, CSP source, plugin, file location, import format, or anything crossing a trust boundary | `docs/threat-model.md`                                             |
| Anything about keys, encryption or what the server sees                                                          | `docs/threat-model.md` and ADR 0002                                |
| Theme field, token or preset                                                                                     | `docs/theming.md` (and `theming.md` → "Changing the theme system") |
| A budget or how it is measured                                                                                   | `docs/performance.md`                                              |
| Anything sent off the device, stored, or logged                                                                  | `apps/web/src/pages/privacy.astro`                                 |
| User-visible feature status                                                                                      | `apps/web/src/data/site.ts` and `README.md`                        |
| Commands to build, test or run                                                                                   | `README.md`, `CLAUDE.md` → "Commands"                              |
| Security reporting or supported versions                                                                         | `SECURITY.md`                                                      |

- Docs are for maintainers unless they say otherwise; write for one reader.
- Describe what is true now. Plans go in issues or the roadmap, history in commits. Proposals are
  labeled as proposals (as the threat model is).
- Word choice: `writing.md`.
