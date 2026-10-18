// Ayarlar paneli: Genel, Eşleşmiş cihaz, Güncellemeler, Sorun giderme, Hakkında.
import { invoke, run, setUi, store } from "../api.js";
import { renderUpdate } from "../components/update.js";

const $ = (id) => document.getElementById(id);

async function loadAutostart() {
  try {
    $("autostart").checked = await invoke("autostart_enabled");
    $("autostart-error").hidden = true;
  } catch (error) {
    showError($("autostart-error"), `Başlangıç ayarı okunamadı: ${error}`);
  }
}

function showError(element, message) {
  element.hidden = false;
  element.textContent = message;
}

function showLogResult(message) {
  $("log-result").hidden = false;
  $("log-result").textContent = message;
}

async function loadLog() {
  try {
    const text = await invoke("read_log");
    $("log").textContent = text;
    return text;
  } catch (error) {
    $("log").textContent = `Kayıt okunamadı: ${error}`;
    return null;
  }
}

function resetUnpair() {
  $("unpair-ask").hidden = false;
  $("unpair-confirm").hidden = true;
}

export function bindSettings() {
  $("close-settings").addEventListener("click", () => setUi({ settingsOpen: false }));
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && store.ui.settingsOpen) setUi({ settingsOpen: false });
  });
  $("autostart").addEventListener("change", async (event) => {
    const error = await run("set_autostart", { enabled: event.target.checked });
    if (error) showError($("autostart-error"), `Başlangıç ayarı değiştirilemedi: ${error}`);
    loadAutostart();
  });
  $("open-files").addEventListener("click", async () => {
    const error = await run("open_files_dir");
    if (error) showError($("open-files-error"), `Klasör açılamadı: ${error}`);
    else $("open-files-error").hidden = true;
  });
  $("unpair").addEventListener("click", () => {
    $("unpair-ask").hidden = true;
    $("unpair-confirm").hidden = false;
  });
  $("unpair-no").addEventListener("click", resetUnpair);
  $("unpair-yes").addEventListener("click", () => {
    resetUnpair();
    setUi({ settingsOpen: false });
    invoke("unpair");
  });
  $("log-copy").addEventListener("click", async () => {
    const text = await loadLog();
    if (text === null) return;
    try {
      await navigator.clipboard.writeText(text);
      showLogResult("Kayıt panoya kopyalandı; e-postaya ya da sohbete yapıştırabilirsiniz.");
    } catch (error) {
      showLogResult(`Kopyalanamadı: ${error}`);
    }
  });
  $("log-save").addEventListener("click", async () => {
    try {
      showLogResult(`Kaydedildi: ${await invoke("save_log_to_desktop")}`);
    } catch (error) {
      showLogResult(`Kaydedilemedi: ${error}`);
    }
  });
  $("log-reveal").addEventListener("click", async () => {
    const error = await run("reveal_log");
    if (error) showLogResult(error);
  });
  $("log-details").addEventListener("toggle", (event) => {
    if (event.target.open) loadLog();
  });
  $("open-project").addEventListener("click", () => invoke("open_project_page"));
}

let wasOpen = false;

export function renderSettings({ view, ui, version }) {
  const sheet = $("settings");
  sheet.classList.toggle("open", ui.settingsOpen);
  document.getElementById("app").classList.toggle("sheet-open", ui.settingsOpen);
  sheet.setAttribute("aria-hidden", String(!ui.settingsOpen));
  if (ui.settingsOpen && !wasOpen) {
    loadAutostart();
    $("log-result").hidden = true;
    sheet.querySelector(".sheet-body").scrollTop = 0;
  }
  if (ui.settingsOpen && ui.focusTroubleshoot) {
    ui.focusTroubleshoot = false;
    $("log-details").open = true;
    $("settings-troubleshoot").scrollIntoView({ block: "start" });
  }
  wasOpen = ui.settingsOpen;

  const snapshot = view.snapshot;
  const peer = snapshot.peer;
  $("settings-peer").hidden = !peer;
  if (peer) {
    $("peer-name").textContent = peer.name;
    $("peer-fingerprint").textContent = peer.fingerprint_prefix;
    $("peer-address").textContent = peer.address || "—";
  } else {
    resetUnpair();
  }
  renderUpdate($("settings-update"), view.update, version);
  $("about-version").textContent = version ? `v${version}` : "";
  $("about-device").textContent = snapshot.device_name || "—";
  $("about-fingerprint").textContent = snapshot.fingerprint_prefix || "";
}
