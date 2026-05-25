<script>
  import { fmtTimestamp } from "../utils/format.js";
  import { apiFetch } from "../api.js";

  let {
    active,
    enabled,
    status = $bindable(),
    statusError = $bindable(),
  } = $props();

  /** @type {Array<{monitor_id: string|number, monitor_name: string, width: number, height: number, timestamp: string, mime: string, data: string}>} */
  let screens = $state([]);

  async function refresh() {
    const start = Date.now();
    try {
      const r = await apiFetch("/api/screenshot");
      if (!r.ok) throw new Error("HTTP error");
      const data = await r.json();
      screens = data.screens;
      const elapsed = Date.now() - start;
      status = `● LIVE · ${data.screens.length} SCREEN${data.screens.length > 1 ? "S" : ""} · ${elapsed}MS`;
      statusError = false;
    } catch {
      status = `● OFFLINE · ${new Date().toLocaleTimeString()}`;
      statusError = true;
    }
  }

  // Use $effect so polling starts/stops reactively based on `enabled`
  $effect(() => {
    if (!enabled) return;
    refresh();
    const id = setInterval(refresh, 2000);
    return () => clearInterval(id);
  });
</script>

<main id="screens-pane">
  <div class="pane-header">
    <h2>Monitored Screens</h2>
  </div>
  <div id="screens">
    {#each screens as screen, i (screen.monitor_id ?? i)}
      <div class="screen-container">
        <div class="screen-label-row">
          <span class="screen-label screen-name"
            >{screen.monitor_name || `Display ${i + 1}`}</span
          >
          {#if screen.width && screen.height}
            <span class="screen-meta screen-dims"
              >{screen.width}×{screen.height}</span
            >
          {/if}
          <span class="screen-meta screen-ts"
            >{fmtTimestamp(screen.timestamp)}</span
          >
        </div>
        <img
          src={`data:${screen.mime};base64,${screen.data}`}
          alt={screen.monitor_name || `Display ${i + 1}`}
        />
      </div>
    {/each}
  </div>
</main>

<style>
  #screens-pane {
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
  #screens {
    padding: 1.75rem 2rem 2rem;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(520px, 100%), 1fr));
    gap: 1.5rem;
    flex: 1;
    align-content: start;
  }
  @media (max-width: 768px) {
    #screens {
      grid-template-columns: 1fr;
      padding: 1rem;
    }
    .pane-header {
      padding: 0.85rem 1rem;
    }
  }
  .screen-container {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 0.85rem;
    padding: 0.6rem;
    box-shadow: 0 10px 30px -18px rgba(0, 0, 0, 0.6);
    transition:
      transform 0.2s ease,
      border-color 0.2s ease;
  }
  .screen-container:hover {
    transform: translateY(-2px);
    border-color: var(--accent);
  }
  .screen-label {
    font-size: 0.7rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
    padding: 0 0.5rem;
  }
  .screen-container img {
    width: 100%;
    height: auto;
    display: block;
    border-radius: 0.6rem;
    background: #000;
    object-fit: contain;
  }
  .screen-label-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.5rem;
  }
  .screen-meta {
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      monospace;
  }
</style>
