<script>
  import { formatBytes } from "../utils/format.js";
  import { apiFetch } from "../api.js";

  let {
    proc,
    isRoot = false,
    isBeingTracked = true,
    allowProcessCommands = false,
    isAuthenticated = false,
    signals = /** @type {{ value: string; label: string }[]} */ ([]),
    onRefresh = null,
  } = $props();

  const FD_TYPE_COLOR = {
    file: "#a78bfa",
    socket: "#34d399",
    pipe: "#fbbf24",
    other: "#a1a1aa",
  };

  function stateCls(state) {
    if (state === "running") return "state-running";
    if (state === "sleeping") return "state-sleeping";
    if (state === "gone") return "state-gone";
    return "state-other";
  }

  function fdColor(type) {
    return FD_TYPE_COLOR[type] || "#a1a1aa";
  }

  // ── Command state ─────────────────────────────────────────────────
  let cmdPending = $state(false);
  let cmdError = $state(null);
  let showSignalPicker = $state(false);

  let canCommand = $derived(allowProcessCommands && isAuthenticated);
  let singleSignal = $derived(signals.length === 1);

  async function killProc(signal = "SIGTERM") {
    if (!confirm(`Send ${signal} to PID ${proc.pid} (${proc.name})?`)) return;
    cmdPending = true;
    cmdError = null;
    showSignalPicker = false;
    try {
      const r = await apiFetch(`/api/process/kill/${proc.pid}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ signal }),
      });
      if (!r.ok) throw new Error((await r.json()).message ?? (await r.text()));
      onRefresh?.();
    } catch (e) {
      cmdError = e.message || "Kill failed";
    } finally {
      cmdPending = false;
    }
  }

  async function killTree() {
    if (
      !confirm(
        `Kill entire process tree rooted at PID ${proc.pid} (${proc.name})?`,
      )
    )
      return;
    cmdPending = true;
    cmdError = null;
    showSignalPicker = false;
    try {
      const r = await apiFetch(`/api/process/kill-tree/${proc.pid}`, {
        method: "POST",
      });
      if (!r.ok) throw new Error((await r.json()).message ?? (await r.text()));
      onRefresh?.();
    } catch (e) {
      cmdError = e.message || "Kill tree failed";
    } finally {
      cmdPending = false;
    }
  }

  async function trackProc() {
    cmdPending = true;
    cmdError = null;
    try {
      const r = await apiFetch(`/api/process/track/${proc.pid}`, {
        method: "POST",
      });
      if (!r.ok) throw new Error((await r.json()).message ?? (await r.text()));
      onRefresh?.();
    } catch (e) {
      cmdError = e.message || "Track failed";
    } finally {
      cmdPending = false;
    }
  }

  async function untrackProc() {
    cmdPending = true;
    cmdError = null;
    try {
      const r = await apiFetch(`/api/process/untrack/${proc.pid}`, {
        method: "POST",
      });
      if (!r.ok) throw new Error((await r.json()).message ?? (await r.text()));
      onRefresh?.();
    } catch (e) {
      cmdError = e.message || "Untrack failed";
    } finally {
      cmdPending = false;
    }
  }
</script>

<div class="proc-card" class:root-card={isRoot}>
  <div class="proc-header">
    <div class="proc-name" title="{proc.name} (PID {proc.pid})">
      {#if isRoot}⬢{/if}{proc.name}
    </div>
    <span class="state-pill {stateCls(proc.state)}">{proc.state}</span>
  </div>

  <div class="proc-meta">
    <div class="proc-meta-item">
      <span class="label">PID</span>
      <span class="value">{proc.pid}</span>
    </div>
    <div class="proc-meta-item">
      <span class="label">CPU</span>
      <span class="value">{proc.cpu_usage.toFixed(1)}%</span>
    </div>
    <div class="proc-meta-item">
      <span class="label">MEM</span>
      <span class="value">{formatBytes(proc.memory_bytes)}</span>
    </div>
  </div>

  {#if proc.cmdline && proc.cmdline.length > 0}
    {@const cmd = proc.cmdline.join(" ")}
    <div class="proc-cmdline" title={cmd}>{cmd}</div>
  {/if}

  {#if proc.cwd != null || proc.open_files != null || proc.io_stats != null}
    <div class="proc-meta proc-meta-linux">
      {#if proc.cwd != null}
        <div class="proc-meta-item">
          <span class="label">CWD</span>
          <span class="value" title={proc.cwd}>{proc.cwd}</span>
        </div>
      {/if}
      {#if proc.open_files != null}
        <div class="proc-meta-item">
          <span class="label">FDs</span>
          <span class="value">{proc.open_files.length}</span>
        </div>
      {/if}
      {#if proc.io_stats != null}
        <div class="proc-meta-item">
          <span class="label">READ</span>
          <span class="value">{formatBytes(proc.io_stats.read_bytes)}</span>
        </div>
        <div class="proc-meta-item">
          <span class="label">WRITE</span>
          <span class="value">{formatBytes(proc.io_stats.write_bytes)}</span>
        </div>
      {/if}
    </div>
  {/if}

  {#if proc.open_files && proc.open_files.length > 0}
    <div class="fd-section">
      <div class="fd-section-header">
        <span>Open File Descriptors</span>
        <span class="count-badge">{proc.open_files.length}</span>
      </div>
      <div class="fd-list">
        {#each proc.open_files as f}
          <div class="fd-row">
            <span class="fd-num">{f.fd}</span>
            <span class="fd-type" style="color:{fdColor(f.fd_type)}"
              >{f.fd_type}</span
            >
            <span class="fd-target" title={f.target}>{f.target}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- ── Process commands ────────────────────────────────────────── -->
  {#if canCommand}
    <div class="proc-actions">
      <div class="kill-group">
        <button
          class="proc-btn kill"
          disabled={cmdPending}
          onclick={() => killProc(singleSignal ? signals[0].value : "kill")}
          title={singleSignal ? `Send ${signals[0].label}` : "Send SIGKILL"}
          >✕ Kill</button
        >
        {#if !singleSignal}
          <button
            class="proc-btn kill-chevron"
            disabled={cmdPending}
            onclick={() => (showSignalPicker = !showSignalPicker)}
            title="Choose signal"
            aria-label="Signal options">▾</button
          >
        {/if}
      </div>

      {#if isBeingTracked}
        <button
          class="proc-btn untrack"
          disabled={cmdPending}
          onclick={untrackProc}
          title="Untrack this PID">− Untrack</button
        >
      {:else}
        <button
          class="proc-btn track"
          disabled={cmdPending}
          onclick={trackProc}
          title="Track this PID">+ Track</button
        >
      {/if}

      {#if cmdError}
        <span class="proc-cmd-error">{cmdError}</span>
      {/if}

      {#if isRoot && singleSignal}
        <button
          class="proc-btn kill-tree"
          disabled={cmdPending}
          onclick={killTree}>Kill Tree</button
        >
      {/if}

      {#if showSignalPicker && !singleSignal}
        <div class="signal-picker">
          {#each signals as sig}
            <button class="proc-btn signal" onclick={() => killProc(sig.value)}>
              {sig.label}
            </button>
          {/each}
          {#if isRoot}
            <button
              class="proc-btn kill-tree"
              disabled={cmdPending}
              onclick={killTree}>Kill Tree</button
            >
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .proc-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    transition: all 0.2s ease;
  }
  .proc-card:hover {
    border-color: rgba(255, 255, 255, 0.1);
    background: var(--bg-elev);
  }
  .proc-card.root-card {
    border: 1px solid var(--accent);
    background: var(--accent-glow);
  }
  .proc-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    overflow: hidden;
  }
  .proc-name {
    font-size: 0.9rem;
    font-weight: 600;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .proc-card.root-card .proc-name {
    color: var(--accent);
  }
  .proc-meta {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.75rem;
  }
  .proc-meta-item {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }
  .proc-meta-item .label {
    font-size: 0.65rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 500;
  }
  .proc-meta-item .value {
    font-size: 0.85rem;
    color: #e4e4e7;
    font-family: ui-monospace, SFMono-Regular, monospace;
  }
  .state-pill {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.625rem;
    border-radius: 9999px;
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.025em;
    flex-shrink: 0;
  }
  .state-running {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
  }
  .state-sleeping {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
  }
  .state-gone {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
  }
  .state-other {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
  }

  .proc-cmdline {
    font-size: 0.72rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      monospace;
    color: var(--text-muted);
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    padding: 0.375rem 0.5rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .proc-meta-linux {
    border-top: 1px solid var(--border);
    padding-top: 0.75rem;
  }
  .proc-meta-linux .value {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
    display: block;
  }
  .fd-section {
    border-top: 1px solid var(--border);
    padding-top: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }
  .fd-section-header {
    font-size: 0.65rem;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.25rem;
  }
  .count-badge {
    background: var(--border);
    color: var(--text-base);
    border-radius: 6px;
    padding: 0.125rem 0.5rem;
    font-size: 0.7rem;
  }
  .fd-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 180px;
    overflow-y: auto;
  }
  .fd-row {
    display: grid;
    grid-template-columns: 2rem 3.5rem 1fr;
    gap: 0.375rem;
    align-items: center;
    font-size: 0.7rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      monospace;
    padding: 2px 4px;
    border-radius: 3px;
  }
  .fd-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .fd-num {
    color: var(--text-muted);
    text-align: right;
  }
  .fd-type {
    font-weight: 600;
  }
  .fd-target {
    color: #d4d4d8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Command actions (new) ──────────────────────────────────────── */
  .proc-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    padding-top: 0.625rem;
    border-top: 1px solid var(--border);
  }
  /* Kill + chevron fused into one pill */
  .kill-group {
    display: flex;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid rgba(248, 113, 113, 0.35);
  }
  .kill-group .kill {
    border: none;
    border-radius: 0;
    border-right: 1px solid rgba(248, 113, 113, 0.2);
  }
  .kill-group .kill-chevron {
    border: none;
    border-radius: 0;
    padding: 0.2rem 0.4rem;
  }
  /* Base button */
  .proc-btn {
    background: var(--bg-card);
    color: var(--text-base);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.2rem 0.6rem;
    font-size: 0.68rem;
    font-family: inherit;
    cursor: pointer;
    transition:
      background 0.12s,
      border-color 0.12s;
    white-space: nowrap;
    line-height: 1.6;
  }
  .proc-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  /* Variants */
  .proc-btn.kill,
  .proc-btn.kill-chevron {
    color: #f87171;
    background: rgba(248, 113, 113, 0.07);
  }
  .proc-btn.kill:hover,
  .proc-btn.kill-chevron:hover {
    background: rgba(248, 113, 113, 0.15);
  }
  .proc-btn.kill-tree {
    color: #fb923c;
    border-color: rgba(251, 146, 60, 0.35);
    background: rgba(251, 146, 60, 0.07);
  }
  .proc-btn.kill-tree:hover {
    background: rgba(251, 146, 60, 0.15);
  }
  .proc-btn.track {
    color: #34d399;
    border-color: rgba(52, 211, 153, 0.35);
    background: rgba(52, 211, 153, 0.07);
  }
  .proc-btn.track:hover {
    background: rgba(52, 211, 153, 0.15);
  }
  .proc-btn.untrack {
    color: #94a3b8;
    border-color: rgba(148, 163, 184, 0.3);
  }
  .proc-btn.untrack:hover {
    background: rgba(148, 163, 184, 0.08);
  }
  .proc-btn.signal {
    color: #f87171;
    border-color: rgba(248, 113, 113, 0.25);
    background: rgba(248, 113, 113, 0.05);
    font-size: 0.62rem;
    padding: 0.15rem 0.45rem;
  }
  .proc-btn.signal:hover {
    background: rgba(248, 113, 113, 0.13);
  }
  /* Signal picker row */
  .signal-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    width: 100%;
  }
  /* Inline error */
  .proc-cmd-error {
    width: 100%;
    color: #f87171;
    font-size: 0.67rem;
    font-family: ui-monospace, SFMono-Regular, monospace;
  }
</style>
