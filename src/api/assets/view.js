const statusEl = document.getElementById("status");
const screensDiv = document.getElementById("screens");
const screensPane = document.getElementById("screens-pane");
const rootSection = document.getElementById("root-section");
const workBanner = document.getElementById("work-banner");
const topList = document.getElementById("top-processes-list");
const topProcessesSection = document.getElementById("top-processes-section");
const topSortSelect = document.getElementById("top-sort-select");
const topLimitInput = document.getElementById("top-limit-input");
const telegramIndicator = document.getElementById("telegram-indicator");
const systemPanel = document.getElementById("system-panel");

// ── Config ─────────────────────────────────────────────────────────

let config = null;

async function loadConfig() {
  try {
    const r = await fetch("/config");
    if (!r.ok) throw new Error("config fetch failed");
    config = await r.json();
  } catch {
    config = {
      blind: false,
      pid: null,
      top_processes: false,
      limit_processes: 5,
      telegram_bot: false,
      system_monitor: false,
    };
  }

  // Telegram indicator
  if (telegramIndicator) {
    telegramIndicator.style.display = "inline-flex";
    if (config.telegram_bot) {
      telegramIndicator.textContent = "TG Bot";
      telegramIndicator.className = "telegram-indicator tg-on";
      telegramIndicator.title = "Telegram bot is running";
    } else {
      telegramIndicator.textContent = "TG Bot";
      telegramIndicator.className = "telegram-indicator tg-off";
      telegramIndicator.title = "Telegram bot is not running";
    }
  }

  // Hide screenshots pane if blind
  if (config.blind) {
    screensPane.style.display = "none";
  }

  // Hide process pane sections based on config
  if (config.pid.length === 0) {
    rootSection.style.display = "none";
    workBanner.style.display = "none";
  }

  if (!config.top_processes) {
    topProcessesSection.style.display = "none";
  }

  // Clamp the limit input max to server's supported maximum
  if (topLimitInput && config.limit_processes != null) {
    topLimitInput.max = config.limit_processes;
    if (parseInt(topLimitInput.value) > config.limit_processes) {
      topLimitInput.value = config.limit_processes;
    }
  }

  if (!config.system_monitor) {
    systemPanel.style.display = "none";
  }
}

// ── Helpers ────────────────────────────────────────────────────────

function statePill(state) {
  const cls =
    state === "running"
      ? "state-running"
      : state === "sleeping"
        ? "state-sleeping"
        : state === "gone"
          ? "state-gone"
          : "state-other";
  return `<span class="state-pill ${cls}">${state}</span>`;
}

function metaItem(label, value) {
  return `<div class="proc-meta-item">
    <span class="label">${label}</span>
    <span class="value">${value}</span>
  </div>`;
}

function fmtBytes(n) {
  if (n == null) return "—";
  if (n >= 1073741824) return (n / 1073741824).toFixed(1) + " GB";
  if (n >= 1048576) return (n / 1048576).toFixed(1) + " MB";
  if (n >= 1024) return (n / 1024).toFixed(1) + " KB";
  return n + " B";
}

function fmtTimestamp(ts) {
  if (!ts) return "—";
  try {
    return new Date(ts).toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return ts;
  }
}

