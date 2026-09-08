# Historical slot reuse maintenance record

Recorded 2026-09-07 for the user-authorized GAIN-860 worker-slot exception.
This controller directory is not Git-backed, so this record preserves the
reviewable before/current contract comparison and exact hashes of every changed
file. No live epic state or worker checkout was changed by this maintenance
implementation.

## Contract comparison

Before this change, `bind` accepted only `key` and `slot`. A prepared follow-up
rejected every slot found in retained delivery history, and it rejected binding
when `origin/main` advanced after preparation. The only recovery was another
task-scoped preparation decision. The controller had no supported way to record
the user's exact historical-slot exception.

The current `bind` input still accepts the ordinary two-field request. It also
accepts this optional, strict object:

```json
{
  "key": "GAIN-860",
  "slot": "worker7",
  "historical_slot_reuse": {
    "historical_key": "GAIN-866",
    "source": "Current user: these all make sense. Bypass the worker-slot restriction for GAIN-860 I need this work done",
    "artifact": "/home/michael/businesscraft/businesscraft/.local/epic-runs/GAIN-847-20260905/approval-slot-exception-and-database-20260907.md"
  }
}
```

The optional object rejects unknown fields. The controller accepts it only for
the active, unbound prepared follow-up. The named historical task must have a
confirmed completed delivery assigned to that slot. Existing reservation,
unfinished-owner, clean checkout, repository identity, task branch, and
`HEAD == origin/main` checks still run.

The current delivery must have no launch, operation, evidence, review, loop
plan, measurement, human review, checks, PR, or verified merge. Its extra-cycle
allowance may be absent or wholly unused. The bind hashes the user artifact and
stores the delivery ID, task, slot, named historical task, source, requirements,
old prepared base, refreshed base, and timestamp. Launch validation checks that
receipt again.

If main advanced, the accepted bind updates only the follow-up prepared base,
current snapshot, and wholly unused current extra-cycle snapshot. It does not
change the first archived task base, delivery history, archived evidence,
reviews, findings, counters, deadlines, or launch records. Preparation of a
later delivery archives and clears the current receipt. A new exact bind request
is still required for that later delivery.

## Behavioral proof

`explicit_historical_slot_reuse_refreshes_only_unstarted_delivery_state` drives
the JSON CLI through follow-up preparation, a budget receipt, a main advance,
slot reservation, and binding. It would fail if the controller kept the stale
prepared base, failed to update the unused allowance, changed the archived
baseline or history, consumed a launch, reset counters, accepted ordinary
historical binding, or stopped checking the slot receipt before a launch.

`historical_slot_exception_rejects_unfinished_owner_or_started_prepared_work`
drives the same public bind and verifies no state change when another unfinished
task owns the slot, current evidence exists, or a current-delivery launch has
already occurred. The retained-state setup is injected because reproducing an
earlier completed delivery or corrupted pre-bind state through the public CLI
would replay unrelated delivery work.

The canonical gate passed under Rust 1.98.0:

```text
cargo run --locked --manifest-path /home/michael/.codex/skills/holy-ai-agents-batman/controller/Cargo.toml -p xtask -- quality
Rust quality gates passed.
```

It covered formatting, strict Clippy, cyclomatic complexity, 104 controller
behavior tests, 6 gate behavior tests, documentation tests, and the locked
release build. The cached Nix-built Rust toolchain had a missing runtime linker
path. The run used a temporary copy with its ELF interpreter changed to the
host loader and selected the host bfd linker. No controller source, policy,
state, or persistent toolchain file was changed for that workaround.

## Changed file hashes

```text
d40f578a2045f705d4e7e3ce8372544b9876fcb75cb3dee64865d45f8ae8e467  controller/src/model.rs
692872f5b84a1a4dd9b042a12fd569894e35506991d9a2897251c746e8058faa  controller/src/store.rs
cbeba76418528b8ea40cdec3230e0cdf045c52c810efd0e860ad913b74f03fbe  controller/src/engine/tasks.rs
d321ad44c52ca02ddfaacc91a5df386f7f5efd43a1b3ff696dc01623fcd735e3  controller/src/followup.rs
0b1357dd21d4177e422f6f26921231ffbf33b48d683679f68c3981f21dac7324  controller/src/runtime.rs
f73cb56374eea43fb77c812a8f9a5235e40822a43a3425e6312495d719a9f8db  controller/tests/improvements/followup.rs
0d010a5a959e21f263f7f43d9089005cc407642d868bf725039c69f6646c5e01  references/controller.md
ce9b7c903a9abe99e8663f001a5dd992b5eeb39993d20f0d9e185d54190a071d  references/followup-recovery.md
```
