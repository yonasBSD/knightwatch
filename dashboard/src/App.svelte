<script>
  import { onMount } from "svelte";
  import "./global.css";
  import ScreensPane from "./lib/components/ScreensPane.svelte";
  import SystemResourcesPane from "./lib/components/SystemResourcesPane.svelte";
  import ProcessesPane from "./lib/components/ProcessesPane.svelte";
  import SystemdPane from "./lib/components/SystemdPane.svelte";
  import LoginPage from "./lib/components/LoginPage.svelte";
  import { auth, apiFetch } from "./lib/api.js";

  // ── State ──────────────────────────────────────────────────────────
  let activeTab = $state("screens");
  let status = $state("Loading…");
  let statusError = $state(false);
  let info = $state(null);
  let shutdownDisabled = $state(false);
  let shutdownLabel = $state("Shutdown");

  // ── Auth ───────────────────────────────────────────────────────────
  // Show login when auth is required and no valid token exists.
  // Covers first load, explicit logout, and 401 expiry.
  let needsLogin = $derived(info !== null && info.auth_enabled && !$auth.token);

  // When an allow command is true but auth_enabled is false,
  // show a login button in the top bar so the user can authenticate
  // only when they need to perform a protected action.
  let showLoginButton = $derived(
    info !== null &&
      !info.auth_enabled &&
      (info.allow_process_commands ||
        info.allow_screen_commands ||
        info.allow_system_resources_commands) &&
      !$auth.token,
  );

  function onLogin() {
    loadInfo();
  }

  let showLoginPage = $state(false);

  function handleLoginButton() {
    showLoginPage = true;
  }

  function handleLoginPageLogin() {
    showLoginPage = false;
    loadInfo();
  }

  function handleLoginPageBack() {
    showLoginPage = false;
  }

  // Tab visibility driven by info
  let showScreens = $derived(!info || !info.blind);
  let showSystem = $derived(!info || info.system_resources !== false);
  let showProcesses = $derived(
    !info || info.top_processes !== false || (info.pid && info.pid.length > 0),
  );
  let showSystemd = $derived(!info || info.systemd !== false);

  // Tab indicator refs
  let tabnavEl = $state(null);
  let tabEls = $state({});
  let indicatorStyle = $state("width:0;transform:translateX(0)");

  const TAB_NAMES = ["screens", "system", "processes", "systemd"];

  // ── Tab indicator ─────────────────────────────────────────────────
  function moveIndicator(name) {
    const btn = tabEls[name];
    if (!btn || !tabnavEl) return;
    const navRect = tabnavEl.getBoundingClientRect();
    const r = btn.getBoundingClientRect();
    indicatorStyle = `width:${r.width}px;transform:translateX(${r.left - navRect.left - 4}px)`;
  }

  function activateTab(name, { focus = false } = {}) {
    activeTab = name;
    try {
      history.replaceState(null, "", "#" + name);
    } catch {}
    // rAF so the DOM has settled before measuring
    requestAnimationFrame(() => {
      moveIndicator(name);
      if (focus) tabEls[name]?.focus();
    });
  }

  // ── info load ───────────────────────────────────────────────────
  async function loadInfo() {
    try {
      const r = await apiFetch("/api/info");
      if (!r.ok) throw new Error("info fetch failed");
      info = await r.json();
    } catch (e) {
      // If we were redirected to login, don't overwrite info with defaults
      if (e?.message === "Unauthorized") return;
      info = {
        auth_enabled: false,
        blind: false,
        pid: [],
        top_processes: false,
        limit_processes: 5,
        telegram_bot: false,
        system_resources: false,
        systemd: false,
        allow_process_commands: false,
        allow_screen_commands: false,
        allow_system_resources_commands: false,
      };
    }

    // Navigate away from blind tab
    if (info.blind && activeTab === "screens") {
      activateTab("system");
    }

    // Ensure active tab is visible
    requestAnimationFrame(() => {
      const visibleTabs = TAB_NAMES.filter((t) => {
        if (t === "screens" && info.blind) return false;
        if (t === "system" && !info.system_resources) return false;
        if (t === "systemd" && !info.systemd) return false;
        return true;
      });
      if (!visibleTabs.includes(activeTab)) {
        activateTab(visibleTabs[0] ?? "processes");
      } else {
        moveIndicator(activeTab);
      }
    });
  }

  // ── Shutdown ──────────────────────────────────────────────────────
  async function handleShutdown() {
    if (!confirm("Shut down the server?")) return;
    shutdownDisabled = true;
    shutdownLabel = "Shutting down…";
    try {
      await apiFetch("/api/shutdown", { method: "POST" });
    } catch {}
    status = "● OFFLINE · Server shut down";
    statusError = true;
  }

  // ── Init ──────────────────────────────────────────────────────────
  onMount(() => {
    // Initial tab from hash
    const fromHash = (location.hash || "").replace("#", "");
    const initial = TAB_NAMES.includes(fromHash) ? fromHash : "screens";
    requestAnimationFrame(() => activateTab(initial));

    loadInfo();

    const onResize = () => moveIndicator(activeTab);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  });