function buildCard(proc, isRoot = false) {
  // ── Linux-only extras ──────────────────────────────────────────────
  let linuxExtras = "";

  if (proc.cmdline && proc.cmdline.length > 0) {
    const cmd = proc.cmdline.join(" ");
    linuxExtras += `<div class="proc-cmdline" title="${cmd}">${cmd}</div>`;
  }

  const hasIO = proc.io_stats != null;
  const hasCwd = proc.cwd != null;
  const hasFds = proc.open_fds != null;

  if (hasCwd || hasFds || hasIO) {
    linuxExtras += `<div class="proc-meta proc-meta-linux">`;
    if (hasCwd)
      linuxExtras += metaItem(
        "CWD",
        `<span title="${proc.cwd}">${proc.cwd}</span>`,
      );
    if (hasFds) linuxExtras += metaItem("FDs", proc.open_fds);
    if (hasIO) {
      linuxExtras += metaItem("READ", fmtBytes(proc.io_stats.read_bytes));
      linuxExtras += metaItem("WRITE", fmtBytes(proc.io_stats.write_bytes));
    }
    linuxExtras += `</div>`;
  }

  if (proc.open_files && proc.open_files.length > 0) {
    const fdTypeColor = {
      file: "#a78bfa",
      socket: "#34d399",
      pipe: "#fbbf24",
      other: "#a1a1aa",
    };
    const rows = proc.open_files
      .map((f) => {
        const color = fdTypeColor[f.fd_type] || "#a1a1aa";
        return `<div class="fd-row">
          <span class="fd-num">${f.fd}</span>
          <span class="fd-type" style="color:${color}">${f.fd_type}</span>
          <span class="fd-target" title="${f.target}">${f.target}</span>
        </div>`;
      })
      .join("");
    linuxExtras += `
      <div class="fd-section">
        <div class="fd-section-header">
          <span>Open File Descriptors</span>
          <span class="count-badge">${proc.open_files.length}</span>
        </div>
        <div class="fd-list">${rows}</div>
      </div>`;
  }

  return `
    <div class="proc-card ${isRoot ? "root-card" : ""}">
      <div class="proc-header">
        <div class="proc-name" title="${proc.name} (PID ${proc.pid})">
          ${isRoot ? "⬢ " : ""}${proc.name}
        </div>
        ${statePill(proc.state)}
      </div>
      <div class="proc-meta">
        ${metaItem("PID", proc.pid)}
        ${metaItem("CPU", proc.cpu_usage.toFixed(1) + "%")}
        ${metaItem("MEM", proc.memory_human)}
      </div>
      ${linuxExtras}
    </div>`;
}

// ── Screenshot refresh ─────────────────────────────────────────────

function refreshScreenshots() {
  const start = Date.now();
  fetch("/screenshot")
    .then((r) => {
      if (!r.ok) throw new Error("HTTP error");
      return r.json();
    })
    .then((data) => {
      data.screens.forEach((screen, i) => {
        const containerId = `screen-${screen.monitor_id ?? i}`;
        let container = document.getElementById(containerId);

        if (!container) {
          container = document.createElement("div");
          container.id = containerId;
          container.className = "screen-container";
          container.innerHTML = `
            <div class="screen-label-row">
              <span class="screen-label screen-name"></span>
              <span class="screen-meta screen-dims"></span>
              <span class="screen-meta screen-ts"></span>
            </div>
            <img alt="" />`;
          screensDiv.appendChild(container);
        }

        // Update label fields
        const nameEl = container.querySelector(".screen-name");
        const dimsEl = container.querySelector(".screen-dims");
        const tsEl = container.querySelector(".screen-ts");

        if (nameEl)
          nameEl.textContent = screen.monitor_name || `Display ${i + 1}`;
        if (dimsEl && screen.width && screen.height)
          dimsEl.textContent = `${screen.width}×${screen.height}`;
        if (tsEl) tsEl.textContent = fmtTimestamp(screen.timestamp);

        // Only swap src if the image actually changed
        const img = container.querySelector("img");
        img.alt = screen.monitor_name || `Display ${i + 1}`;
        const newSrc = `data:${screen.mime};base64,${screen.data}`;
        if (img.src !== newSrc) img.src = newSrc;
      });

      // Remove stale containers
      screensDiv.querySelectorAll(".screen-container").forEach((el) => {
        const idx = [
          ...screensDiv.querySelectorAll(".screen-container"),
        ].indexOf(el);
        if (idx >= data.screens.length) el.remove();
      });

      const elapsed = Date.now() - start;
      statusEl.textContent = `● LIVE · ${data.screens.length} SCREEN${data.screens.length > 1 ? "S" : ""} · ${elapsed}MS`;
    })
    .catch(() => {
      statusEl.innerHTML = `<span style="color:var(--error)">● OFFLINE · ${new Date().toLocaleTimeString()}</span>`;
    });
}

