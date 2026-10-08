# Proposal: reusable remote terminal component

Status: draft for design review. This document proposes an independent Rust component and its Combe adapter; it does not enable remote access or replace the approved Combe design. Source inspection used Combe `bf1520fec69f1b29b83123701e57cd7c8c17cd0e` and vendored Ghostty `82232ecde55405559dec29c5466cb9e39938cb41`. External documentation was checked on 2026-10-08.

## Recommendation

Build a narrow PTY/session component, reuse `iroh` for authenticated transport, and keep Combe responsible for its existing workspace, tab, split, and terminal UI. Ship a user service, a raw-stdio bridge for each pane, a control SDK, and prebuilt relay deployment assets. Do not implement another terminal parser or renderer.

Two feasibility gates precede implementation: correct terminal recovery through real Ghostty surfaces, and an automated user-owned relay deployment within a cost boundary still to be agreed. Neither gate has passed. Keeping a remote process alive is straightforward; restoring its terminal without losing state or repeating effects is the decisive difficulty.

The strongest simpler session alternative is `shpool` plus a relay agent. Its default restoration loses parser continuation and inactive-screen state in a reproducible synthetic probe. Keeping its original attachment alive avoids that restoration during an ordinary network outage, so the alternative remains credible for that narrower case. A new surface, agent restart, or exhausted output window still requires a complete recovery mechanism.

## Required user outcome

- Enable the capability through Combe CLI and bind another host once. Do not require SSH keys, SSH server setup, port forwarding, or user-developed relay code.
- Selecting a host establishes the target for the existing worktree list, new tabs, and splits. Everyday terminal actions retain their existing meaning and shortcuts.
- Each new pane creates its own remote PTY and shell. A split inherits the remote host and current directory.
- Network disconnects preserve the original remote process. Reconnection attaches to that process; it must not rerun the original command as a substitute.
- The component is reusable by other terminal applications and may live in a separate Rust open-source project.
- Provide native service installation and prebuilt user-owned relay deployment. Cloudflare is a deployment candidate, not a mandatory platform.

Pairing replaces manual SSH authentication with device authentication; it does not remove authentication. Initial installation must be authorized locally on each machine. Without an existing authorized entrypoint, the component cannot install itself on an inaccessible host.

Command names below are interface proposals, not existing Combe commands or approved UI copy. Binding introduces a host selector; the target is unchanged everyday terminal semantics, not identical network timing or zero new setup steps.

## What a remote terminal consists of

| Layer | Responsibility | Existing capability | Component responsibility |
| --- | --- | --- | --- |
| Terminal emulation | Interpret characters, cursor movement, modes, screens, history; render locally | Full `libghostty` surface | Preserve the renderer and supply a verified recovery adapter |
| Local bridge | Connect the surface's local PTY to remote bytes | Ghostty can spawn a local command | Raw mode, no echo, binary-transparent stdin/stdout, resize forwarding; no logs on stdout |
| Transport | Move input/output, reconnect, control queue growth | No remote transport in Combe | Reuse iroh; add application session offsets, acknowledgements, deduplication and ownership |
| Remote PTY | Spawn shell with cwd, environment and size; retain the process | Ghostty currently does this locally | Daemon owns the PTY independently of network connections |
| Resize and signals | Deliver terminal size and interactive signals | Local PTY semantics | Convert `SIGWINCH` into ordered resize requests; Ctrl-C normally remains PTY input semantics |
| Trust | Authenticate a device and authorize execution | Existing SSH URL only prepares a shell line | Pair and pin device identities; reject unknown or revoked devices |
| Persistence | Reattach to the original session | Local surfaces follow the app lifecycle | Separate create, attach, detach and terminate |
| Roaming and latency | Reconnect after sleep or network changes | Local operations have no network delay | Reuse transport path management; do not add speculative keystroke prediction |

A remote PTY makes a program believe it is attached to an interactive terminal. Its output is still a byte stream; Ghostty turns those bytes into the local terminal state. Resize is a control message, not text. A program's terminal query must receive one appropriate reply, even across disconnect and recovery.

## Architecture and ownership

```mermaid
flowchart LR
    App[Combe: host, workspace, tabs, splits] -->|control SDK| Local[Local user daemon]
    Surface[Ghostty surface per pane] -->|local PTY| Bridge[Raw-stdio bridge per pane]
    Bridge -->|user socket| Local
    Local <-->|iroh direct or encrypted relay path| Remote[Remote user daemon]
    Local <-.-> Relay[User-owned official iroh relay]
    Relay <-.-> Remote
    Remote --> PTY[PTY and original shell/process]
```

