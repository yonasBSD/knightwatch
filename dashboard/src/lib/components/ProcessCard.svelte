<script>
  import { formatBytes } from "../utils/format.js";

  let { proc, isRoot = false } = $props();

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
</script>

<div class="proc-card" class:root-card={isRoot}>
  <div class="proc-header">
    <div class="proc-name" title="{proc.name} (PID {proc.pid})">
      {#if isRoot}⬢
      {/if}{proc.name}
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
</style>