// ── Process refresh ────────────────────────────────────────────────

const detailsOpenState = new Set();

async function refreshProcess() {
  try {
    const existingDetails = rootSection.querySelectorAll(
      "details.children-group",
    );
    existingDetails.forEach((details) => {
      if (details.dataset.pid) {
        if (details.open) {
          detailsOpenState.add(details.dataset.pid);
        } else {
          detailsOpenState.delete(details.dataset.pid);
        }
      }
    });

    const rIds = await fetch("/root_pids");
    if (!rIds.ok) throw new Error("HTTP error");
    const pids = await rIds.json();

    if (pids.length === 0) {
      workBanner.classList.remove("visible");
      rootSection.innerHTML = `<div class="muted">No process tracker running.</div>`;
      return;
    }

    let rootsHtml = "";
    let allWorkDone = true;

    for (const pid of pids) {
      try {
        const r = await fetch(`/process/${pid}`);
        if (!r.ok) continue;
        const data = await r.json();

        allWorkDone = allWorkDone && data.work_done;

        rootsHtml += `<div class="process-group">`;

        if (data.root) {
          rootsHtml += buildCard(data.root, true);
        } else {
          rootsHtml += `<div class="muted">Root process ${pid} exited</div>`;
        }

        if (data.child_count > 0) {
          const isOpen = detailsOpenState.has(String(pid)) ? "open" : "";
          rootsHtml += `
            <details class="children-group" data-pid="${pid}" ${isOpen} style="margin-top: 0.5rem; margin-left: 0.75rem;">
              <summary class="section-header" style="margin-top: 0; cursor: pointer; user-select: none;">
                Children <span class="count-badge">${data.child_count}</span>
                <span class="muted" style="margin-left:auto; font-size: 0.7rem; font-weight: normal;">(click to toggle)</span>
              </summary>
              <div style="border-left: 2px solid var(--border); padding-left: 0.75rem; margin-top: 0.5rem; display: flex; flex-direction: column; gap: 0.5rem;">
                ${data.children.map((c) => buildCard(c)).join("")}
              </div>
            </details>`;
        }

        rootsHtml += `</div>`;
      } catch (err) {
        continue;
      }
    }

    workBanner.classList.toggle("visible", pids.length > 0 && allWorkDone);
    rootSection.style.display = "flex";
    rootSection.style.flexDirection = "column";
    rootSection.style.gap = "1rem";
    rootSection.innerHTML = rootsHtml;
  } catch (err) {
    rootSection.innerHTML = `<div class="muted">Monitor disabled</div>`;
  }
}

// ── Top Processes refresh ──────────────────────────────────────────

function refreshTopProcesses() {
  const sort = topSortSelect?.value || "cpu";
  const limit = topLimitInput?.value || 5;

  fetch(`/top-processes?sort=${sort}&limit=${limit}`)
    .then((r) => {
      if (!r.ok) throw new Error("HTTP error");
      return r.json();
    })
    .then((data) => {
      if (data && data.length > 0) {
        topList.innerHTML = data.map((c) => buildCard(c)).join("");
      } else {
        topList.innerHTML = `<div class="muted">No processes found</div>`;
      }
    })
    .catch(() => {
      topList.innerHTML = `<div class="muted">Failed to load top processes</div>`;
    });
}

// ── Shutdown ───────────────────────────────────────────────────────

document.getElementById("shutdown-btn").addEventListener("click", () => {
  if (!confirm("Shut down the server?")) return;
  const btn = document.getElementById("shutdown-btn");
  btn.disabled = true;
  btn.textContent = "Shutting down…";
  fetch("/shutdown", { method: "POST" })
    .then(() => {
      statusEl.innerHTML = `<span style="color:var(--error)">● OFFLINE · Server shut down</span>`;
    })
    .catch(() => {
      // Server likely closed the connection immediately — that's expected
      statusEl.innerHTML = `<span style="color:var(--error)">● OFFLINE · Server shut down</span>`;
    });
});