| Owner | Owns | Does not own |
| --- | --- | --- |
| Rust SDK | Typed host/session operations, events and errors | AppKit, layout, Git parsing |
| Local daemon | Device identity, binding, connections, recovery relationships, pending termination/revocation | Combe layout and workspace ordering |
| Remote daemon | PTY/process, session identity, output sequence, terminal recovery state, cwd/busy/exit | Editor, file synchronization, tabs or window UI |
| Bridge | Local TTY mode, bytes and size forwarding | Authentication UI, recovery algorithm, SSH subprocess |
| Relay and installer | Encrypted routing, admission, deployment and owned-resource cleanup | PTY, terminal plaintext, command execution, terminal recordings |
| Combe adapter | Host selection, catalog, registration, tabs/splits, close confirmation, Ghostty surfaces | Cryptography, connection retries, platform service installation |

One release binary may expose service, bridge and setup subcommands. Both hosts may run the same daemon with client and host roles. Each remote PTY has one owner. Local sockets must restrict access to the current OS user; SDK control is not a public network listener.

The local service has a concrete purpose beyond connection sharing: after an offline Quit, Combe is gone, but a confirmed termination request still needs delivery when connectivity returns. The simpler alternative stores intent in the app and sends it on its next launch. That leaves the process running indefinitely if the app is never reopened. Prefer the local service to preserve close semantics; it does not persist layout.

The project remains Rust at its core. Small Cloudflare Worker entrypoint glue may be TypeScript, shipped as prebuilt assets inside the installer. Requiring Rust/Wasm for that glue would add a toolchain without reducing the user's setup work.

## Public interface sketch

These are component responsibilities, not implemented APIs or invented Ghostty bindings. Terminal bytes and lifecycle/control events use separate channels.

| Type or operation | Contract |
| --- | --- |
| `HostId` | Stable public-key identity of the daemon installation for one OS user; aliases are display names, not authorization |
| `RemoteLocation` | `{ host: HostId, path: RemotePath }`; explicit path encoding, no local filesystem validation |
| `SessionRef` | `{ host, id, generation }`; generation prevents stale operations targeting replacement processes |
| `OwnerId`, `RunId`, `PaneKey` | Stable application installation/binding, current app run, and pane recovery relationship; no layout tree |
| `bind(invite)`, `revoke(host)` | One-time authenticated pairing; revocation invalidates existing and future access |
| `hosts()`, `watch_hosts()` | Bound devices and connection state |
| `resolve_location(host, path)`, `stat(host, path)` | Bounded directory facts for registration and cleanup; no general file download API |
| `run(host, argv, cwd, limits)` | Authorized non-PTY execution with separate stdout/stderr/exit, deadline and output bounds |
| `create(owner, run, pane, location, shell, size, request_id)` | Idempotent creation of one PTY, returning `SessionRef`; structured parameters, not shell concatenation |
| `cancel_create(owner, run, request_id)` | Cancel a pane still being created; late replies cannot leave an orphan shell |
| `attach(session, resume_token, renderer_caps)` | Return a bridge launch description and recovery requirements; never silently create a new session |
| `resize(session, size, revision)` | Ordered dimensions with a defined recovery boundary |
| `watch_session(session)` | Versioned cwd, busy, exit, connection and recovery events |
| `sessions(owner)` | Authorized sessions with original location, pane/run identity and attachment state |
| `detach(attachment)`, `terminate(session, request_id)` | Detach preserves the process; termination explicitly targets its exact generation |
| `recover(owner, pane)` | Return the original session or `Lost`/`Exited`; never rerun a command |
| `owner_channel(owner, message)` | Authenticated, bounded opaque application messages; Combe interprets CLI/Hop semantics |

`run` is user-account execution, not a read-only Git sandbox. The terminal already allows arbitrary commands under that account. Git-specific RPC would add Git ownership to a reusable terminal component without removing that authority. Every execution entrypoint still requires device and owner authorization, limits and error handling; no root escalation or arbitrary user switching is provided.

Ghostty's command string contains only a correctly quoted trusted bridge executable path. An opaque, non-secret attachment identifier can use `env_vars`. User host/path/command values are not interpolated into that shell string. The surface uses a valid local working directory; the daemon receives the remote cwd separately.

## Session lifecycle

| Event | Proposed behavior | Boundary |
| --- | --- | --- |
| Network loss, sleep or relay restart | Keep remote process and pane; reconnect to the same session | Socket EOF is not user termination |
| Host switch | Preserve panes on both hosts | Do not respawn on selection |
| New tab or split | Create an independent PTY on the selected host | Split inherits typed host/cwd |
| Confirmed pane/workspace close or normal Quit | End sessions currently owned by that app run | Offline termination remains pending until daemon acknowledgement |
| App/bridge crash | Preserve remote process and authorized recovery relationship | Do not treat crash as confirmed close |
| Remote shell exits | Report the real exit and apply Combe's existing pane behavior | Bridge exit is not remote shell exit |
| Remote host reboot or PTY-owner crash | Report the session lost/exited | Service restart cannot resurrect its original PID |
| Daemon update | Delay replacing a PTY owner while sessions are alive | Transparent live upgrades are outside the first version |

