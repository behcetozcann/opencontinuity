// Tauri köprüsü ve uygulama durumu. Görünümler `subscribe` ile her
// değişimde yeniden çizilir; `view` Rust'tan gelir, `ui` yalnız arayüzündür.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

export { invoke };

export const isMac = navigator.userAgent.includes("Mac");

export const store = {
  view: null,
  version: "",
  ui: { settingsOpen: false, celebrating: false, awaitingPeer: false, pairedSeen: 0 },
};

const subscribers = new Set();

export function subscribe(render) {
  subscribers.add(render);
}

export function notify() {
  subscribers.forEach((render) => render(store));
}

export function setUi(patch) {
  Object.assign(store.ui, patch);
  notify();
}

export async function start() {
  await listen("view", (event) => {
    store.view = event.payload;
    notify();
  });
  await listen("open-settings", () => setUi({ settingsOpen: true }));
  store.version = await invoke("app_version");
  store.view = await invoke("get_view");
  notify();
}

/** Komutu çalıştırır; hata olursa metni döner (arayüz gösterir). */
export async function run(command, args) {
  try {
    await invoke(command, args);
    return null;
  } catch (error) {
    return String(error);
  }
}
