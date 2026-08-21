// Compatibility shim: maps the original Electron-style globals (W, S, Welcome,
// window.api) onto Tauri 2's invoke/event APIs. Lets us re-use the renderer
// JS verbatim from the Electron version.
(function () {
  'use strict';

  const T = window.__TAURI__;
  if (!T) {
    console.error('[tauri-shim] window.__TAURI__ is missing — is `withGlobalTauri` enabled in tauri.conf.json?');
    return;
  }
  const invoke = T.core ? T.core.invoke : T.invoke;
  const listen = T.event && T.event.listen;

  // Generic API used by settings.js and the quran widget chrome polyfill.
  // The original code passed channel names like `s:get`, `q:store:set`. Tauri 2
  // command names cannot contain colons, so we translate `:` → `_` here.
  function translate(channel) {
    return String(channel).replace(/:/g, '_').replace(/-/g, '_');
  }

  function apiInvoke(channel, payload) {
    const cmd = translate(channel);
    if (payload === undefined) return invoke(cmd);
    if (payload === null) return invoke(cmd, { keys: null });
    if (Array.isArray(payload)) return invoke(cmd, { keys: payload });

    // For commands that take a single typed argument (jump index, search query),
    // Tauri 2 expects `{ argName: value }`. We map the well-known commands here.
    switch (cmd) {
      case 's_jump':
        return invoke(cmd, { index: payload });
      case 's_search':
        return invoke(cmd, { query: payload });
      case 'q_store_get':
      case 'q_store_remove':
        return invoke(cmd, { keys: payload });
      case 'q_store_set':
        return invoke(cmd, { data: payload });
      case 's_save':
        return invoke(cmd, { payload: payload });
      case 'q_bg_message':
        return invoke(cmd, { req: payload });
      case 'welcome_done':
        return invoke(cmd, { autoLaunch: !!payload.autoLaunch });
      default:
        return invoke(cmd, payload);
    }
  }

  const _listeners = new Map();
  window.api = {
    invoke: apiInvoke,
    receive: (channel, fn) => {
      const evt = translate(channel);
      if (listen) {
        // Clean up previous listener if re-registered
        if (_listeners.has(evt)) {
          const old = _listeners.get(evt);
          if (typeof old === 'function') old();
        }
        const unlistenPromise = listen(evt, (e) => fn(e.payload));
        _listeners.set(evt, () => unlistenPromise.then(u => typeof u === 'function' && u()));
      }
    },
  };

  // Widget bridge — used by widget.js.
  let _hadithUnlisten = null;
  window.W = {
    onHadith: async (cb) => {
      if (listen) {
        // Clean up previous listener
        if (typeof _hadithUnlisten === 'function') {
          _hadithUnlisten();
          _hadithUnlisten = null;
        }
        _hadithUnlisten = await listen('hadith', (e) => cb(e.payload));
      }
      invoke('widget_ready').catch(() => {});
    },
    hide: () => invoke('w_hide').catch(() => {}),
    memorized: (i) => invoke('w_memorized', { index: i }).catch(() => {}),
    forgot: (i) => invoke('w_forgot', { index: i }).catch(() => {}),
    next: (i) => invoke('w_next', { index: i }).catch(() => {}),
    prev: (i) => invoke('w_prev', { index: i }).catch(() => {}),
  };

  // Settings bridge — used by settings.js.
  window.S = {
    get: () => invoke('s_get'),
    save: (d) => invoke('s_save', { payload: d }),
    reset: () => invoke('s_reset'),
    showNow: () => invoke('s_show_now'),
    jump: (i) => invoke('s_jump', { index: i }),
    search: (q) => invoke('s_search', { query: q }),
    backup: () => invoke('s_backup'),
    restore: () => invoke('s_restore'),
    resetGeometry: () => invoke('s_reset_geometry'),
  };

  // Welcome bridge — used by welcome.js.
  window.Welcome = {
    done: (d) => invoke('welcome_done', { autoLaunch: !!(d && d.autoLaunch) }),
  };
})();
