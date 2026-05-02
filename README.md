# 🖥️ Knightwatch

A lightweight, real-time browser-based dashboard for monitoring system performance, live screenshots and process activity on a remote or local machine.

---

## Overview

Knightwatch provides a sleek dark-mode web interface that streams system performance, live screen captures and process telemetry directly in your browser. The backend is a Rust server built on [Tokio](https://tokio.rs/) and [Axum](https://github.com/tokio-rs/axum), keeping the footprint small and performance high — no heavy agents or desktop apps required. It's designed for quick visual oversight of a running system, whether you're monitoring a headless server, a build machine, or a long-running automation task.

---

## Features

- **Live Screenshots** — Displays one or more connected screens, refreshed every 2 seconds via `/screenshot`
- **Process Monitor** — Tracks a root process and its children with real-time CPU, memory, and state indicators
- **Work-Done Detection** — Automatically shows a completion banner when all child processes have exited
- **Responsive Layout** — Side-by-side panels on desktop, stacked on mobile
- **Linux Extended Telemetry** — On Linux, child process snapshots include working directory, command line, open file descriptors, and I/O stats
- **System Monitor** — Real-time hardware telemetry: CPU, memory, disks, network, battery, thermals, and aggregate health scoring
- **Telegram Bot** — Optional bot for remote monitoring and push notifications on process and system events
- **Webhook Dispatcher** — POST process and system events to one or more URLs with automatic retry
- **Structured Logging** — Tracing via `tracing-subscriber` with configurable log levels via `RUST_LOG`

---

## How It Works

The Rust backend exposes a small HTTP API (served by Axum) that the frontend polls every 2 seconds:

| Endpoint | Method | Description |
| --- | --- | --- |
| `GET /` | GET | Serves the self-contained `view.html` dashboard |
| `GET /health` | GET | Returns server status, version, and uptime |
| `GET /config` | GET | Returns server config |
| `GET /screenshot` | GET | Returns a JSON array of base64-encoded PNG screen captures |
| `GET /root_pids` | GET | Returns a list of pids being tracked |
| `GET /process/<PID>` | GET | Returns root process info, child processes, CPU/memory stats, and `work_done` flag |
| `GET /process/root/<PID>` | GET | Returns only the root process snapshot, or 404 if it has exited |
| `GET /process/children/<PID>` | GET | Returns snapshots of all currently live child processes |
| `GET /process/status/<PID>` | GET | Lightweight summary — root alive/dead, child count, and `work_done` flag |
| `GET /top-processes` | GET | Returns the top N processes sorted by the given key |
| `GET /system` | GET | Returns the full system snapshot (CPU, memory, disks, network, battery, thermals, health) |
| `GET /cpu` | GET | Returns the current CPU snapshot only |
| `GET /memory` | GET | Returns the current memory snapshot only |
| `GET /disks` | GET | Returns per-disk snapshots |
| `GET /networks` | GET | Returns per-network-interface snapshots |
| `GET /gpus` | GET | Returns GPU snapshots (empty if no supported GPU detected) |
| `GET /battery` | GET | Returns the battery snapshot, or 404 if no battery present |
| `GET /host-info` | GET | Returns static host information (hostname, OS, kernel, arch, uptime) |
| `GET /temperatures` | GET | Returns thermal sensor readings |
| `POST /shutdown` | POST | Gracefully shuts down the server |

### Expected Response Shapes

**`/screenshot`**

```json
{
  "screens": [
    {
      "mime": "image/png",
      "data": "<base64>",
      "monitor_name": "Built-in Display",
      "monitor_id": 0,
      "width": 1920,
      "height": 1080,
      "timestamp": "2025-01-01T00:00:00Z"
    }
  ],
  "count": 1
}
```

**`/root_pids`**

```json
[123, 12345]
```

**`/process/1234`**

```json
{
  "work_done": false,
  "root": {
    "name": "my-app",
    "pid": 1234,
    "state": "running",
    "cpu_usage": 12.5,
    "memory_human": "128 MB"
  },
  "child_count": 2,
  "children": [...],
  "timestamp": "2025-01-01T00:00:00Z"
}
```

**`/process/root/1234`**

Returns a single `ProcessInfo` object, or `404` if the root process has exited.

**`/process/status/1234`**

```json
{
  "root_alive": true,
  "root_pid": 1234,
  "root_name": "my-app",
  "child_count": 2,
  "work_done": false,
  "timestamp": "2025-01-01T00:00:00Z"
}
```

**`/health`**

```json
{
  "status": "healthy",
  "timestamp": "2025-01-01T00:00:00Z",
  "version": "0.1.0",
  "uptime": "3600s"
}
```

**`/config`**

```json
{
  "blind": false,
  "pid": [],
  "top_processes": false,
  "limit_processes": 5,
  "telegram_bot": false,
  "system_monitor": false
}
```

**`/top-processes?sort=cpu&limit=1`**

```json
[
  {
    "name": "my-app",
    "pid": 1234,
    "state": "running",
    "cpu_usage": 12.5,
    "memory_human": "128 MB"
  }
]
```

Process `state` can be `running`, `sleeping`, `gone`, or any other string (rendered as a warning-colored pill).

**`/system`**

```json
{
  "timestamp": "2025-01-01T00:00:00Z",
  "health": "healthy",
  "cpu": {
    "usage_percent": 14.2,
    "brand": "Intel(R) Core(TM) i9-13900K",
    "frequency_mhz": 3200,
    "physical_core_count": 24,
    "cores": [{ "name": "cpu0", "usage_percent": 12.1, "frequency_mhz": 3200 }],
    "load_avg": { "one": 0.45, "five": 0.60, "fifteen": 0.72 }
  },
  "memory": {
    "total_bytes": 34359738368,
    "used_bytes": 12884901888,
    "available_bytes": 21474836480,
    "free_bytes": 18253611008,
    "used_percent": 37.5,
    "swap_total_bytes": 4294967296,
    "swap_used_bytes": 0,
    "swap_free_bytes": 4294967296,
    "swap_used_percent": 0.0
  },
  "disks": [
    {
      "name": "/dev/sda1",
      "mount_point": "/",
      "file_system": "ext4",
      "kind": "Ssd",
      "is_removable": false,
      "total_bytes": 500107862016,
      "used_bytes": 120259084288,
      "available_bytes": 379848777728,
      "used_percent": 24.0
    }
  ],
  "networks": [
    {
      "interface": "eth0",
      "rx_bytes_per_sec": 2048,
      "tx_bytes_per_sec": 512,
      "rx_total_bytes": 1073741824,
      "tx_total_bytes": 536870912,
      "rx_packets_per_sec": 4,
      "tx_packets_per_sec": 2,
      "rx_errors": 0,
      "tx_errors": 0
    }
  ],
  "gpus": [],
  "battery": null,
  "temperatures": [
    {
      "label": "coretemp Package id 0",
      "temperature_celsius": 52.0,
      "temperature_max_celsius": 71.0,
      "temperature_critical_celsius": 100.0
    }
  ],
  "host": {
    "hostname": "my-machine",
    "os_name": "Ubuntu 24.04.1 LTS",
    "kernel_version": "6.8.0-40-generic",
    "cpu_arch": "x86_64",
    "uptime_secs": 86400,
    "process_count": 312
  }
}
```

`health` can be `healthy`, `warning`, or `critical`. Individual sub-endpoints (`/cpu`, `/memory`, `/disks`, `/networks`, `/gpus`, `/battery`, `/host-info`, `/temperatures`) return their respective nested objects directly.

---

## Getting Started

### Running

```bash
knightwatch --pid <PID> --pid <PID>
```

Pass the PID of the root process you want to monitor. The server will start on `0.0.0.0:8083` by default.

### CLI Arguments

| Flag | Default | Description |
| --- | --- | --- |
| `--pid <PID>` | — | PID of the root process to track (repeatable) |
| `--host <HOST>` | `0.0.0.0` | Host address for the API server |
| `--port <PORT>` / `-p` | `8083` | Port for the API server |
| `--no-server` | `false` | Disable the API server entirely |
| `--blind` | `false` | Disable the Screen Capture API (useful on platforms where it requires elevated permissions) |
| `--system-monitor` | `false` | Enable the system monitor (CPU, memory, disks, network, battery, thermals) |
| `--telegram` | `false` | Enable the Telegram bot |
| `--with-webhook` | `false` | Enable webhook dispatching |
| `--webhook <URL>` | — | Webhook URL to POST process events to (repeatable) |
| `--top-processes` | `false` | Enable top processes tracker |
| `--limit-processes <NUMBER>` | `5` | Limit number of top processes to track (default is 5) |

To run without the API server:

```bash
knightwatch --pid <PID> --no-server
```

To run without screen capture (e.g. on a headless server or where permissions are restricted):

```bash
knightwatch --pid <PID> --blind
```

To enable the system monitor:

```bash
knightwatch --pid <PID> --system-monitor
```

To enable webhook dispatching with one or more targets:

```bash
knightwatch --pid <PID> --with-webhook --webhook https://example.com/hook
```

To enable tracking top processes with a limit:

```bash
knightwatch --top-processes --limit-processes 10
```

### Log Level

Set the `RUST_LOG` environment variable to control verbosity:

```bash
RUST_LOG=debug knightwatch --pid <PID>
```

---

## System Monitor

When enabled with `--system-monitor`, Knightwatch polls hardware metrics every second and exposes them via the `/system` family of endpoints. It also emits threshold-based events to the Telegram bot and webhook dispatcher.

### Default Thresholds

| Metric | Warning threshold |
| --- | --- |
| CPU usage | ≥ 90% |
| Memory usage | ≥ 90% |
| Disk usage (per mount) | ≥ 90% |
| Battery charge | ≤ 15% (discharging only) |

### System Monitor Events

The following events are emitted by the system monitor (see [Webhooks](#webhooks) and [Telegram Bot](#telegram-bot)):

| Event | Description |
| --- | --- |
| `systemo.initial_snapshot` | Full snapshot emitted on the first tick |
| `systemo.tick` | Full snapshot emitted every subsequent tick |
| `systemo.cpu_threshold_exceeded` | CPU usage crossed the warning threshold |
| `systemo.memory_threshold_exceeded` | Memory usage crossed the warning threshold |
| `systemo.disk_threshold_exceeded` | A disk's used percentage crossed the warning threshold |
| `systemo.battery_low` | Battery is discharging and charge fell below the threshold |
| `systemo.battery_state_changed` | Battery state changed (e.g. plugged in / unplugged) |

---

## Telegram Bot

Knightwatch includes a Telegram bot for remote monitoring and alerting without opening the web dashboard.

### Setup

Store your bot token in persistent config:

```bash
knightwatch config set telegram-token <YOUR_BOT_TOKEN>
```

Clear it if needed:

```bash
knightwatch config set telegram-token --clear
```

Verify it was saved:

```bash
knightwatch config get telegram-token
```

### Enabling

Pass the `--telegram` flag at runtime:

```bash
knightwatch --pid <PID> --telegram
```

### Capabilities

The bot sends push notifications for all process events when tracking at least one process:

- 🟢 **Initial snapshot** — root and children when tracking begins
- 🆕 **Children appeared** — new child processes detected
- 🔴 **Children exited** — specific child PIDs exited
- ✅ **All children gone** — all child processes have exited
- 💀 **Root process exited** — the root process itself has stopped

When `--system-monitor` is also enabled, the bot additionally sends alerts for:

- ⚠️ **CPU threshold exceeded** — aggregate CPU usage above 90%
- ⚠️ **Memory threshold exceeded** — memory usage above 90%
- ⚠️ **Disk threshold exceeded** — a mount point usage above 90%
- 🔋 **Battery low** — charge below 15% while discharging
- 🔌 **Battery state changed** — plugged in, unplugged, or full

---

## Webhooks

Knightwatch can POST process and system events to one or more HTTP endpoints. Useful for integrating with external orchestration, alerting, or logging pipelines.

### Usage

Enable webhooks with `--with-webhook` and provide one or more targets via `--webhook`:

```bash
knightwatch --pid <PID> --with-webhook --webhook https://example.com/hook --webhook https://other.com/hook
```

Webhook URLs can also be stored in persistent config (merged with any provided via `--webhook` at runtime, deduplicated). See [Persistent Configuration](#persistent-configuration) below.

### Payload Format

```json
{
  "version": "1.0.0",
  "event": "process.children_exited",
  "timestamp": "2025-01-01T00:00:00Z",
  "data": {
    "pids": [5678, 5679]
  }
}
```

**Process event names:**

| Event | Description |
| --- | --- |
| `process.initial_snapshot` | First capture after startup |
| `process.children_appeared` | New child processes detected |
| `process.children_exited` | One or more children exited |
| `process.all_children_gone` | All children have exited |
| `process.root_exited` | Root process exited |
| `process.work_complete` | Work-done condition met |

**System monitor event names** (requires `--system-monitor`):

| Event | Description | Key `data` fields |
| --- | --- | --- |
| `systemo.initial_snapshot` | First hardware snapshot | `snapshot` |
| `systemo.tick` | Periodic hardware snapshot | `snapshot` |
| `systemo.cpu_threshold_exceeded` | CPU crossed warning threshold | `usage_percent`, `threshold` |
| `systemo.memory_threshold_exceeded` | Memory crossed warning threshold | `usage_percent`, `threshold` |
| `systemo.disk_threshold_exceeded` | Disk crossed warning threshold | `mount_point`, `usage_percent`, `threshold` |
| `systemo.battery_low` | Battery charge below threshold | `charge_percent`, `threshold` |
| `systemo.battery_state_changed` | Battery state changed | `state` |

Failed deliveries are retried up to 3 times with exponential backoff.

---

## Persistent Configuration

Knightwatch stores settings such as the Telegram token and webhook URLs in a persistent config file, managed via the `config` subcommand. Persisted webhook URLs are merged with any `--webhook` flags provided at runtime and deduplicated.

The `config` subcommand uses `get` and `set` actions with the following fields:

### `telegram-token`

```bash
# Set the token
knightwatch config set telegram-token <TOKEN>

# Clear the token
knightwatch config set telegram-token --clear

# Read the current value
knightwatch config get telegram-token
```

### `webhook-urls`

```bash
# Add one or more URLs
knightwatch config set webhook-urls --add https://example.com/hook --add https://other.com/hook

# Remove a specific URL
knightwatch config set webhook-urls --remove https://example.com/hook

# Clear all stored URLs
knightwatch config set webhook-urls --clear

# List all stored URLs
knightwatch config get webhook-urls
```

## License

MIT
