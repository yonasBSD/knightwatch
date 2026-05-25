<script>
  import { auth } from "../api.js";

  let { onLogin } = $props();

  let username = $state("");
  let password = $state("");
  let error = $state("");
  let loading = $state(false);

  async function handleSubmit() {
    if (!username || !password) {
      error = "Username and password are required.";
      return;
    }
    loading = true;
    error = "";
    try {
      const r = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password }),
      });
      if (r.status === 401) {
        error = "Invalid username or password.";
        return;
      }
      if (!r.ok) {
        error = "Login failed. Please try again.";
        return;
      }
      const { token } = await r.json();
      auth.login(token);
      onLogin();
    } catch {
      error = "Could not reach the server.";
    } finally {
      loading = false;
    }
  }

  function onKeydown(e) {
    if (e.key === "Enter") handleSubmit();
  }
</script>

<div class="login-backdrop">
  <div class="login-card">
    <div class="login-brand">
      <span class="brand-dot"></span>
      <h1>Knight Watch</h1>
    </div>
    <p class="login-sub">Sign in to continue</p>

    <div class="field">
      <label for="username">Username</label>
      <input
        id="username"
        type="text"
        autocomplete="username"
        placeholder="username"
        bind:value={username}
        onkeydown={onKeydown}
        disabled={loading}
      />
    </div>

    <div class="field">
      <label for="password">Password</label>
      <input
        id="password"
        type="password"
        autocomplete="current-password"
        placeholder="••••••••"
        bind:value={password}
        onkeydown={onKeydown}
        disabled={loading}
      />
    </div>

    {#if error}
      <p class="login-error">{error}</p>
    {/if}

    <button class="login-btn" onclick={handleSubmit} disabled={loading}>
      {#if loading}
        <span class="spinner"></span> Signing in…
      {:else}
        Sign in
      {/if}
    </button>
  </div>
</div>

<style>
  .login-backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background:
      radial-gradient(
        1200px 600px at 10% -10%,
        rgba(59, 130, 246, 0.08),
        transparent 60%
      ),
      radial-gradient(
        900px 500px at 110% 10%,
        rgba(139, 92, 246, 0.06),
        transparent 60%
      ),
      var(--bg-body);
    z-index: 200;
  }

  .login-card {
    width: 100%;
    max-width: 360px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 1rem;
    padding: 2rem 2rem 1.75rem;
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
    box-shadow:
      0 0 0 1px var(--border-soft),
      0 24px 60px rgba(0, 0, 0, 0.5);
  }

  .login-brand {
    display: flex;
    align-items: center;
    gap: 0.65rem;
  }
  .brand-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 12px var(--accent);
    flex-shrink: 0;
  }
  .login-brand h1 {
    font-size: 1rem;
    font-weight: 700;
    color: #fff;
    letter-spacing: 0.02em;
  }

  .login-sub {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: -0.4rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .field label {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-muted);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }
  .field input {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    color: var(--text-base);
    font-family: inherit;
    font-size: 0.875rem;
    padding: 0.6rem 0.75rem;
    outline: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
    width: 100%;
  }
  .field input::placeholder {
    color: #52525b;
  }
  .field input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-glow);
  }
  .field input:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .login-error {
    font-size: 0.75rem;
    color: var(--error);
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 0.4rem;
    padding: 0.45rem 0.65rem;
  }

  .login-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    background: var(--accent);
    border: none;
    border-radius: 0.5rem;
    color: #fff;
    cursor: pointer;
    font-family: inherit;
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 0.65rem 1rem;
    transition: opacity 0.15s ease, transform 0.1s ease;
    margin-top: 0.25rem;
  }
  .login-btn:hover:not(:disabled) {
    opacity: 0.88;
  }
  .login-btn:active:not(:disabled) {
    transform: translateY(1px);
  }
  .login-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255, 255, 255, 0.35);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>