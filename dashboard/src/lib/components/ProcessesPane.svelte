<script>
  import { onMount, onDestroy } from "svelte";
  import ProcessCard from "./ProcessCard.svelte";

  let { active, hasPids, hasTopProcesses, limitProcesses } = $props();

  // ── Tracked processes ─────────────────────────────────────────────
  let rootGroups = $state([]); // [{pid, root, children, child_count, work_done}]
  let workDone = $state(false);
  let trackedError = $state(false);
  let openPids = new Set(); // preserve open/close state of <details>

  // ── Top processes ─────────────────────────────────────────────────
  let topProcesses = $state([]);
  let topSort = $state("cpu");
  let topLimit = $state(5);
  let topError = $state(false);

  let processInterval = null;
  let topInterval = null;

  // ── Tracked fetch ─────────────────────────────────────────────────
  async function refreshTracked() {
    if (!hasPids) return;
    try {
      const rIds = await fetch("/api/root_pids");
      if (!rIds.ok) throw new Error("HTTP error");
      const pids = await rIds.json();

      if (pids.length === 0) {
        workDone = false;
        rootGroups = [];
        return;
      }

      const groups = [];
      let allDone = true;

      for (const pid of pids) {
        try {
          const r = await fetch(`/api/process/${pid}`);
          if (!r.ok) continue;
          const data = await r.json();
          allDone = allDone && data.work_done;
          groups.push({ pid, ...data });
        } catch {}
      }

      rootGroups = groups;
      workDone = pids.length > 0 && allDone;
      trackedError = false;
    } catch {
      trackedError = true;
    }
  }

  // ── Top processes fetch ───────────────────────────────────────────
  async function refreshTop() {
    if (!hasTopProcesses) return;
    try {
      const r = await fetch(
        `/api/top-processes?sort=${topSort}&limit=${topLimit}`,
      );
      if (!r.ok) throw new Error("HTTP error");
      const data = await r.json();
      topProcesses = data || [];
      topError = false;
    } catch {
      topError = true;
    }
  }

  onMount(() => {
    refreshTracked();
    processInterval = setInterval(refreshTracked, 2000);

    if (hasTopProcesses) {
      refreshTop();
      topInterval = setInterval(refreshTop, 2000);
    }
  });

  onDestroy(() => {
    clearInterval(processInterval);
    clearInterval(topInterval);
  });

  // Re-fetch top when sort/limit change
  $effect(() => {
    topSort;
    topLimit;
    if (hasTopProcesses) refreshTop();
  });

  // Cap topLimit to server max
  $effect(() => {
    if (topLimit > limitProcesses) topLimit = limitProcesses;
  });
</script>

<aside id="process-pane">
  <div class="pane-header">
    <h2>Processes</h2>
  </div>

  <div id="process-content">
    {#if workDone}
      <div id="work-banner" class="visible">
        ✔ Work complete — all children exited
      </div>
    {/if}

    <div class="process-columns">
      <!-- Tracked column -->
      {#if hasPids}
        <div class="process-column">
          <div class="section-header">Tracked</div>

          {#if trackedError}
            <div class="muted">Monitor disabled</div>
          {:else if rootGroups.length === 0}
            <div class="muted">No process tracker running.</div>
          {:else}
            {#each rootGroups as group (group.pid)}
              <div class="process-group">
                {#if group.root}
                  <ProcessCard proc={group.root} isRoot={true} />
                {:else}
                  <div class="muted">Root process {group.pid} exited</div>
                {/if}

                {#if group.child_count > 0}
                  <details
                    class="children-group"
                    style="margin-top:0.5rem;margin-left:0.75rem"
                    bind:open={openPids[group.pid]}
                  >
                    <summary
                      class="section-header"
                      style="margin-top:0;cursor:pointer;user-select:none"
                    >
                      Children <span class="count-badge"
                        >{group.child_count}</span
                      >
                      <span
                        class="muted"
                        style="margin-left:auto;font-size:0.7rem;font-weight:normal"
                        >(click to toggle)</span
                      >
                    </summary>
                    <div
                      style="border-left:2px solid var(--border);padding-left:0.75rem;margin-top:0.5rem;display:flex;flex-direction:column;gap:0.5rem"
                    >
                      {#each group.children as child (child.pid)}
                        <ProcessCard proc={child} />
                      {/each}
                    </div>
                  </details>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
      {/if}

      <!-- Top processes column -->
      {#if hasTopProcesses}
        <div class="process-column">
          <div class="section-header">
            Top Processes
            <div class="top-controls">
              <select class="control-input" bind:value={topSort}>
                <option value="cpu">CPU</option>
                <option value="mem">MEM</option>
              </select>
              <input
                type="number"
                class="control-input"
                style="width:3rem"
                bind:value={topLimit}
                min="1"
                max={limitProcesses}
              />
            </div>
          </div>

          {#if topError}
            <div class="muted">Failed to load top processes</div>
          {:else if topProcesses.length === 0}
            <div class="muted">No processes found</div>
          {:else}
            <div id="top-processes-list">
              {#each topProcesses as proc (proc.pid)}
                <ProcessCard {proc} />
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</aside>

<style>
  #process-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 2rem;
    border-bottom: 1px solid var(--border-soft);
    flex-shrink: 0;
  }
  .pane-header h2 {
    font-size: 0.78rem;
    font-weight: 700;
    color: #fff;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  #process-content {
    overflow-y: auto;
    flex: 1;
    padding: 1.5rem 2rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .process-columns {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(420px, 100%), 1fr));
    gap: 1.5rem;
    align-items: start;
  }
  .process-column {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    background: var(--bg-panel);
    border: 1px solid var(--border-soft);
    border-radius: 0.85rem;
    padding: 1rem 1rem 1.1rem;
  }
  .process-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  #work-banner {
    display: none;
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.25);
    border-radius: 0.6rem;
    color: var(--success);
    font-size: 0.8rem;
    font-weight: 600;
    padding: 0.75rem;
    text-align: center;
  }
  #work-banner.visible {
    display: block;
  }
  .section-header {
    font-size: 0.7rem;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .count-badge {
    background: var(--border);
    color: var(--text-base);
    border-radius: 6px;
    padding: 0.125rem 0.5rem;
    font-size: 0.7rem;
  }
  #top-processes-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .top-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .control-input {
    background: var(--bg-card);
    color: var(--text-base);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.2rem 0.4rem;
    font-size: 0.7rem;
    outline: none;
    font-family: inherit;
  }
  .control-input:focus {
    border-color: var(--accent);
  }
  .muted {
    color: var(--text-muted);
    font-size: 0.85rem;
    text-align: center;
    padding: 2rem 0;
    font-style: italic;
  }
  .children-group > summary {
    list-style: none;
  }
  .children-group > summary::-webkit-details-marker {
    display: none;
  }
</style>