// ── System helpers (mirrors Rust formatting) ───────────────────────

function formatBytes(bytes) {
  const KB = 1024,
    MB = KB * 1024,
    GB = MB * 1024,
    TB = GB * 1024;
  if (bytes >= TB) return (bytes / TB).toFixed(1) + " TB";
  if (bytes >= GB) return (bytes / GB).toFixed(1) + " GB";
  if (bytes >= MB) return (bytes / MB).toFixed(1) + " MB";
  if (bytes >= KB) return (bytes / KB).toFixed(1) + " KB";
  return bytes + " B";
}

function formatUptime(secs) {
  const days = Math.floor(secs / 86400);
  const hours = Math.floor((secs % 86400) / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  return days > 0 ? `${days}d ${hours}h ${mins}m` : `${hours}h ${mins}m`;
}

// ── System panel rendering ─────────────────────────────────────────

function kv(label, value, cls = "") {
  return `<div class="sys-kv">
    <span class="sk">${label}</span>
    <span class="sv${cls ? " " + cls : ""}">${value ?? "—"}</span>
  </div>`;
}

function usageBar(label, pct, extraClass = "") {
  const fill = pct == null ? 0 : Math.min(100, pct);
  const colorCls = fill >= 90 ? "crit" : fill >= 75 ? "warn" : "";
  return `<div class="sys-bar-row">
    <span class="sys-bar-label">${label}</span>
    <div class="sys-bar-track"><div class="sys-bar-fill ${colorCls}" style="width:${fill.toFixed(1)}%"></div></div>
    <span class="sys-bar-val">${fill.toFixed(1)}%</span>
  </div>`;
}

function renderSystemSnapshot(snap) {
  // ── Host ────────────────────────────────────────────────────────
  const h = snap.host;
  const healthCls =
    snap.health === "healthy"
      ? "health-healthy"
      : snap.health === "warning"
        ? "health-warning"
        : "health-critical";
  document.getElementById("sys-host-grid").innerHTML = [
    kv("Hostname", h.hostname),
    kv("OS", h.os_name),
    kv("Kernel", h.kernel_version),
    kv("Arch", h.cpu_arch),
    kv("Uptime", formatUptime(h.uptime_secs)),
    kv("Procs", h.process_count),
    kv("Health", snap.health, healthCls),
  ].join("");

  // ── CPU ─────────────────────────────────────────────────────────
  const cpu = snap.cpu;
  let cpuGridHtml = [
    kv("Brand", cpu.brand),
    kv("Cores", cpu.physical_core_count ?? cpu.cores.length),
    kv("Freq", cpu.frequency_mhz + " MHz"),
    kv("Usage", cpu.usage_percent.toFixed(1) + "%"),
  ];
  if (cpu.load_avg) {
    cpuGridHtml.push(kv("Load 1m", cpu.load_avg.one.toFixed(2)));
    cpuGridHtml.push(kv("Load 5m", cpu.load_avg.five.toFixed(2)));
  }
  document.getElementById("sys-cpu-grid").innerHTML = cpuGridHtml.join("");

  // Core bars
  const coresEl = document.getElementById("sys-cpu-cores");
  const maxH = 28;
  coresEl.innerHTML = cpu.cores
    .map((c) => {
      const h = Math.max(2, (c.usage_percent / 100) * maxH);
      const col =
        c.usage_percent >= 90
          ? "var(--error)"
          : c.usage_percent >= 75
            ? "var(--warning)"
            : "var(--accent)";
      return `<div class="sys-core-bar" title="${c.name}: ${c.usage_percent.toFixed(1)}%"
      style="height:${h}px;background:${col}"></div>`;
    })
    .join("");

  // ── Memory ──────────────────────────────────────────────────────
  const mem = snap.memory;
  document.getElementById("sys-mem-grid").innerHTML = `
    <div class="sys-bar-wrap" style="grid-column:1/-1">
      ${usageBar("RAM", mem.used_percent)}
      ${mem.swap_used_percent != null ? usageBar("SWAP", mem.swap_used_percent) : ""}
    </div>
    ${kv("Total", formatBytes(mem.total_bytes))}
    ${kv("Used", formatBytes(mem.used_bytes))}
    ${kv("Free", formatBytes(mem.free_bytes))}
    ${kv("Avail", formatBytes(mem.available_bytes))}
    ${mem.swap_total_bytes > 0 ? kv("Swap Total", formatBytes(mem.swap_total_bytes)) : ""}
    ${mem.swap_used_bytes > 0 ? kv("Swap Used", formatBytes(mem.swap_used_bytes)) : ""}
  `;

  // ── Disks ───────────────────────────────────────────────────────
  document.getElementById("sys-disk-list").innerHTML = snap.disks
    .map(
      (d) => `
    <div class="sys-item">
      <span class="sys-item-name" title="${d.name}">${d.mount_point}</span>
      <span class="sys-item-sub">${d.file_system} · ${d.kind}${d.is_removable ? " · removable" : ""}</span>
      <div class="sys-bar-track" style="min-width:140px">
        <div class="sys-bar-fill ${d.used_percent >= 95 ? "crit" : d.used_percent >= 80 ? "warn" : ""}"
          style="width:${Math.min(100, d.used_percent).toFixed(1)}%"></div>
      </div>
      <div style="display:flex;gap:0.75rem">
        ${kv("Used", formatBytes(d.used_bytes))}
        ${kv("Free", formatBytes(d.available_bytes))}
        ${kv("Total", formatBytes(d.total_bytes))}
      </div>
    </div>
  `,
    )
    .join("");

  // ── Network ─────────────────────────────────────────────────────
  const nets = snap.networks.filter(
    (n) => n.rx_total_bytes > 0 || n.tx_total_bytes > 0,
  );
  document.getElementById("sys-net-list").innerHTML =
    nets.length === 0
      ? `<span class="sys-item-sub">No active interfaces</span>`
      : nets
          .map(
            (n) => `
      <div class="sys-item">
        <span class="sys-item-name">${n.interface}</span>
        <div class="sys-net-io">
          <div class="sys-net-badge"><span class="dir">↓</span><span class="bw">${formatBytes(n.rx_bytes_per_sec)}/s</span></div>
          <div class="sys-net-badge"><span class="dir">↑</span><span class="bw">${formatBytes(n.tx_bytes_per_sec)}/s</span></div>
        </div>
        <div style="display:flex;gap:0.75rem">
          ${kv("RX Total", formatBytes(n.rx_total_bytes))}
          ${kv("TX Total", formatBytes(n.tx_total_bytes))}
        </div>
      </div>
    `,
          )
          .join("");

  // ── GPU ─────────────────────────────────────────────────────────
  const gpuSection = document.getElementById("sys-gpu-section");
  if (snap.gpus && snap.gpus.length > 0) {
    gpuSection.style.display = "";
    document.getElementById("sys-gpu-list").innerHTML = snap.gpus
      .map(
        (g) => `
      <div class="sys-item">
        <span class="sys-item-name">${g.name}</span>
        ${
          g.usage_percent != null
            ? `<div class="sys-bar-track" style="min-width:120px">
          <div class="sys-bar-fill ${g.usage_percent >= 90 ? "crit" : g.usage_percent >= 75 ? "warn" : ""}"
            style="width:${Math.min(100, g.usage_percent).toFixed(1)}%"></div>
        </div>`
            : ""
        }
        <div style="display:flex;gap:0.75rem;flex-wrap:wrap">
          ${g.usage_percent != null ? kv("Core", g.usage_percent.toFixed(1) + "%") : ""}
          ${g.vram_used_human != null ? kv("VRAM", g.vram_used_human + " / " + (g.vram_total_human ?? "?")) : ""}
          ${g.temperature_celsius != null ? kv("Temp", g.temperature_celsius.toFixed(0) + "°C") : ""}
          ${g.power_draw_watts != null ? kv("Power", g.power_draw_watts.toFixed(0) + "W") : ""}
          ${g.fan_speed_percent != null ? kv("Fan", g.fan_speed_percent.toFixed(0) + "%") : ""}
        </div>
      </div>
    `,
      )
      .join("");
  } else {
    gpuSection.style.display = "none";
  }

  // ── Battery ─────────────────────────────────────────────────────
  const batSection = document.getElementById("sys-battery-section");
  if (snap.battery) {
    batSection.style.display = "";
    const bat = snap.battery;
    document.getElementById("sys-battery-grid").innerHTML = `
      <div class="sys-bar-wrap" style="grid-column:1/-1">
        ${usageBar("Charge", bat.charge_percent)}
      </div>
      ${kv("State", bat.state)}
      ${bat.time_to_empty_secs != null ? kv("Empty in", formatUptime(bat.time_to_empty_secs)) : ""}
      ${bat.time_to_full_secs != null ? kv("Full in", formatUptime(bat.time_to_full_secs)) : ""}
      ${bat.power_draw_watts != null ? kv("Draw", bat.power_draw_watts.toFixed(1) + "W") : ""}
      ${bat.health_percent != null ? kv("Health", bat.health_percent.toFixed(0) + "%") : ""}
      ${bat.cycle_count != null ? kv("Cycles", bat.cycle_count) : ""}
    `;
  } else {
    batSection.style.display = "none";
  }

  // ── Thermals ────────────────────────────────────────────────────
  const thermalSection = document.getElementById("sys-thermal-section");
  if (snap.temperatures && snap.temperatures.length > 0) {
    thermalSection.style.display = "";
    document.getElementById("sys-thermal-list").innerHTML = snap.temperatures
      .filter((t) => t.temperature_celsius != null)
      .map((t) => {
        const temp = t.temperature_celsius;
        const crit = t.temperature_critical_celsius;
        const isCrit = crit != null && temp >= crit;
        const isWarn = !isCrit && temp >= 80;
        return `<div class="sys-thermal-chip">
          <span class="sys-thermal-label" title="${t.label}">${t.label}</span>
          <span class="sys-thermal-val${isCrit ? " crit" : isWarn ? " warn" : ""}">${temp.toFixed(0)}°C</span>
          ${crit != null ? `<span class="sys-thermal-label" style="min-width:0">/ ${crit.toFixed(0)}°C</span>` : ""}
        </div>`;
      })
      .join("");
  } else {
    thermalSection.style.display = "none";
  }
}

// ── System refresh ─────────────────────────────────────────────────

function refreshSystem() {
  fetch("/system")
    .then((r) => {
      if (!r.ok) throw new Error("HTTP error");
      return r.json();
    })
    .then((snap) => renderSystemSnapshot(snap))
    .catch(() => {
      /* silently skip if endpoint unavailable */
    });
}

let screenshotInterval = null;
let processInterval = null;
let topInterval = null;
let systemInterval = null;

loadConfig().then(() => {
  if (!config.blind) {
    refreshScreenshots();
    screenshotInterval = setInterval(refreshScreenshots, 2000);
  }

  refreshProcess();
  processInterval = setInterval(refreshProcess, 2000);

  if (config.top_processes) {
    refreshTopProcesses();
    topSortSelect?.addEventListener("change", refreshTopProcesses);
    topLimitInput?.addEventListener("change", refreshTopProcesses);
    topInterval = setInterval(refreshTopProcesses, 2000);
  }

  // System panel — always enabled; gracefully no-ops if /system is absent
  if (config.system_monitor) {
    refreshSystem();
    systemInterval = setInterval(refreshSystem, 2000);
  }
});