Normal Quit ending sessions is the proposal's interpretation of unchanged Combe semantics. Keeping processes after normal Quit would be a separate product change, not an implicit consequence of network persistence.

Allocate a create request ID before sending; retain its result so a lost reply cannot create two shells. Output carries session generation and byte offset. Input uses writer epoch and sequence; accepted input is deduplicated within a defined ledger window. Once that window expires, return `UnknownOutcome`/`Expired`, rather than executing again. Exactly-once execution across a daemon crash is not promised.

Each session initially has one writer. Reattachment fences the old writer; delayed input, close and resize from the old attachment cannot affect the new one. Offline user input is disabled rather than queued for surprising later execution. Status travels out of band; inserting “reconnecting” into stdout can corrupt a partial UTF-8 or escape sequence. Do not use Ghostty readonly mode blindly: it also changes close-confirmation behavior.

Persist close intent before removing an offline pane. Pending creation is cancelled by request ID even if no `SessionRef` has arrived. Before the remote acknowledgement, do not claim the process has terminated. Normal Quit targets the current run's owned sessions, not every session belonging to the device.

Proposed crash adoption: a new run queries the same stable `OwnerId`, adopts unclosed sessions without a valid attachment, and opens a new tab in the original workspace. Missing catalog entries fall under that host's Home while retaining the original session cwd. Do not restore the old split tree, order or zoom, steal another live attachment, or adopt a pending termination. This visible recovery behavior needs prototype review before implementation.

## Terminal recovery: the unresolved gate

| Recovery path | Necessary conditions | Required action |
| --- | --- | --- |
| Existing surface and bridge | Original parser state survives; missing output remains continuous | Append only the missing suffix, preserve order and resize boundaries |
| New surface or missing output outside the log window | Original renderer state is gone or bytes have been discarded | Restore an internally consistent checkpoint, then its output suffix, before accepting input |

A transport ACK is not proof that Ghostty parsed output. A bridge's successful PTY write is not a renderer-consumption ACK. A surviving bridge can preserve ordered delivery to the same surface; a rebuilt bridge/surface cannot reuse its offset as proof of restored state.

Investigate `libghostty-vt` as a state mirror, not a second handwritten parser. Enable continuation tracking before the first input byte. Checkpoint requirements include both screens, bounded history, cursor, modes, tab stops, scroll regions, palette, hyperlinks, incomplete VT/UTF-8 input, size, output offset and version.

