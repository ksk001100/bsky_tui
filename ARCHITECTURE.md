# Architecture

The application follows a unidirectional Elm/Redux-style data flow:

```text
Input or effect result
        │
        ▼
     Message
        │
        ▼
update(&mut App, Message) ──► Update { control, commands }
        │                              │
        │                              ▼
        │                        effect runtime
        │                              │
        └──────── EffectMessage ◄──────┘

App ──► ui::render
```

## Boundaries

- `app/message.rs` contains every event accepted by the reducer. Network and
  filesystem results return as typed `EffectMessage` variants.
- `app/update/` is the only state-transition layer. It is synchronous and does
  not perform network, process, browser, clipboard, logging, or task-spawning
  I/O.
- `app/command.rs` describes effects as data. Commands carry an immutable
  `EffectContext` snapshot; effect workers never hold or mutate the live model.
- `io/handler/` executes commands and sends typed results back to the main event
  loop. Applying each result returns a fresh context acknowledgement for
  multi-step effects.
- `app/ui/` renders the current model. Mutable values used during rendering are
  presentation caches required by Ratatui and do not update domain state.
- `app/state/` owns a single top-level `Model`, composed from session,
  navigation, composer, home, notification, explore, thread, and profile state.
  State operations are grouped by feature and mutate only their domain state.
- `bsky/` contains Bluesky API services and is independent of the reducer.

## Invariants

1. Application state changes only through `App::init` or `App::update`.
2. Update handlers return effects as `Command` values; they never execute them.
3. Effect workers receive snapshots and cannot access `Arc<Mutex<App>>`.
4. Effect results re-enter the same event loop as user input.
5. New asynchronous features require a `Command` and a typed `EffectMessage`.

## Effect runtime policy

- I/O commands run in FIFO order on one worker. A multi-step command waits for
  the reducer acknowledgement after each result, so later commands cannot
  observe an older model snapshot.
- Commands are not individually spawned or cancelled. Closing the command
  channel stops the worker after the command currently being handled; dropping
  the application runtime cancels the worker task.
- Each completed I/O command logs `queue_wait_ms`, `execution_ms`, and
  `ack_wait_ms` with operation name and status only. `queue_wait_ms` exposes
  head-of-line blocking without logging payloads or account data. Add effect
  lanes only after these measurements demonstrate a user-visible bottleneck;
  lane work must add request generations and reject stale reducer results.

## Reducer tests

Reducer tests construct an `App`, submit a `Message`, and assert both the model
and emitted `Command` values without executing I/O. Regression coverage must
include failure completion, valid empty responses, command payloads, and
repeated input while an effect is in flight. Handler tests cover API-response
transformation separately from reducer tests.
