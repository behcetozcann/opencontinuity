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
    return { state: "connected", title: "Bağlı", detail: "Kopyaladığınız metin ve görseller diğer bilgisayara anında geçer." };
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

const LABELS = {
  sent: { text: "Metin gönderildi", image: "Görsel gönderildi" },
  received: { text: "Metin alındı", image: "Görsel alındı" },
  too_large: { text: "Metin gönderilmedi", image: "Görsel gönderilmedi" },
};
const BUSY_LABELS = { sent: "Görsel gönderiliyor…", received: "Görsel alınıyor…" };
const LIMITS = { text: "4 MB", image: "32 MB" };

/** Tür simgesi + köşede yön rozeti; renk yönü (gönderildi/alındı/gönderilmedi) gösterir. */
function transferItem(transfer, { label, sub, when, className }) {
  const item = document.createElement("li");
  item.className = className;
  const badge = { sent: icons.sent(10), received: icons.received(10), too_large: icons.alert(10) }[transfer.direction];
  item.innerHTML = `<span class="transfer-icon">${transfer.content === "image" ? icons.image() : icons.text()}<span class="badge">${badge}</span></span><span class="transfer-what"><b></b><small></small></span><span class="transfer-size"></span><span class="transfer-when"></span>`;
  item.querySelector(".transfer-what b").textContent = label;
  item.querySelector(".transfer-what small").textContent = sub ?? "";
  item.querySelector(".transfer-size").textContent = formatBytes(transfer.bytes);
  item.querySelector(".transfer-when").textContent = when;
  return item;
}

function renderTransfers(transfers, inFlight) {
  const list = $("transfers");
  const newest = transfers[0]?.at_ms ?? null;
  // İlk çizimde değil, yalnız gerçekten yeni bir aktarım gelince canlandırılır.
  const fresh = lastTransferAt !== null && newest !== null && newest !== lastTransferAt;
  if (fresh && transfers[0].direction !== "too_large") {
    flashTransfer($("home-devices"), transfers[0].direction, transfers[0].content);
  }
  lastTransferAt = newest ?? lastTransferAt ?? 0;
  const items = transfers.map((transfer, index) => {
    const tooLarge = transfer.direction === "too_large";
    return transferItem(transfer, {
      label: LABELS[transfer.direction][transfer.content],
      sub: tooLarge ? `${LIMITS[transfer.content]} sınırını aşıyor` : null,
      when: formatWhen(transfer.at_ms),
      className: `${transfer.direction}${fresh && index === 0 ? " fresh" : ""}`,
    });
  });
  if (inFlight) {
    items.unshift(transferItem(inFlight, { label: BUSY_LABELS[inFlight.direction], when: "şimdi", className: `${inFlight.direction} busy` }));
  }
  list.replaceChildren(...items);
  $("transfers-empty").hidden = items.length > 0;
}

export function bindHome() {
  $("sharing").addEventListener("change", (event) => invoke("set_paused", { paused: !event.target.checked }));
  $("open-settings").addEventListener("click", () => setUi({ settingsOpen: true }));
}

export function renderHome({ view, version }) {
  const snapshot = view.snapshot;
  const current = status(view);
  $("home").dataset.state = current.state;
  renderDevices($("home-devices"), {
    selfName: snapshot.device_name,
    peerName: snapshot.peer?.name,
    state: current.state,
    busy: view.in_flight?.direction,
  });
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
  renderTransfers(view.recent_transfers, view.in_flight);
  $("home-version").textContent = version ? `v${version}` : "";
}
