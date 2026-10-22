# Security policy

Tessera holds people's private notes, so we treat security reports as the highest-priority work
in the project.

## Reporting a vulnerability

**Do not open a public issue, discussion or pull request for a security problem.**

Report it privately through GitHub's private vulnerability reporting:

1. Go to the repository's **Security** tab.
2. Choose **Report a vulnerability**.
3. Describe the problem, the affected version or commit, and the steps to reproduce it. A
   proof of concept helps but is not required.

Only the maintainers can see the report. We will:

- acknowledge it within **5 working days**;
- tell you whether we can reproduce it, and our planned fix, within **14 days**;
- credit you in the advisory and release notes, unless you ask us not to;
- publish a GitHub Security Advisory (with a CVE where it applies) once a fix is released.

Please give us a reasonable time to fix the problem before you disclose it publicly. We aim to
ship a fix within 90 days and will tell you if we need longer.

## Scope

In scope: the code in this repository, which includes the desktop app, the sync server, the
shared Rust core, the website and the release and CI configuration.

Out of scope: third-party services and dependencies (report those upstream; tell us too if
Tessera is affected), and attacks that need an already-compromised, unlocked device. The
assumptions behind that are in [`docs/threat-model.md`](docs/threat-model.md).

## Supported versions

Tessera has not had a release yet. Until 1.0, only the latest release (and `main`) gets
security fixes.

## Cryptography

Tessera's sync is designed to be end-to-end encrypted, and **none of that encryption is
implemented yet**. The design is in [`docs/threat-model.md`](docs/threat-model.md) and
[`docs/adr/0002-end-to-end-encrypted-sync.md`](docs/adr/0002-end-to-end-encrypted-sync.md). We
welcome review of the design before code exists. Open a discussion for design questions, or use
private reporting if you find a flaw that would matter once the design is built.
