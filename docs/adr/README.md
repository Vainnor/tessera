# Architecture decision records

One file per decision that is expensive to reverse: a library at the core, a data format, a
security model. Each record says what was decided, why, and what was given up.

| ADR                                       | Title                                     | Status                                |
| ----------------------------------------- | ----------------------------------------- | ------------------------------------- |
| [0001](0001-repository-and-stack.md)      | Repository layout, stack and dependencies | Accepted                              |
| [0002](0002-end-to-end-encrypted-sync.md) | End-to-end encrypted sync                 | Accepted (direction); design proposed |
| [0003](0003-crdt-library.md)              | CRDT library for note sync                | Accepted                              |
| [0004](0004-server-access.md)             | Who may use a sync server                 | Accepted                              |

Start a new record from [`template.md`](template.md), numbered next in sequence, and add it to
the table. A record is never rewritten after it is accepted: a later decision supersedes it with
a new record, and the old one gets `Superseded by NNNN` in its status line.
