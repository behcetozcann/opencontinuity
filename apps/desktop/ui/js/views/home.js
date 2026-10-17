// Ana ekran: iki cihaz, durum, bağlamsal kart, paylaşım anahtarı, son aktarımlar.
import { invoke, setUi } from "../api.js";
import { flashTransfer, renderDevices } from "../components/devices.js";
import { pickNotice, renderNotice } from "../components/notice.js";
import { icons } from "../icons.js";

const $ = (id) => document.getElementById(id);
let lastTransferAt = null;

function status(view) {
  const snapshot = view.snapshot;
  const peer = snapshot.peer?.name || "Diğer bilgisayar";
  if (view.engine_error) {
    return { state: "error", title: "Çalışmıyor", detail: "Aşağıdaki karttan sorunu giderebilirsiniz." };
  }
  if (snapshot.paused) {
    return { state: "paused", title: "Duraklatıldı", detail: "Kopyaladıklarınız gönderilmiyor, gelenler panoya yazılmıyor." };
  }
  if (snapshot.connected_ip) {
    return { state: "connected", title: "Bağlı", detail: "Kopyaladığınız metin diğer bilgisayara anında geçer." };
  }
  return { state: "waiting", title: "Bağlantı bekleniyor", detail: `${peer} açık ve aynı ağda olmalı.` };
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toLocaleString("tr-TR", { maximumFractionDigits: 1 })} KB`;
  return `${(bytes / 1024 / 1024).toLocaleString("tr-TR", { maximumFractionDigits: 1 })} MB`;
}

function formatWhen(atMs) {
  const seconds = Math.round((Date.now() - atMs) / 1000);
  if (seconds < 60) return "az önce";
  if (seconds < 3600) return `${Math.floor(seconds / 60)} dk önce`;
  return new Date(atMs).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });
}

function renderTransfers(transfers) {
  const list = $("transfers");
  const newest = transfers[0]?.at_ms ?? null;
  // İlk çizimde değil, yalnız gerçekten yeni bir aktarım gelince canlandırılır.
  const fresh = lastTransferAt !== null && newest !== null && newest !== lastTransferAt;
  if (fresh) flashTransfer($("home-devices"), transfers[0].direction);
  lastTransferAt = newest ?? lastTransferAt ?? 0;
  list.replaceChildren(
    ...transfers.map((transfer, index) => {
      const item = document.createElement("li");
      const sent = transfer.direction === "sent";
      item.className = `${sent ? "sent" : "received"}${fresh && index === 0 ? " fresh" : ""}`;
      item.innerHTML = `<span class="transfer-icon">${sent ? icons.sent() : icons.received()}</span><span class="transfer-what"></span><span class="transfer-size"></span><span class="transfer-when"></span>`;
      item.querySelector(".transfer-what").textContent = sent ? "Gönderildi" : "Alındı";
      item.querySelector(".transfer-size").textContent = formatBytes(transfer.bytes);
      item.querySelector(".transfer-when").textContent = formatWhen(transfer.at_ms);
      return item;
    }),
  );
  $("transfers-empty").hidden = transfers.length > 0;
}

export function bindHome() {
  $("sharing").addEventListener("change", (event) => invoke("set_paused", { paused: !event.target.checked }));
  $("open-settings").addEventListener("click", () => setUi({ settingsOpen: true }));
}

export function renderHome({ view, version }) {
  const snapshot = view.snapshot;
  const current = status(view);
  $("home").dataset.state = current.state;
  renderDevices($("home-devices"), { selfName: snapshot.device_name, peerName: snapshot.peer?.name, state: current.state });
  const title = $("home-title");
  if (title.textContent !== current.title) {
    title.textContent = current.title;
    title.classList.remove("swap");
    void title.offsetWidth;
    title.classList.add("swap");
  }
  $("home-detail").textContent = current.detail;
  renderNotice($("home-notice"), pickNotice(view, { includePairing: false }));
  const sharing = $("sharing");
  sharing.checked = !snapshot.paused;
  sharing.disabled = Boolean(view.engine_error);
  $("sharing-sub").textContent = snapshot.paused ? "Kapalı — hiçbir şey gönderilmiyor" : "Açık";
  renderTransfers(view.recent_transfers);
  $("home-version").textContent = version ? `v${version}` : "";
}
