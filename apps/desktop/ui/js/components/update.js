// Tek güncelleme bileşeni: ana ekrandaki kartta ve Ayarlar'da aynı davranır.
// Denetle → (bulundu) aynı düğme "Güncelle ve yeniden başlat" → ilerleme → yeniden başlat.
import { invoke } from "../api.js";

/** Bulunan ama kurulmamış sürüm; kurulum hatasından sonra da hatırlanır. */
let pendingVersion = null;

export function rememberUpdate(update) {
  if (update.state === "available") pendingVersion = update.version;
  if (update.state === "up_to_date") pendingVersion = null;
}

/** Ana ekranda kart gösterilmeli mi? */
export function updateNeedsAttention(update) {
  return ["available", "downloading", "installing"].includes(update.state) || (update.state === "failed" && pendingVersion);
}

function describe(update, currentVersion) {
  const busy = update.state === "downloading" || update.state === "installing";
  switch (update.state) {
    case "checking":
      return { text: "Güncellemeler denetleniyor…", button: "Denetleniyor…", disabled: true };
    case "available":
      return { title: `Yeni sürüm: v${update.version}`, text: "Kurulum birkaç saniye sürer, uygulama yeniden açılır.", button: "Güncelle ve yeniden başlat", primary: true, action: "install" };
    case "downloading":
      return { title: `v${update.version} indiriliyor`, text: update.percent == null ? "İndiriliyor…" : `%${update.percent}`, progress: update.percent ?? 0, busy };
    case "installing":
      return { title: `v${update.version} kuruluyor`, text: "Uygulama birazdan yeniden açılacak.", progress: 100, busy };
    case "failed":
      return pendingVersion
        ? { title: `v${pendingVersion} kurulamadı`, text: update.message, button: "Tekrar dene", primary: true, action: "install", tone: "error" }
        : { text: update.message, button: "Tekrar denetle", action: "check", tone: "error" };
    case "up_to_date": {
      const time = new Date(update.checked_at_ms).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });
      return { text: `v${currentVersion} · Güncel (${time}'de denetlendi)`, button: "Güncellemeleri denetle", action: "check" };
    }
    default:
      return { text: `v${currentVersion}`, button: "Güncellemeleri denetle", action: "check" };
  }
}

export function renderUpdate(element, update, currentVersion) {
  if (!element.dataset.built) {
    element.innerHTML = `
      <div class="update-head"><b class="update-title"></b><p class="update-text"></p></div>
      <div class="progress"><span></span></div>
      <button class="btn update-button"></button>`;
    element.querySelector(".update-button").addEventListener("click", () => {
      const button = element.querySelector(".update-button");
      button.disabled = true;
      invoke(button.dataset.action === "install" ? "install_update" : "check_update").catch(() => {});
    });
    element.dataset.built = "1";
  }
  const model = describe(update, currentVersion);
  element.dataset.tone = model.tone || "";
  const title = element.querySelector(".update-title");
  title.hidden = !model.title;
  title.textContent = model.title || "";
  element.querySelector(".update-text").textContent = model.text;
  const progress = element.querySelector(".progress");
  progress.hidden = model.progress == null;
  progress.firstElementChild.style.width = `${model.progress ?? 0}%`;
  const button = element.querySelector(".update-button");
  button.hidden = !model.button;
  button.textContent = model.button || "";
  button.className = `btn update-button ${model.primary ? "primary" : "secondary"}`;
  button.dataset.action = model.action || "";
  button.disabled = Boolean(model.disabled);
}
