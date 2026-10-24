# Focus Roadmap

## Vision

Focus grows from a single pomodoro app into a modular, API-first platform: one small background service (`focusd`) owns the timer, tasks and stats, and everything else is an optional client. Users install only the parts they want, and other developers build their own integrations on the same API.

- **Modular:** Core is required; the GUI, bar mode, CLI, Focus Guard, Focus Quest, Focus Agent and Focus Social are each optional.
- **API-first:** Every feature goes through the `focusd` API, including Focus's own tools.
- **Cross-platform:** Windows first, then Linux, sharing one Rust codebase.
- **Open to builders:** A stable, documented protocol so people can wire Focus into anything that helps them stay productive.

## Who it's for

Each audience uses a different mix of modules, which is why every module is optional.

| User | What they use most |
| --- | --- |
| Worker or freelancer | Timesheet export for billing, Slack status sync, IDE plugin |
| Individual student | Study streaks, time per subject, exam countdown that plans sessions backward from the date |
| Classroom | Teacher-started class-wide sessions, Guard on lab computers, class totals only (never one student's details) |
| Smart home | Desk light during focus, phone DND, door sign, music that follows sessions |
| Study streamers | OBS overlay, shared co-working timers |
| Remote teams and co-working groups | Synced sessions in a Discord voice channel |
| People who struggle to start tasks | Body-doubling rooms, gentle agent nudges, 10-minute starter sessions |
| Writers and creators | Progress per session |

## Architecture

`focusd` is the only required piece; every other module, including third-party apps, talks to it through the same local API.

```
 Desktop + bar   Focus CLI   Focus Guard   Focus Agent   Your apps
       \             |            |             |            /
        \            |            |             |           /
   +-------------------------------------------------------------+
   |  focusd (required core)                                     |
   |  Local API: commands in, events out (versioned protocol)    |
   |  Owns the timer, tasks, stats and config                    |
   +-------------------------------------------------------------+
              |                                   |
     Storage (SQLite, config.toml)     Event hooks (user scripts)
```

Repo layout as a Cargo workspace:

```
focus/
├── crates/
│   ├── focus-core/    # timer, tasks, storage logic
│   ├── focusd/        # background service + local API
│   ├── focus-client/  # typed Rust client (published later)
│   ├── focus-app/     # GUI + bar mode
│   ├── focus-cli/
│   ├── focus-guard/
│   └── focus-agent/   # MCP server + AI features
└── Cargo.toml
```

Modules are switched on and off in one shared config file:

```toml
[modules]
bar = true
guard = false
agent = false

[guard]
mode = "warn"   # warn | minimize | close
```

## v1: the whole project for now

v1 is `focusd`, the CLI, bar mode and hooks, shipped publicly. It is complete and useful on its own; everything under **Later** waits until v1 shows real traction.

**Checkpoint after launch** (about 2 to 3 months out): keep building Later features if any of these happen.

- Strangers star the repo, file issues or open pull requests
- Someone builds a hook, script or integration on the API
- You still use Focus daily and keep hitting limits v1 doesn't solve

If none happen, stop without regret: you have a strong portfolio project and a tool you use every day.

### v1, step 1: Foundation

Goal: move all state out of the GUI and into `focusd`, behind a versioned protocol.

- [ ] Convert the repo into a Cargo workspace (`crates/focus-core`, `focus-app`, `focusd`, `focus-client`)
- [ ] Move timer, tasks and storage logic into `focus-core`
- [ ] Build `focusd`: owns state, runs the local API (named pipe on Windows, Unix socket on Linux via `interprocess`)
- [ ] Broadcast events: `SessionStarted`, `SessionEnded`, `BreakStarted`, `BreakEnded`, `TaskCompleted`
- [ ] Add a `"v": 0` field to every message
- [ ] Write `PROTOCOL.md` with every command and event
- [ ] Build `focus-client`, the typed Rust client crate
- [ ] Auto-start `focusd` from any client when it isn't running

### v1, step 2: Eat your own cooking

Goal: Focus's own tools become the first three API clients, so gaps in the protocol show up early.

- [ ] **Focus CLI:** `focus start`, `focus stop`, `focus add`, `focus done`, `focus status`, `focus config` (built with `clap`)
- [ ] **GUI on `focusd`:** Rewire the existing app to read and write through the API instead of owning state
- [ ] **Bar mode (Windows):** A compact, always-on-top, borderless timer view inside `focus-app`, toggled from the tray
- [ ] **Bar mode (Linux bars):** `focus status --json` for Waybar and Polybar
- [ ] **Event hooks:** `on_session_start`, `on_session_end`, `on_break_start` in `config.toml` run user scripts

### v1, step 3: Launch

Goal: strangers can install it in minutes and understand what makes it different.

- [ ] README with a GIF of bar mode, install steps and a "build on Focus" section
- [ ] Publish `PROTOCOL.md` (still `v0`, marked unstable)
- [ ] Basic Windows installer (app, `focusd`, CLI) and `cargo install` for Linux
- [ ] Two example hooks: Slack/Discord status and smart-light toggle
- [ ] Post to r/rust, and the Waybar module to r/unixporn
- [ ] Set the checkpoint date and note what you're watching for

## Later: if v1 takes off

These stay parked until the checkpoint says go. Build them in this order, and let what v1 users ask for reorder them.

### Later 1: Linux, Guard, logs and Quest

Goal: Focus runs on Linux and can protect a session, not just time it.

- [ ] **Focus for Linux:** Port the GUI and run `focusd` as a systemd user service
- [ ] **Focus Guard, websites:** Redirect blocked domains in the hosts file during sessions, restore on breaks
- [ ] **Focus Guard, apps:** Poll running processes with `sysinfo` and act on blocked ones
- [ ] **Guard modes:** `warn` (default), `minimize`, `close`
- [ ] **Guard privileges:** Windows service set up by the installer; small privileged helper on Linux
- [ ] **Module config:** `[modules]` on/off switches in `config.toml`, plus a settings page in the GUI

**Session logs** (local and private by default):

- [ ] Log every session, break, task and Guard block to SQLite
- [ ] Stats view: focus hours per day and week, streaks, heatmap, best hours of the day
- [ ] Estimate vs. actual pomodoros per task
- [ ] Distraction report from Guard blocks
- [ ] Burnout signals: skipped breaks, late-night sessions, no rest days
- [ ] Tag sessions to a task, project or git branch
- [ ] Export to CSV/JSON (timesheets, invoices) and a daily Obsidian note

**Focus Quest** (gamified mode, optional module):

- [ ] XP from completed sessions (e.g. 10 XP per pomodoro), plus task and streak bonuses
- [ ] Allocate XP after each session to user-defined skills (Rust, Reading, Fitness, Spanish)
- [ ] Overall level from total XP, with a rising curve: `xp_needed = 100 × level^1.5`
- [ ] Skill tree view: level and progress bar per skill
- [ ] Weekly quests and achievements
- [ ] One rest-day token per week to protect streaks
- [ ] Daily XP cap and bonus XP for taking breaks, so it rewards consistency, not burnout
- [ ] XP bar in bar mode
- [ ] Emit `XpGained` and `LevelUp` events for integrations

### Later 2: Full installers

Goal: anyone can install exactly the parts they want in a few clicks.

- [ ] **Windows installer:** Inno Setup components or WiX features (`cargo-wix`), with core required and Desktop app, Bar mode, CLI, Guard, Quest, Agent and Social as checkboxes
- [ ] Register `focusd` and the Guard service at install
- [ ] Publish to `winget`
- [ ] **Linux packages:** One package per module (`focus-core`, `focus-cli`, `focus-guard`, ...) and `cargo install` support
- [ ] Cargo features (`--features guard`, `--features agent`) for packagers

### Later 3: Open it up

Goal: other developers can build on Focus without reading its source.

- [ ] Freeze protocol `v1` and publish full docs
- [ ] **WebSocket/HTTP gateway:** localhost only, off by default, token-protected, for JS and Python developers
- [ ] Publish `focus-client` to crates.io
- [ ] Publish the Guard blocking engine as a standalone crate
- [ ] **Reference integrations:** Discord/Slack status, Spotify pause on break, OBS overlay for study streams
- [ ] **Community showcase:** A page or repo listing what people have built
- [ ] Optional: a small Python client package

### Later 4: Focus Agent

Goal: an optional AI layer that plans, protects and reviews your focus time. It is just another API client, so it stays optional like every other module.

**Step 1: MCP server (`focus-mcp`).** Expose `focusd` as MCP tools: `start_session`, `add_task`, `list_tasks`, `get_stats`, `complete_task`. Any AI assistant that speaks MCP, including Claude, can then drive Focus ("plan my afternoon and start the first session"). This is the cheapest, highest-leverage step because you write no AI logic yourself.

**Step 2: Built-in agent features.** Opt-in, using the user's own API key or a local model (e.g. via Ollama):

- [ ] **Plan my day:** Break big tasks into pomodoro-sized chunks and order them by priority and energy
- [ ] **Session review:** End-of-day summary from session logs: what got done, where focus slipped, one suggestion for tomorrow
- [ ] **Smart Guard:** Judge whether an app or site fits the current task (a Rust tutorial on YouTube is fine during "learn Rust") instead of using a fixed blocklist
- [ ] **Adaptive sessions:** Suggest longer or shorter sessions based on completion history
- [ ] **Natural-language CLI:** `focus ask "what did I work on this week?"`

**Guardrails:**

- Off by default, enabled in `[modules]`
- The agent proposes; the user approves anything that changes tasks or blocks apps
- Local-first: show exactly what data leaves the machine, and support local models
- Agent actions go through the same public API, so they're logged and auditable

### Later 5: Focus Social

Goal: people focus together, with social features around sessions and never during them. This is the first module that needs a server, so it stays optional (`focus-social`) and shares only what the user chooses.

**Step 1: Test it on Discord.** Build a Discord bot on the public API before building any network:

- [ ] Synced focus rooms in voice channels
- [ ] Show who's focusing right now
- [ ] Post level-ups and weekly recaps

**Step 2: Native social features**, if the bot proves the demand:

- [ ] **Focus rooms:** Synced timers; chat opens only on breaks
- [ ] **Friend presence:** "3 friends are focusing now" in the bar, no messaging
- [ ] **Accountability partners:** See whether each other hit the daily goal
- [ ] **Guilds:** Pool XP toward a shared weekly quest
- [ ] **Profiles:** Opt-in Focus Quest skill card
- [ ] **Shareable recaps:** A weekly summary image to post anywhere
- [ ] Accounts, rooms and presence server, plus privacy settings per item shared

No endless feed: it would be the exact distraction Focus Guard blocks.

## Guiding principles

- **No feature lives only in the GUI.** Anything a user can do anywhere goes through the `focusd` API.
- **Every module is optional** except core.
- **Treat your own tools as third-party clients.** If the CLI finds something awkward, so will outside developers.
- **Version from day one.** Every message carries `"v"`.
- **Events are additive.** Add fields and event types freely; never rename or remove them after protocol v1.
- **Keep the protocol at `v0` until v1 has outside users**, then freeze protocol v1.
- **Logs are local and private by default.** Anything shared with a teacher, team or employer is opt-in and summarized, so Focus never becomes surveillance software.
- **Games and social reward healthy focus.** Consistency and breaks earn more than cramming, and social features happen around sessions, never during them.
