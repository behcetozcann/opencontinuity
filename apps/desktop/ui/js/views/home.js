// Ana ekran: bu cihaz ve grup, durum, bağlamsal kart, paylaşım anahtarı, son aktarımlar.
import { invoke, run, setUi } from "../api.js";
import { flashTransfer, renderDevices } from "../components/devices.js";
import { pickNotice, renderNotice } from "../components/notice.js";
import { icons } from "../icons.js";

const $ = (id) => document.getElementById(id);
let lastTransferAt = null;

function status(view) {
  const snapshot = view.snapshot;
  const peers = snapshot.peers;
  const connected = peers.filter((peer) => peer.connected).length;
  const group = peers.length > 1;
  if (view.engine_error) {
    return { state: "error", title: "Çalışmıyor", detail: "Aşağıdaki karttan sorunu giderebilirsiniz." };
  }
  if (snapshot.paused) {
    return { state: "paused", title: "Duraklatıldı", detail: "Kopyaladıklarınız gönderilmiyor, gelenler panoya yazılmıyor." };
  }
  if (connected > 0 && !group) {
    return { state: "connected", title: "Bağlı", detail: "Kopyaladığınız metin, görsel ve dosyalar diğer bilgisayara geçer." };
  }
  if (connected === peers.length) {
    return { state: "connected", title: "Bağlı", detail: `Kopyaladıklarınız ${connected} cihazın hepsine geçer.` };
  }
  if (connected > 0) {
    return { state: "connected", title: "Bağlı", detail: `${peers.length} cihazdan ${connected} tanesi bağlı; kopyaladıklarınız bağlı olanlara geçer.` };
  }
  const waitingFor = group ? "Diğer cihazlar" : peers[0]?.name || "Diğer bilgisayar";
  return { state: "waiting", title: "Bağlantı bekleniyor", detail: `${waitingFor} açık ve aynı ağda olmalı.` };
}

/** Grupta her cihaz için durum noktalı bir etiket; tek eşte gizli (çizim zaten anlatıyor). */
function renderPeerChips(peers) {
  const list = $("home-peers");
  list.hidden = peers.length < 2;
  if (list.hidden) return;
  list.replaceChildren(
    ...peers.map((peer) => {
      const chip = document.createElement("li");
      chip.dataset.connected = String(peer.connected);
      chip.title = peer.connected ? "Bağlı" : peer.link_error || "Bağlantı bekleniyor";
      chip.innerHTML = `<span class="dot"></span><span class="chip-name"></span>`;
      chip.querySelector(".chip-name").textContent = peer.name;
      return chip;
    }),
  );
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

const LIMITS = { text: "4 MB", image: "32 MB", files: "500 MB" };

function subject(transfer) {
  if (transfer.content === "text") return "Metin";
  if (transfer.content === "image") return "Görsel";
  return transfer.items > 1 ? `${transfer.items} öğe` : "Dosya";
}

function label(transfer, busy) {
  const what = subject(transfer);
  if (busy) return `${what} ${transfer.direction === "sent" ? "gönderiliyor…" : "alınıyor…"}`;
  return `${what} ${{ sent: "gönderildi", received: "alındı", too_large: "gönderilmedi" }[transfer.direction]}`;
}

const TYPE_ICONS = { text: () => icons.text(), image: () => icons.image(), files: () => icons.files() };

/** Tür simgesi + köşede yön rozeti; renk yönü (gönderildi/alındı/gönderilmedi) gösterir. */
function transferItem(transfer, { busy = false, fresh = false, group = false } = {}) {
  const item = document.createElement("li");
  item.className = `${transfer.direction}${busy ? " busy" : ""}${fresh ? " fresh" : ""}`;
  const badge = { sent: icons.sent(10), received: icons.received(10), too_large: icons.alert(10) }[transfer.direction];
  item.innerHTML = `<span class="transfer-icon">${TYPE_ICONS[transfer.content]()}<span class="badge">${badge}</span></span><span class="transfer-what"><b></b><small></small></span><span class="transfer-size"></span><span class="transfer-when"></span>`;
  item.querySelector(".transfer-what b").textContent = label(transfer, busy);
  const note = item.querySelector(".transfer-what small");
  if (transfer.direction === "too_large") note.textContent = `${LIMITS[transfer.content]} sınırını aşıyor`;
  else if (group && transfer.from) {
    note.textContent = transfer.from;
    note.className = "from";
  }
  item.querySelector(".transfer-size").textContent = formatBytes(transfer.bytes);
  const when = item.querySelector(".transfer-when");
  if (transfer.can_reveal) {
    // Gelen dosyalar: zaman yerine klasörü açan düğme; zaman ipucunda kalır.
    const reveal = document.createElement("button");
    reveal.className = "btn link reveal";
    reveal.textContent = "Göster";
    reveal.title = formatWhen(transfer.at_ms);
    reveal.addEventListener("click", async () => {
      const error = await run("reveal_transfer", { atMs: transfer.at_ms });
      if (error) reveal.textContent = "Klasör yok";
    });
    when.append(reveal);
  } else {
    when.textContent = busy ? "şimdi" : formatWhen(transfer.at_ms);
  }
  return item;
}

function renderTransfers(transfers, inFlight, group) {
  const list = $("transfers");
  const newest = transfers[0]?.at_ms ?? null;
  // İlk çizimde değil, yalnız gerçekten yeni bir aktarım gelince canlandırılır.
  const fresh = lastTransferAt !== null && newest !== null && newest !== lastTransferAt;
  if (fresh && transfers[0].direction !== "too_large") {
    flashTransfer($("home-devices"), transfers[0].direction, transfers[0].content);
  }
  lastTransferAt = newest ?? lastTransferAt ?? 0;
  const items = transfers.map((transfer, index) => transferItem(transfer, { fresh: fresh && index === 0, group }));
  if (inFlight) items.unshift(transferItem(inFlight, { busy: true, group }));
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
    peerName: snapshot.peers.length > 1 ? `${snapshot.peers.length} cihaz` : snapshot.peers[0]?.name,
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
  renderPeerChips(snapshot.peers);
  renderNotice($("home-notice"), pickNotice(view, { includePairing: false }));
  const sharing = $("sharing");
  sharing.checked = !snapshot.paused;
  sharing.disabled = Boolean(view.engine_error);
  $("sharing-sub").textContent = snapshot.paused ? "Kapalı — hiçbir şey gönderilmiyor" : "Açık";
  renderTransfers(view.recent_transfers, view.in_flight, snapshot.peers.length > 1);
  $("home-version").textContent = version ? `v${version}` : "";
}
