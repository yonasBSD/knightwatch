import { writable, get } from "svelte/store";

// ── Auth store ────────────────────────────────────────────────────────────────
// Shape: { token: string | null, loggedOut: boolean }
// `loggedOut` flips to true on explicit logout or a 401, telling App to show
// the login screen even before info is loaded.

function createAuthStore() {
  const { subscribe, set, update } = writable({
    token: sessionStorage.getItem("auth_token") ?? null,
    loggedOut: false,
  });

  return {
    subscribe,
    login(token) {
      sessionStorage.setItem("auth_token", token);
      set({ token, loggedOut: false });
    },
    async logout() {
      const { token } = get(auth);
      try {
        await fetch("/api/auth/logout", {
          method: "POST",
          headers: token ? { Authorization: `Bearer ${token}` } : {},
        });
      } catch {}
      sessionStorage.removeItem("auth_token");
      set({ token: null, loggedOut: true });
    },
    _expire() {
      sessionStorage.removeItem("auth_token");
      update((s) => ({ ...s, token: null, loggedOut: true }));
    },
  };
}

export const auth = createAuthStore();

export async function apiFetch(url, opts = {}) {
  const { token } = get(auth);

  const res = await fetch(url, {
    ...opts,
    headers: {
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(opts.headers ?? {}),
    },
  });

  if (res.status === 401) {
    auth._expire();
    throw new Error("Unauthorized");
  }

  return res;
}