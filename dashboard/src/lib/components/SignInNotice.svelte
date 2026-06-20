<script>
  let { name } = $props();
</script>

<div class="cmd-auth-notice">
  <span aria-hidden="true">🔒</span> Sign in to use {name} commands
</div>

<style>
  .cmd-auth-notice {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.72rem;
    color: var(--text-muted);
    background: var(--bg-card);
    border: 1px solid var(--border-soft);
    border-radius: 6px;
    padding: 0.25rem 0.7rem;
  }
</style>