The current binary snapshot API belongs to an independent VT object; full surfaces have no public snapshot import. Snapshot v1 also omits Kitty image/placement state and resets presentation state such as selection, viewport and search. The public formatter emits the active screen; cell hyperlinks are represented in HTML, not a complete raw-VT reconstruction. It cannot be called a full surface snapshot. See [snapshot API](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/include/ghostty/vt/snapshot.h), [continuation API](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/include/ghostty/vt/terminal.h), [snapshot omissions](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/src/terminal/snapshot/terminal.zig#L230), and [formatter](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/src/terminal/formatter.zig#L486).

First test public formatter + continuation + necessary metadata against the real surface. If a demonstrated difference cannot be repaired through public APIs, propose the smallest upstream recovery interface. Do not assume a fork is necessary, invent bindings, or promise recovery because the daemon can save a snapshot.

Terminal query replies and effects need an explicit recovery policy. Simply switching between mirror and surface replies does not prevent a daemon-answered query from being answered again during replay. Bridge stdin cannot reliably distinguish user input from terminal-generated replies. Already delivered OSC 52 clipboard writes, notifications, bells and queries must not repeat. Newly produced effects during disconnection may arrive late; that is a separate issue. Questions that cannot be answered while detached must remain available for the next authorized attachment.

Unlimited raw output, finite resources and complete recovery cannot all be guaranteed. Use checkpoints plus a bounded suffix, targeting Combe's existing 10,000-line history. Eviction may remove history under that contract; it must not corrupt the current terminal or end the process. An exhausted suffix without a valid checkpoint is a visible recovery failure, not success. Validate lengths, integrity and versions before applying state; CRC is not authentication.

Same-surface reconnection must preserve its existing presentation state. A fresh app does not acquire new promises to persist selection or layout, but it still needs a correct terminal state for the original process. G1 must distinguish those contracts.

## Transport and deployment

Prefer pinned `iroh 1.3.0`, its Minimal preset, persisted identity, a custom relay and explicit `EndpointAddr`. Do not default to N0 public test relays or discovery. Address tickets locate devices; they do not authorize execution. Reuse device-authenticated transport rather than designing another cryptographic protocol. A new connection does not restore old streams: session sequencing and resume remain component responsibilities. See [release](https://github.com/n0-computer/iroh/releases/tag/v1.3.0), [presets](https://github.com/n0-computer/iroh/blob/v1.3.0/iroh/src/endpoint/presets.rs), and [self-hosted relay](https://docs.iroh.computer/iroh-services/relays/self-hosted).

Pin both device identities during pairing. Unknown devices may enter only a constrained pairing exchange. Disable early execution; deployment authorization, relay admission and daemon execution authorization remain separate. Replace the official relay's default permissive admission policy. Revocation must reject both direct and relay paths and invalidate existing connections. Merely setting an unimplemented limiter field is not protection. See [relay configuration](https://github.com/n0-computer/iroh/blob/v1.3.0/iroh-relay/README.md).

Both hosts connect outward. Direct paths are optional; acceptance must include UDP blocked and encrypted traffic through the WSS relay. Do not require public SSH ports or claim that every network permits direct connectivity.

### Preferred deployment candidate: official relay in Cloudflare Containers

Ship a fixed official Rust relay image and prebuilt Worker/DO entrypoint. CLI setup uses browser OAuth, creates its owned resources, verifies the workers.dev WSS endpoint and tests both devices. Users should not need Node, Docker, GitHub, DNS configuration or a VPS tutorial. Device admission and its updates must also be automatic.

Cloudflare supports container WebSocket upgrade and linux/amd64 images. Its default scheduling policy can reference a public Docker Hub image; the `durable_object` policy requires an image in the user's Cloudflare registry pinned by digest. A default-policy candidate must route both devices to the same relay instance. Relay challenge, subprotocol, binary frames, headers, image availability and instance routing are **unverified in a deployed system**. See [WebSocket example](https://developers.cloudflare.com/containers/examples/websocket/), [image management](https://developers.cloudflare.com/containers/guides/image-management/), and [application API](https://developers.cloudflare.com/api/resources/containers/subresources/applications/methods/create/).

This is not a Worker-only deployment. Containers require Workers Paid, with a $5/month base subscription plus applicable container, Worker/DO and traffic usage. Open sockets can keep resources active; do not promise both permanent connections and free hibernation. Container restart can disconnect the relay while the PTY remains on the user's host. Billing, permissions and resource budgets require an actual G3 check; installation must not activate billing automatically. See [pricing](https://developers.cloudflare.com/containers/platform/pricing/).

### Strongest transport alternative: Worker + DO WSS relay

A prebuilt Worker/DO relay avoids the Container/Paid prerequisite, but the project owns more routing, authentication composition and backpressure code. Use endpoint-terminated `rustls` TLS 1.3 inside WSS, mutual device authentication and public-key pinning; disable 0-RTT execution. Custom certificate verification must still validate handshake signatures. Outer TLS to Cloudflare alone is not end-to-end device trust. [Rustls](https://github.com/rustls/rustls) supplies the TLS implementation.

Native CLI Authorization Code + PKCE can upload prebuilt Worker assets through Cloudflare APIs. Third-party OAuth supports public native clients without an embedded client secret; third-party device flow is not equivalent to Wrangler's login flow. The publisher must register a public OAuth client and verify its domain. Loopback redirect behavior, exact scopes and the full account/upload flow remain untested. See [OAuth client configuration](https://developers.cloudflare.com/fundamentals/oauth/create-an-oauth-client/) and [Worker upload](https://developers.cloudflare.com/api/resources/workers/subresources/scripts/methods/update/).

Prefer declarative DO class exports for a new deployment; do not mix them with migrations. Read back owned resource IDs, bindings and endpoint. Refuse to overwrite unknown resources with the same name. Keep deployment tokens away from relay and peer devices; prefer revoking the deployment grant after installation and authorizing updates separately, subject to the OAuth probe. See [DO declarations](https://developers.cloudflare.com/durable-objects/reference/durable-objects-migrations/) and [Workers authorization](https://developers.cloudflare.com/workers/authorization/workers/).

Incoming WebSockets can use DO hibernation, but deployments and platform events can close them. Reconstruct ephemeral routing after wake; socket attachments are not durable device trust. Bound queues and frame sizes, separate control/data budgets, and schedule panes fairly. A free plan has finite limits; it is not a promise of free unlimited terminal use. See [WebSocket guidance](https://developers.cloudflare.com/durable-objects/best-practices/websockets/) and [DO pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/).

**Deciding fact:** can the official relay be deployed automatically within the required setup effort and an operating cost acceptable to users? The cost boundary is not yet set. Prefer iroh if yes; choose the Worker-only alternative if that boundary requires it. Validate first, then ship one backend. Do not build both or quietly replace the target with manual VPS setup.

## Proposed enable and bind flow

1. On host A, `combe remote enable` calls component setup and installs the current-user service without editing plist files manually.
2. If no relay exists, browser authorization deploys the prebuilt owned resources and verifies connectivity. An existing trusted endpoint may be reused.
3. Host A produces a short-lived, single-use pairing invitation. Host B installs the component and confirms binding locally using the invitation.
4. An authenticated exchange binds both public keys; the relay cannot substitute a peer. Persist the binding only after verification.
5. Combe receives a host-available event. Each host has its own Home workspace, so a newly bound host can open a terminal before registering a repo; do not scan its disk.

Remote registration is part of a usable first version: a candidate `combe add --host <alias> <path>` validates the path remotely and updates the owning Combe catalog. Daily actions then follow the selected host. Keep NSOpenPanel local; do not introduce a remote file browser. Setup and registration wording require later prototype review.

## Persistent state and security boundary

| State | Owner | Loss or recovery meaning |
| --- | --- | --- |
| Registered repo location | Combe: `Local(path)` or `Remote(host_id, path)` | Identical paths on different hosts must remain distinct; preserve old local registrations |
| Device private key, bound identities, relay endpoint | Component: Keychain and current-user-restricted state | Identity loss requires re-pairing, not automatic trust downgrade |
| Session to PTY/process mapping | Remote daemon live state | Disk metadata cannot resurrect a PTY after owner crash or reboot |
| Checkpoint and bounded output suffix | PTY owner/recovery adapter | Rebuildable recovery state; losing it can prevent a fresh surface from reattaching correctly |
| Owner/pane recovery and pending close/revoke | Local component durable state | Loss can strand processes or misdirect recovery; never guess another owner |
| Last successful catalog | Combe cache | May be shown stale while offline; do not delete registrations because of local path checks |
| Relay routing/admission metadata | User-owned relay | Endpoint device trust remains authoritative; routing loss does not create new trust |

The relay can observe connection metadata, timing and volume, or deny service; it must not read terminal content, alter authenticated commands or impersonate a bound device. A compromised endpoint user account is outside this protection. Revocation is pending until the target daemon applies it; revoking deployment OAuth is not device revocation and cannot undo executed commands. Do not log keys, tokens, raw terminal content or unrelated private data.

Propose current-user LaunchAgent on macOS, without root. This covers logged-in use, not execution before login or after logout. Logout survival would require another service account, key-unlock and permission design. Stop/uninstall must account for live sessions, remove only component-owned resources, and preserve repositories, unrelated shell configuration and other cloud projects.

## Combe integration evidence

The full surface has no public custom output backend, PTY FD injection or snapshot import. Its backend is `exec` and it opens a local PTY. `ghostty_surface_text` sends paste/input to the child; it does not feed renderer output. Therefore a remote full surface currently requires a local process such as this bridge. See [surface config and callbacks](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/include/ghostty.h), [backend](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/src/termio/backend.zig#L14), [Exec creation](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/src/Surface.zig#L659), and [input entrypoint](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/src/apprt/embedded.zig#L2067).

| Current path at inspected Combe revision | Observed behavior | Required adapter change |
| --- | --- | --- |
| `crates/combe/src/surface.rs:455` | Local cwd and initial input; no command override | Typed local/remote target; trusted bridge command |
| `crates/combe/src/entry.rs:68`, `app.rs:176` | `ssh:` URL becomes a confirmed line pasted into a local shell | Preserve existing confirmation; not a binding/session mechanism |
| `crates/combe/src/window.rs:583`, `split.rs:112` | New split inherits `view.cwd` | Inherit host/location and create a separate remote PTY |
| `crates/combe/src/tabs.rs:9`, `:41` | String workspace keys | Include host identity to avoid same-path collisions |
| `crates/combe/src/window.rs:423`, `:441` | Close confirmation uses Ghostty | Remote prompt/busy information plus stale/unknown handling |
| `crates/combe-catalog/src/scan.rs:19`, `:50` | Local validation, Git execution and canonicalization | Execute structured argv remotely; reuse porcelain parser |
| `crates/combe-catalog/src/lib.rs:57`, `:105` | Load can fail as a whole; cleanup checks local directory existence | Isolate offline/error entries; do not delete remote registrations |
| `crates/combe-catalog/src/store.rs:21` | Repo is a `PathBuf` | Migrate registration to typed location |
| `crates/combe/src/ghostty.rs:211` | PWD action updates cwd | Separate local PWD from authenticated remote cwd events |
| `crates/combe/src/ghostty.rs:348` | Allows http/https/mailto links | Preserve restriction; do not map a remote file URL to a local path |
| `crates/combe/src/habits.rs:88`, `:93` | Shell integration and 10,000-line history | Verify remote terminfo/integration and bounded recovery |

Run remote `git worktree list --porcelain` through non-PTY `run`, separating stdout/stderr/exit; interactive prompts and profiles must not pollute the result. Keep the Git 2.25 floor and validate special-character paths against the existing parser. Distinguish host offline, Git missing, timeout and invalid repo.

Ghostty rejects OSC 7 with a different hostname, so transparent SSH-style bytes are insufficient for remote cwd inheritance. Report cwd over the authenticated control channel; do not rewrite it as a misleading localhost file URL. Prompt status uses OSC 133 rather than a foreground PID check; no integration causes false busy and disconnection can leave stale state. See [OSC 7 handling](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/src/termio/stream_handler.zig#L1519) and [quit confirmation](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/src/Surface.zig#L951).

Ghostty's embedded command path forces wait-after-command. Bridge failure or remote natural exit must be handled through control events; setting `wait_after_command=false` is insufficient. Combe currently does not handle `SHOW_CHILD_EXITED`. Distinguish these events so a remote pane does not introduce a spurious “Press any key to close” state. See [command override](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/src/apprt/embedded.zig#L568) and [child exit](https://github.com/ghostty-org/ghostty/blob/82232ecde55405559dec29c5466cb9e39938cb41/src/Surface.zig#L1249).

Preserve CLI Hop: outside Combe, route the path to its registered workspace or that host's Home without registering it; inside Combe, Hop remains a no-op. The remote adapter needs authenticated owner and ancestry markers, not the local app PID copied to another machine. Existing catalog commands remain usable and updates route to the owning Combe. The component forwards opaque owner messages; Combe retains parsing and confirmation.

Quota keeps its existing local Claude/Codex snapshot meaning and does not switch with host selection. Remote quota providers and cross-host aggregation are outside this proposal. Combe telemetry remains local and default-off; any future host/location identity must exclude raw paths and secrets and distinguish remote exit from bridge exit. The component adds no business telemetry.

## Contracts that implementation would change

This draft leaves [AGENTS.md](../../AGENTS.md), [CONTEXT.md](../../CONTEXT.md), [DESIGN.md](../DESIGN.md), and [the prototype](../design.html) unchanged. Approval of a proposal is distinct from implementation and deployment authorization. Before changing behavior, reconcile the following known conflicts and definitions requiring host scope, and update the affected canonical prototype/design together:

| Contract | Current text | Proposed future change |
| --- | --- | --- |
| `AGENTS.md`, Scope | “Do not add agents, an editor, a browser, an SSH client or any other remote transport, a settings GUI, a theme system, a command palette, or cloud sync.” | Permit this remote component adapter; retain the other exclusions |
| `AGENTS.md`, Stack | “Persist only repo paths. Every other preference is a constant in `crates/combe/src/habits.rs`.” | Store typed repo locations; component owns mandatory identity/recovery state separately |
| `docs/DESIGN.md`, Non-goals | “An SSH client, WSL, remote hosts, a PTY daemon that survives app updates” | Permit bound remote hosts and service-owned sessions; define update limits explicitly |
| `docs/DESIGN.md`, state table | “Registered repo paths. Nothing else persists.” | Distinguish repo locations from required component security/lifecycle state |
| `docs/DESIGN.md`, external command entry | “Never use it as the surface command.” | Retain the rule for external user commands; specify a trusted internal bridge exception |
| `CONTEXT.md`, Home workspace; `docs/DESIGN.md`, Workspaces | Home is based on `$HOME`; “Home is not registered or persisted.” | Apply the same meaning per host; continue injecting Home without registration or persistence |
| `CONTEXT.md`, Catalog | “Missing registered paths are skipped.” | Distinguish a confirmed missing remote path from an unreachable host; offline cached rows may remain stale, while cleanup requires facts from that host |
| `docs/DESIGN.md`, Hop nest mark | `COMBE=<app pid>` and local process ancestry | Preserve Hop's no-op behavior inside Combe using authenticated remote owner/ancestry information; retain the local mechanism |

Do not turn host binding into remote file browsing, broaden external-command execution, or relax the requirement that each leaf is a full Ghostty surface. Line references above describe the inspected revision. Host-scoped Home preserves its current registration/persistence rules; retaining an offline registration is not the same operation as displaying its cached catalog row.

## Options and the strongest alternative

Estimates describe integration scale, not a schedule or a measured implementation. Recovery and deployment verification dominate the uncertainty.

| Option | Package and mechanism | Remote prerequisites, trust and persistence | Contract/maintenance cost |
| --- | --- | --- | --- |
| A: system OpenSSH | Wrapper and catalog adapter, roughly hundreds of lines; bridge command is `ssh -t` with correctly quoted remote `cd`/`exec`; catalog uses non-PTY SSH | SSH server, config, keys and host keys; ControlMaster/ControlPersist share connections, not PTY survival | Smallest integration, but violates the required no-SSH onboarding and does not preserve sessions by itself |
| B: SSH + tmux/zellij/dtach/abduco or mosh | Wrapper plus installed session tool; hundreds to low thousands of integration lines | SSH setup plus tool; detached PTY survives with session tool; mosh adds roaming and UDP prerequisites, not arbitrary fresh-client restoration | Mature alternative for users already using SSH; second UI/history owner or limited redraw; misses the fixed setup target |
| C: custom session daemon + bridge + SDK | Own narrow session protocol; reuse PTY library and iroh; likely thousands to tens of thousands across installation, recovery and integration | Local installation and one device pairing; daemon retains PTY across disconnect; authenticated allowlist | Recommended foundation, with G1/G3 feasibility gates; changes explicit remote and persistence exclusions |
| Worker-only transport for C | Prebuilt WSS relay and endpoint TLS; same PTY/session owner | No SSH or Container prerequisite; project owns more transport composition | Strongest network alternative if deployment cost/automation defeats iroh; one backend only |
| Embedded russh/libssh2 | SSH client library and deployment adapter, still needing persistence layer | SSH protocol/server and authentication remain | Does not remove setup/session problem; adds protocol compatibility obligations |
| Tailscale SSH/network | Reuse network/identity product | Additional account/product or control-plane operation; no automatic terminal recovery | Optional in existing installations, not the default user flow |

`shpool 0.11.5` exposes `unsafe run(Args, Hooks)` rather than a composable public session SDK. Its default restore uses `contents_formatted + input_mode_formatted`; input-mode restoration does cover some modes and must not be described as losing every mode. See [public entrypoint](https://github.com/shell-pool/shpool/blob/3c41df9a610428b6c1766d78d36d3fefd5685c3b/libshpool/src/lib.rs#L370) and [restore implementation](https://github.com/shell-pool/shpool/blob/3c41df9a610428b6c1766d78d36d3fefd5685c3b/libshpool/src/session_restore.rs#L81).

An agent can retain the original shpool UDS attachment and buffer missing bytes between agent and client. An ordinary outage then avoids its formatter entirely. shpool also isolates its PTY owner from a network agent crash; a single custom daemon loses both if it crashes. These are real advantages. Both routes still need the same G1 recovery adapter.

**Deciding fact for session ownership:** can a maintained shpool adapter supply the required lifecycle and atomic checkpoint/output boundaries through stable interfaces, while removing more project-owned code than its integration adds? The inspected release exposes CLI `run`/hooks, not those composable session APIs, and its default restore fails the probe below. Prefer direct PTY ownership for those seams, rather than assuming a second daemon eliminates recovery work. A binary/UDS adapter may still be viable; no minimal adapter comparison has been executed. Reconsider if it demonstrates the required seams with less code and version coupling. Requiring survival of the network agent's crash would independently strengthen the case for shpool's separate PTY owner. Do not add FD handoff/supervisor machinery before that extra crash guarantee is required.

Other foundations do not eliminate the gap: wezterm mux uses its own cell/render protocol and unpublished mux/codec crates; zellij owns layout and rendered UI; dtach/abduco keep processes without a full screen; EternalTerminal's bounded sequence replay is useful but not fresh-surface reconstruction. `portable-pty 0.9.0` is a PTY API candidate, not a persistent service; the daemon must retain its writer across network loss. See [wezterm codec](https://github.com/wezterm/wezterm/blob/372548295b0b25c9a5400f0ea56f0e89f47e0524/codec/src/lib.rs), [zellij IPC](https://github.com/zellij-org/zellij/blob/b24a7a1e3edf06b6bf411f54d4232197b3de6b25/zellij-utils/src/ipc.rs), and [PTY API](https://docs.rs/portable-pty/0.9.0/portable_pty/trait.MasterPty.html).

### Executed synthetic restoration probe

Using `shpool_vt100 = "=0.1.3"`, a 5×40 terminal and 20 history lines, format the current state with the same two functions as default shpool restoration, load it into a fresh parser, then append the remaining bytes. The probe produced:

| Input boundary | Continuous parsing | Formatted restoration |
| --- | --- | --- |
| `hello\x1b[31` then `mRED` | `helloRED` | `hellomRED` |
| `hello\xe4\xb8` then `\xad` | `hello` followed by U+4E2D | `hello` followed by U+FFFD |
| `PRIMARY\x1b[?1049hALT`, restore, then `\x1b[?1049l` | `PRIMARY` | `ALT` |

This proves the tested representation is lossy. It did not launch a full shpool daemon, its optional vterm engine, a Ghostty window or a network reconnect. It does not prove every real TUI fails. The following minimal source reproduces the parser comparison in a temporary Cargo project with that pinned dependency; observed output above comes from the executed probe, not an end-to-end acceptance test.

```rust
fn restore(p: &shpool_vt100::Parser) -> Vec<u8> {
    let mut bytes = p.screen().contents_formatted();
    bytes.extend(p.screen().input_mode_formatted());
    bytes
}
fn partial(name: &str, before: &[u8], after: &[u8]) {
    let mut original = shpool_vt100::Parser::new(5, 40, 20);
    original.process(before);
    let mut fresh = shpool_vt100::Parser::new(5, 40, 20);
    fresh.process(&restore(&original));
    fresh.process(after);
    original.process(after);
    println!("{name}: continuous={:?} restored={:?}", original.screen().contents(), fresh.screen().contents());
}
fn main() {
    partial("partial CSI", b"hello\x1b[31", b"mRED");
    partial("partial UTF8", b"hello\xe4\xb8", b"\xad");
    let mut original = shpool_vt100::Parser::new(5, 40, 20);
    original.process(b"PRIMARY\x1b[?1049hALT");
    let mut fresh = shpool_vt100::Parser::new(5, 40, 20);
    fresh.process(&restore(&original));
    original.process(b"\x1b[?1049l");
    fresh.process(b"\x1b[?1049l");
    println!("leave alternate: continuous={:?} restored={:?}", original.screen().contents(), fresh.screen().contents());
}
```

## Failures and acceptance gates

| Failure | Required result |
| --- | --- |
| Host/relay unavailable | Keep pane and registration, bounded retry, visible connectivity; no replacement shell |
| Host identity changes | Stop and require re-pairing; never automatically accept a new key |
| OAuth denied, expired or missing permission | Report incomplete setup/update; preserve unrelated cloud resources |
| Remote Git missing | Fail that repo refresh, retain registration, do not install Git silently |
| Shell/cwd missing | Fail creation explicitly; no silent fallback to Home pretending it is the repo |
| Checkpoint corrupt/incompatible or output gap | Preserve process; show recovery failure; do not restart or claim success |
| Congestion or quota rejection | Bounded fair queues, visible degraded connection; original process retained |
| Stale busy or late operation | Treat busy as unknown; reject stale generation/writer operations |

| Gate | Scenarios | Passing observation |
| --- | --- | --- |
| G1a: surviving surface | Every-byte UTF-8/VT cut, sleep/reconnect, OSC effects and terminal queries | Same state and input results as continuous baseline; no repeated delivered effects or replies |
| G1b: reconstructed surface | New bridge/surface, output beyond retained suffix, zsh/vim/less, mouse, paste, kitty keyboard, size change | Correct screens/modes/bounded history; real public APIs or an explicitly implemented minimal upstream interface |
| G2: sessions and limits | Relay restart, bridge crash, lost ACK, late writer, noisy pane, long output | Original PID/group retained; one create/input effect; bounded resources and fair progress |
| G3: deploy/pair/security | Fresh accounts, no SSH/Node/Docker/GitHub, partial deployment, UDP blocked, hostile relay, revoke/replay/oversized frames | Automated owned deployment within a cost boundary agreed before this gate; both devices communicate; no public test relay, secrets or unauthorized execution |
| G4: Combe entrypoints | Local regression; two hosts/same paths; tabs/splits/cwd; offline/missing Git; close; Hop; quota; light/dark | Existing semantics preserved, no path collision or accidental deletion, actual windows inspected |
| G5: services | Login/logout, update, stop/uninstall, reboot | Declared service guarantees hold; service restart is never called original-process recovery |

Run small G1 and G3 feasibility probes first. Then build G2, integrate G4, and verify packaging/service G5. Do not scaffold a complete host UI before the two decisive gates. No implementation, installed service, two-host control, OAuth deployment or cloud bill has been validated by this proposal.

## Scope boundaries for implementation planning

Propose a first support matrix of macOS Apple Silicon on both ends with zsh; this is not yet an accepted platform restriction. Linux/systemd, other shells and CPUs require an explicit matrix and acceptance workload. Logged-in user-service operation is not a promise of logout survival. Paid relay operation also remains a planning boundary; free-tier deployment must not be assumed accepted or rejected.

Easy work is spawning a shell, moving bytes, resizing and remote Git execution. Hard work is bounded terminal recovery, exactly-once query/effect handling, close intent, trustworthy cwd/busy, device revocation and reconnect input safety. Cloudflare does not supply those PTY semantics.

Leave out remote file browsing/sync/download, an editor, shared writers/collaboration, root control, remote GUI launch, predictive input, unlimited recording, process resurrection after reboot/PTY-owner crash, transparent hot upgrade, a settings GUI, multi-tenant administration, remote quota and cloud terminal-content telemetry.

Before implementation planning is closed, establish the actual remote OS/CPU, whether execution must survive logout, and the allowed relay operating cost. Project name, license, publisher OAuth identity and release ownership are separate publication decisions. This draft creates none of those resources.
