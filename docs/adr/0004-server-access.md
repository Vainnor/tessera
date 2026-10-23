# 0004: Who may use a sync server

- Status: Accepted (direction: option C). The protocol details in `docs/threat-model.md` → "Server access" are proposed, like the rest of ADR 0002's design, and go to outside review
- Date: 2026-10-22

## Context

The sync server is self-hostable, and its first deployment is the maintainer's homelab. End-to-end
encryption (ADR 0002) stops a server from reading notes, and the signed device log (threat model,
G4) stops it from adding a device that clients trust. Neither says who may use the server. As the
threat model stood, any client that could reach a server could register an account and upload
encrypted blobs: a stranger could not read or change anyone's notes, but could fill the disk and
use the operator's machine as free storage, and every route was attack surface for anyone on the
network (adversary A8).

The server today answers only `/healthz` on loopback (`crates/server/src/routes.rs`), so nothing
is exposed yet. This decision has to come before the first route that accepts data.

## Options

### A. Network only

Rely on a private network (NetBird, Tailscale, a VPN) or a firewall to keep strangers out, with no
access control in the server.

- Gives: the least code, and private networks are a sound default for a homelab.
- Costs: one firewall or proxy mistake exposes an open server, and a server meant for more than one
  person (sharing, a hosted service later) has no way to say who is admitted.

### B. A single configured owner

The server's config names one account. It never accepts another.

- Gives: simple, and enough for one person.
- Costs: sharing a vault with someone else means editing config on the host, and the server still
  needs device sign-in so that only that owner's devices act as the owner.

### C. Invites, device sign-in and limits

Registration needs a one-time invite the operator creates on the server host. Every other request
comes from a device that signed a server challenge with its device key, checked against the signed
device log, which the server verifies itself. Every route has a body limit, every account a quota,
and the deployment docs recommend a private network.

- Gives: strangers cannot create accounts or reach routes beyond a short, listed set, even when the
  server is exposed by mistake; adding a person is one command; a removed device loses access at
  once.
- Costs: a sign-in protocol the project composes (a standard challenge-response, as SSH public-key
  sign-in uses), so it needs the same outside review as the rest of the crypto design; the server
  now verifies signatures and hashes tokens, through `tessera-core`.

## Decision

Option C. The threat model's "Server access" section holds the proposed details; this record fixes the direction and the rules below.

- Registration requires an unused invite from `tessera-server invite`, which runs on the host and
  never through a network route. Only the invite's SHA-256 hash is stored, it expires after 24
  hours, and it is consumed in the same transaction that creates the account.
- The client pins the server's id from the invite or the pairing code, never from a sign-in
  response.
- A device signs in by signing a single-use server challenge with the Ed25519 half of its device
  key, in a fixed, versioned, labeled encoding. Every kind of message a device key signs has its
  own label.
- The server accepts a device-log or membership-log entry only if its signature verifies against a
  device current at that point in the log, or against the recovery signing key named in the log.
  Removing a device revokes its sessions. The format of the recovery-signed entry is open (see the
  threat model's "Account recovery").
- Without a session, only the routes listed in "Server access" answer: the health check, sign-in,
  registration with an invite, recovery sign-in and the joining side of pairing. Everything else is
  refused before its body is read.
- Body limits on every route, a storage quota per account, rate limits held in memory and never
  logged.
- The server performs exactly three cryptographic operations, all through `tessera-core`: strict
  Ed25519 verification, SHA-256 of random tokens, and OS-CSPRNG draws. It holds no private key
  (`.claude/rules/crypto.md`).
- This record adds SHA-256, for hashing random tokens only, to ADR 0002's primitives, and is the
  ADR `crypto.md` requires for a new protocol: the device sign-in.

## Consequences

- The server will gain a small amount of state: invite hashes, challenges, session hashes and rate-limit
  counters. "What the server inevitably sees" now lists sign-in times per device, invite use and
  storage per account.
- Operators will get a command to hand out invites, and must treat a printed invite like a password
  until it is used or expires.
- The sign-in protocol, server-id pinning, recovery sign-in and server-side log verification go to
  the outside review ADR 0002 already requires before any release that syncs real data.
- Implementation adds rejection tests for each sign-in failure: wrong label, server id, account or
  device; a reused or expired challenge; a removed device; a session used after removal; a forged log entry uploaded under a valid session; a removed device re-adding itself; a recovery-signed entry checked against a key not in the log; a recovery session used beyond its scope; a challenge for an unknown account; an invite used twice.
- Open, in the threat model's question 10: the form of the server id, the default quota, rate-limit
  values, session lifetimes, and whether an invite names its person.

Revisit if Tessera runs a hosted service for the public (registration then needs a policy beyond
invites), or if a standard such as WebAuthn or HTTP Message Signatures (RFC 9421) fits device
sign-in with a reviewed library.