</script>

{#if needsLogin}
  <LoginPage {onLogin} />
{:else if showLoginPage}
  <LoginPage onLogin={handleLoginPageLogin} onBack={handleLoginPageBack} />
{:else}
  <header id="topbar">
    <div class="topbar-brand">
      <span class="brand-dot"></span>
      <h1>Knight Watch</h1>
      <span id="status" class:error={statusError}>{status}</span>
    </div>

    <div id="tabnav" role="tablist" aria-label="Sections" bind:this={tabnavEl}>
      {#if showScreens}
        <button
          class="tab"
          role="tab"
          aria-selected={activeTab === "screens"}
          onclick={() => activateTab("screens")}
          bind:this={tabEls["screens"]}
        >
          <span class="tab-icon" aria-hidden="true">▦</span>
          <span class="tab-label">Screenshots</span>
        </button>
      {/if}

      {#if showSystem}
        <button
          class="tab"
          role="tab"
          aria-selected={activeTab === "system"}
          onclick={() => activateTab("system")}
          bind:this={tabEls["system"]}
        >
          <span class="tab-icon" aria-hidden="true">◉</span>
          <span class="tab-label">System</span>
        </button>
      {/if}

      {#if showProcesses}
        <button
          class="tab"
          role="tab"
          aria-selected={activeTab === "processes"}
          onclick={() => activateTab("processes")}
          bind:this={tabEls["processes"]}
        >
          <span class="tab-icon" aria-hidden="true">≡</span>
          <span class="tab-label">Processes</span>
        </button>
      {/if}

      {#if showSystemd}
        <button
          class="tab"
          role="tab"
          aria-selected={activeTab === "systemd"}
          onclick={() => activateTab("systemd")}
          bind:this={tabEls["systemd"]}
        >
          <span class="tab-icon" aria-hidden="true">≡</span>
          <span class="tab-label">Systemd</span>
        </button>
      {/if}

      <span class="tab-indicator" aria-hidden="true" style={indicatorStyle}
      ></span>
    </div>

    <div class="topbar-actions">
      {#if info}
        <span
          class="telegram-indicator"
          class:tg-on={info.telegram_bot}
          class:tg-off={!info.telegram_bot}
          title={info.telegram_bot
            ? "Telegram bot is running"
            : "Telegram bot is not running"}
        >
          TG Bot
        </span>
      {/if}
      <button
        id="shutdown-btn"
        title="Shut down the server"
        disabled={shutdownDisabled}
        onclick={handleShutdown}
      >
        <span class="sd-dot"></span>
        {shutdownLabel}
      </button>

      {#if showLoginButton}
        <button
          id="login-btn"
          title="Sign in to perform actions"
          onclick={handleLoginButton}
        >
          <span class="login-icon" aria-hidden="true">⏻</span>
          Sign in
        </button>
      {/if}

      {#if info?.auth_enabled || (info?.allow_process_commands && info?.allow_screen_commands && info?.allow_system_resources_commands && $auth.token)}
        <button id="logout-btn" title="Sign out" onclick={() => auth.logout()}>
          <span class="logout-icon" aria-hidden="true">⏻</span>
          Sign out
        </button>
      {/if}
    </div>
  </header>

  <div id="panes">
    {#if info}
      <section
        class="pane"
        class:active={activeTab === "screens"}
        role="tabpanel"
      >
        <ScreensPane
          active={activeTab === "screens"}
          bind:status
          bind:statusError
          enabled={!info.blind}
          allowScreenCommands={info.allow_screen_commands ?? false}
          isAuthenticated={!!$auth.token}
        />
      </section>

      <section
        class="pane"
        class:active={activeTab === "system"}
        role="tabpanel"
      >
        <SystemResourcesPane
          active={activeTab === "system"}
          enabled={info.system_resources}
          allowSystemResourcesCommands={info.allow_system_resources_commands ??
            false}
          isAuthenticated={!!$auth.token}
        />
      </section>

      <section
        class="pane"
        class:active={activeTab === "processes"}
        role="tabpanel"
      >
        <ProcessesPane
          active={activeTab === "processes"}
          hasPids={info.pid && info.pid.length > 0}
          hasTopProcesses={info.top_processes}
          limitProcesses={info.limit_processes ?? 50}
          allowProcessCommands={info.allow_process_commands ?? false}
          isAuthenticated={!!$auth.token}
        />
      </section>

      <section
        class="pane"
        class:active={activeTab === "systemd"}
        role="tabpanel"
      >
        <SystemdPane active={activeTab === "systemd"} enabled={info.systemd} />
      </section>
    {/if}
  </div>
{/if}

<style>
  /* ── Top bar ───────────────────────────────────────────── */
  #topbar {
    height: var(--topbar-h);
    flex-shrink: 0;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 1rem;
    padding: 0 1.25rem;
    background: rgba(18, 18, 20, 0.85);
    backdrop-filter: saturate(160%) blur(10px);
    -webkit-backdrop-filter: saturate(160%) blur(10px);
    border-bottom: 1px solid var(--border);
    position: relative;
    z-index: 50;
  }

  .topbar-brand {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-width: 0;
  }
  .brand-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 12px var(--accent);
    flex-shrink: 0;
  }
  .topbar-brand h1 {
    font-size: 1rem;
    font-weight: 700;
    color: #fff;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }
  #status {
    color: var(--text-muted);
    font-size: 0.7rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      monospace;
    padding-left: 0.65rem;
    border-left: 1px solid var(--border);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  #status.error {
    color: var(--error);
  }

  .topbar-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.65rem;
  }

  .telegram-indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.68rem;
    font-weight: 700;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      monospace;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.35rem 0.6rem;
    border-radius: 0.5rem;
    border: 1px solid var(--border);
    background: var(--bg-card);
  }
  .telegram-indicator::before {
    content: "✈";
    font-style: normal;
  }
  .telegram-indicator.tg-on {
    color: #34d399;
    border-color: rgba(16, 185, 129, 0.35);
  }
  .telegram-indicator.tg-off {
    color: var(--text-muted);
  }

  #shutdown-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 1px solid var(--error);
    color: var(--error);
    font-size: 0.7rem;
    font-weight: 700;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      monospace;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.45rem 0.8rem;
    border-radius: 0.5rem;
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      transform 0.1s ease;
  }
  #shutdown-btn :global(.sd-dot) {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--error);
    box-shadow: 0 0 8px var(--error);
  }
  #shutdown-btn:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.12);
  }
  #shutdown-btn:active:not(:disabled) {
    transform: translateY(1px);
  }
  #shutdown-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ── Tab nav ──────────────────────────────────────────── */
  #tabnav {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 999px;
    justify-self: center;
  }
  :global(.tab) {
    position: relative;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-family: inherit;
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    padding: 0.5rem 1rem;
    border-radius: 999px;
    cursor: pointer;
    transition: color 0.2s ease;
    white-space: nowrap;
  }
  :global(.tab .tab-icon) {
    font-size: 0.85rem;
    opacity: 0.85;
  }
  :global(.tab:hover) {
    color: var(--text-base);
  }
  :global(.tab[aria-selected="true"]) {
    color: #fff;
  }
  .tab-indicator {
    position: absolute;
    z-index: 1;
    top: 4px;
    bottom: 4px;
    left: 0;
    width: 0;
    border-radius: 999px;
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    box-shadow: 0 4px 14px rgba(59, 130, 246, 0.35);
    transition:
      transform 0.28s cubic-bezier(0.5, 0.05, 0.2, 1),
      width 0.28s cubic-bezier(0.5, 0.05, 0.2, 1);
  }

  /* ── Pane container ───────────────────────────────────── */
  #panes {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
  .pane {
    position: absolute;
    inset: 0;
    display: none;
    flex-direction: column;
    overflow: hidden;
    animation: paneIn 0.25s ease both;
  }
  .pane.active {
    display: flex;
  }
  @keyframes paneIn {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* ── Login button ─────────────────────────────────────── */
  #login-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 1px solid var(--accent);
    color: var(--accent);
    font-size: 0.7rem;
    font-weight: 700;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      monospace;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.45rem 0.8rem;
    border-radius: 0.5rem;
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      transform 0.1s ease;
  }
  #login-btn:hover {
    background: rgba(59, 130, 246, 0.12);
    color: #fff;
  }
  #login-btn:active {
    transform: translateY(1px);
  }
  .login-icon {
    font-size: 0.8rem;
    opacity: 0.75;
  }

  /* ── Logout button ────────────────────────────────────── */
  #logout-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 0.7rem;
    font-weight: 700;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
      monospace;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.45rem 0.8rem;
    border-radius: 0.5rem;
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      border-color 0.15s ease,
      transform 0.1s ease;
  }
  #logout-btn:hover {
    background: var(--bg-card);
    color: var(--text-base);
    border-color: #3f3f46;
  }
  #logout-btn:active {
    transform: translateY(1px);
  }
  .logout-icon {
    font-size: 0.8rem;
    opacity: 0.75;
  }

  /* ── Responsive ───────────────────────────────────────── */
  @media (max-width: 720px) {
    #topbar {
      grid-template-columns: 1fr auto;
      grid-template-rows: auto auto;
      height: auto;
      padding: 0.6rem 0.85rem;
      gap: 0.5rem;
    }
    #tabnav {
      grid-column: 1 / -1;
      justify-self: stretch;
      overflow-x: auto;
    }
    :global(.tab-label) {
      display: none;
    }
    :global(.tab) {
      padding: 0.5rem 0.75rem;
    }
    #status {
      display: none;
    }
  }
</style>
