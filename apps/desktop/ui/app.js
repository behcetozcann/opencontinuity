const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

/** Elle IP alanının kendiliğinden açılması için arama süresi. */
const MANUAL_AFTER_MS = 20000;

const $ = (id) => document.getElementById(id);
let view = null;
let searchingSince = null;
let manualTimer = null;

function showTab(name) {
  document.querySelectorAll(".tabs button").forEach((button) => {
    button.classList.toggle("active", button.dataset.tab === name);
  });
  document.querySelectorAll(".tab").forEach((section) => {
    section.hidden = section.id !== `tab-${name}`;
  });
}

function formatTransfer(transfer) {
  if (!transfer) return "Henüz aktarım yok.";
  const direction = transfer.direction === "sent" ? "Gönderildi" : "Alındı";
  const time = new Date(transfer.at_ms).toLocaleTimeString("tr-TR");
  return `${direction} · ${transfer.bytes.toLocaleString("tr-TR")} bayt · ${time}`;
}

function renderStatus(snapshot) {
  const peer = snapshot.peer;
  let title, detail, look;
  if (snapshot.pairing) {
    [title, detail, look] = ["Eşleştiriliyor", "Cihazlar sekmesinden devam edin.", "waiting"];
  } else if (!peer) {
    [title, detail, look] = ["Eşleşmiş cihaz yok", "Pano paylaşımı için bir cihazla eşleştirin.", "waiting"];
  } else if (snapshot.paused) {
    [title, detail, look] = ["Duraklatıldı", `${peer.name} ile paylaşım duraklatıldı.`, "paused"];
  } else if (snapshot.connected_ip) {
    [title, detail, look] = ["Bağlı", `${peer.name} · ${snapshot.connected_ip}`, "connected"];
  } else {
    [title, detail, look] = ["Bağlantı bekleniyor", `${peer.name} aynı ağda ve açık olmalı.`, "waiting"];
  }
  $("status-title").textContent = title;
  $("status-detail").textContent = detail;
  $("status-dot").className = `dot ${look}`;
  $("status-pause").hidden = !peer;
  $("status-pause").textContent = snapshot.paused ? "Devam et" : "Duraklat";
  $("status-pair").hidden = Boolean(peer || snapshot.pairing);
  $("last-transfer").textContent = formatTransfer(view.last_transfer);
}

function renderPeer(snapshot) {
  const peer = snapshot.peer;
  $("peer-card").hidden = !peer || Boolean(snapshot.pairing);
  if (!peer) return;
  $("peer-name").textContent = peer.name;
  $("peer-fingerprint").textContent = peer.fingerprint_prefix;
  $("peer-address").textContent = peer.address || "—";
}

function renderPairing(snapshot) {
  const pairing = snapshot.pairing;
  $("pairing").hidden = Boolean(snapshot.peer) && !pairing;
  const stage = pairing ? pairing.stage : "idle";
  for (const name of ["idle", "searching", "confirm", "waiting"]) {
    const element = $(`pair-${name}`);
    element.hidden = !(stage === name || (name === "waiting" && stage === "waiting_peer"));
  }
  $("pair-error").hidden = !view.pairing_error;
  $("pair-error").textContent = view.pairing_error ? `Eşleştirilemedi: ${view.pairing_error}` : "";

  if (stage === "searching") {
    $("own-ip").textContent = pairing.local_ip || "—";
    if (searchingSince === null) {
      searchingSince = Date.now();
      manualTimer = setTimeout(() => ($("manual").hidden = false), MANUAL_AFTER_MS);
    }
  } else {
    searchingSince = null;
    clearTimeout(manualTimer);
    $("manual").hidden = true;
  }
  if (stage === "confirm") {
    $("confirm-peer").textContent = pairing.peer;
    $("code").textContent = pairing.code;
    $("code-yes").disabled = false;
    $("code-no").disabled = false;
  }
  if (stage === "waiting_peer") {
    $("waiting-peer").textContent = pairing.peer;
  }
}

function render(next) {
  const wasPaired = view && view.snapshot.peer;
  view = next;
  const snapshot = view.snapshot;
  $("engine-error").hidden = !view.engine_error;
  $("engine-error").textContent = view.engine_error ? `OpenContinuity durdu: ${view.engine_error}` : "";
  renderStatus(snapshot);
  renderPeer(snapshot);
  renderPairing(snapshot);
  $("paused").checked = snapshot.paused;
  $("paused").disabled = !snapshot.peer;
  $("own-name").textContent = snapshot.device_name || "—";
  $("own-fingerprint").textContent = snapshot.fingerprint_prefix || "—";
  if (!snapshot.peer) resetUnpairConfirm();
  // Eşleşme yeni tamamlandıysa durum ekranına dön.
  if (!wasPaired && snapshot.peer && !snapshot.pairing) showTab("status");
}

function resetUnpairConfirm() {
  $("unpair-ask").hidden = false;
  $("unpair-confirm").hidden = true;
}

async function loadAutostart() {
  try {
    $("autostart").checked = await invoke("autostart_enabled");
    $("autostart-error").hidden = true;
  } catch (error) {
    $("autostart-error").hidden = false;
    $("autostart-error").textContent = `Başlangıç ayarı okunamadı: ${error}`;
  }
}

function bind() {
  document.querySelectorAll(".tabs button").forEach((button) => {
    button.addEventListener("click", () => showTab(button.dataset.tab));
  });
  const startPairing = () => {
    showTab("devices");
    invoke("start_pairing", { address: null });
  };
  $("status-pair").addEventListener("click", startPairing);
  $("pair-start").addEventListener("click", startPairing);
  $("pair-cancel").addEventListener("click", () => invoke("cancel_pairing"));
  $("waiting-cancel").addEventListener("click", () => invoke("cancel_pairing"));
  $("manual-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const address = $("manual-address").value.trim();
    if (!address) return;
    // Önce aramayı bitir; motor komutları sırayla işler.
    await invoke("cancel_pairing");
    await invoke("start_pairing", { address });
  });
  const answer = (accepted) => {
    $("code-yes").disabled = true;
    $("code-no").disabled = true;
    invoke("confirm_code", { accepted });
  };
  $("code-yes").addEventListener("click", () => answer(true));
  $("code-no").addEventListener("click", () => answer(false));
  $("status-pause").addEventListener("click", () => invoke("set_paused", { paused: !view.snapshot.paused }));
  $("paused").addEventListener("change", (event) => invoke("set_paused", { paused: event.target.checked }));
  $("unpair").addEventListener("click", () => {
    $("unpair-ask").hidden = true;
    $("unpair-confirm").hidden = false;
  });
  $("unpair-no").addEventListener("click", resetUnpairConfirm);
  $("unpair-yes").addEventListener("click", () => {
    resetUnpairConfirm();
    invoke("unpair");
  });
  $("autostart").addEventListener("change", async (event) => {
    try {
      await invoke("set_autostart", { enabled: event.target.checked });
    } catch (error) {
      $("autostart-error").hidden = false;
      $("autostart-error").textContent = `Başlangıç ayarı değiştirilemedi: ${error}`;
    }
    loadAutostart();
  });
}

async function main() {
  bind();
  await listen("view", (event) => render(event.payload));
  render(await invoke("get_view"));
  if (!view.snapshot.peer) showTab("devices");
  $("version").textContent = `v${await invoke("app_version")}`;
  loadAutostart();
}

main();
