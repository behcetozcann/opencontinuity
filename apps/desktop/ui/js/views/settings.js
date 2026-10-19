// Ayarlar paneli: Genel, Gelen dosyalar, Cihazlar, Güncellemeler, Sorun giderme, Hakkında.
import { invoke, notify, run, setUi, store } from "../api.js";
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

async function loadNotifications() {
  $("notifications").checked = await invoke("notifications_enabled");
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

// Kaldırma onayı açık olan cihazın parmak izi öneki.
let confirmingRemoval = null;

function peerCard(peer, groupSize) {
  const card = document.createElement("div");
  card.className = "card peer-card";
  card.innerHTML = `
    <dl class="facts">
      <dt>Ad</dt><dd class="peer-name"><span class="dot"></span><b></b></dd>
      <dt>Durum</dt><dd class="peer-state"></dd>
      <dt>Parmak izi</dt><dd class="mono peer-fingerprint"></dd>
      <dt>Adres</dt><dd class="mono peer-address"></dd>
    </dl>
    <div class="unpair-ask"><button class="btn danger-outline">Bu cihazı kaldır</button></div>
    <div class="unpair-confirm" hidden>
      <p class="small"></p>
      <div class="row">
        <button class="btn secondary unpair-no">Vazgeç</button>
        <button class="btn danger unpair-yes">Kaldır</button>
      </div>
    </div>`;
  card.dataset.connected = String(peer.connected);
  card.querySelector(".peer-name b").textContent = peer.name;
  card.querySelector(".peer-state").textContent = peer.connected ? "Bağlı" : "Bağlantı bekleniyor";
  card.querySelector(".peer-fingerprint").textContent = peer.fingerprint_prefix;
  card.querySelector(".peer-address").textContent = peer.address || "—";
  card.querySelector(".unpair-confirm p").textContent =
    groupSize > 1
      ? `${peer.name} ile paylaşım durur, diğer cihazlarla sürer. ${peer.name} öteki cihazlarla eşli kalır; tamamen çıkarmak için onlarda da kaldırın.`
      : "Pano paylaşımı durur. Diğer bilgisayarda da eşleşmeyi kaldırmanız gerekir.";
  const confirming = confirmingRemoval === peer.fingerprint_prefix;
  card.querySelector(".unpair-ask").hidden = confirming;
  card.querySelector(".unpair-confirm").hidden = !confirming;
  card.querySelector(".unpair-ask button").addEventListener("click", () => {
    confirmingRemoval = peer.fingerprint_prefix;
    notify();
  });
  card.querySelector(".unpair-no").addEventListener("click", () => {
    confirmingRemoval = null;
    notify();
  });
  card.querySelector(".unpair-yes").addEventListener("click", () => {
    confirmingRemoval = null;
    // Son cihaz kaldırılınca kurulum ekranı açılır; ayarlar onun üstünde kalmasın.
    if (groupSize === 1) setUi({ settingsOpen: false });
    invoke("unpair", { fingerprintPrefix: peer.fingerprint_prefix });
  });
  return card;
}

/** Kartlar yalnız içerik değişince yeniden kurulur; yoksa açık onay ve odak kaybolurdu. */
function renderPeers(peers) {
  const cards = $("peer-cards");
  const key = JSON.stringify([peers, confirmingRemoval]);
  if (cards.dataset.key === key) return;
  cards.dataset.key = key;
  if (!peers.some((peer) => peer.fingerprint_prefix === confirmingRemoval)) confirmingRemoval = null;
  cards.replaceChildren(...peers.map((peer) => peerCard(peer, peers.length)));
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
  $("notifications").addEventListener("change", async (event) => {
    await run("set_notifications", { enabled: event.target.checked });
    loadNotifications();
  });
  $("open-files").addEventListener("click", async () => {
    const error = await run("open_files_dir");
    if (error) showError($("open-files-error"), `Klasör açılamadı: ${error}`);
    else $("open-files-error").hidden = true;
  });
  $("add-device").addEventListener("click", () => {
    setUi({ settingsOpen: false });
    invoke("start_pairing", { address: null });
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
    loadNotifications();
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
  $("settings-peers").hidden = snapshot.peers.length === 0;
  renderPeers(snapshot.peers);
  $("add-device").disabled = Boolean(view.engine_error);
  renderUpdate($("settings-update"), view.update, version);
  $("about-version").textContent = version ? `v${version}` : "";
  $("about-device").textContent = snapshot.device_name || "—";
  $("about-fingerprint").textContent = snapshot.fingerprint_prefix || "";
}
