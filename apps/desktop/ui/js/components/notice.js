// Bağlamsal kart: ana ekranda ve sihirbazda en fazla bir tane, önceliğe göre.
// Her kart nedeni insan diliyle ve tek tıklık çözümle verir; ham hata "Ayrıntı"da.
import { invoke, isMac, setUi, store } from "../api.js";
import { icons } from "../icons.js";
import { renderUpdate, updateNeedsAttention } from "./update.js";

const LOCAL_NETWORK = /os error 65|No route to host/i;

function engineNotice(error) {
  const portBusy = /47800|kullanılıyor|in use/i.test(error);
  return {
    tone: "error",
    title: "OpenContinuity başlatılamadı",
    text: portBusy
      ? "Bağlantı portu başka bir program tarafından kullanılıyor. Açık kalmış başka bir OpenContinuity varsa kapatıp yeniden deneyin."
      : "Beklenmeyen bir hata oluştu. Yeniden deneyin; düzelmezse teşhis kaydını gönderin.",
    detail: error,
    actions: [
      { label: "Yeniden dene", primary: true, run: () => invoke("restart_engine") },
      { label: "Kaydı gönder", run: () => setUi({ settingsOpen: true, focusTroubleshoot: true }) },
    ],
  };
}

const localNetworkNotice = (detail) => ({
  tone: "warning",
  title: "Yerel ağ izni gerekli",
  text: "macOS bu uygulamanın aynı ağdaki bilgisayarlara erişimini engelliyor. Gizlilik ve Güvenlik → Yerel Ağ'da OpenContinuity'yi açın.",
  detail,
  actions: [{ label: "Ayarları aç", primary: true, run: () => invoke("open_local_network_settings") }],
});

function pairingNotice(error) {
  let text = "Eşleştirme tamamlanamadı. Tekrar deneyin.";
  if (/karşı cihaz reddetti/i.test(error)) text = "Diğer bilgisayarda kodlar onaylanmadı. Hiçbir şey kaydedilmedi.";
  else if (/eşleştirme reddedildi/i.test(error)) text = "Kodlar farklı dediniz; hiçbir şey kaydedilmedi. Tekrar deneyebilirsiniz.";
  else if (/yanıt vermedi|zaman aşımı|bağlantı kurulamadı/i.test(error)) text = "Diğer bilgisayara ulaşılamadı. Orada da OpenContinuity'nin açık ve aynı ağda olduğundan emin olun.";
  else if (/koptu/i.test(error)) text = "Eşleştirme sırasında bağlantı koptu. Tekrar deneyin.";
  return { tone: "error", title: "Eşleştirilemedi", text, detail: error, actions: [] };
}

/** Görünümden gösterilecek kartı seçer: motor hatası > yerel ağ izni > güncelleme. */
export function pickNotice(view, { includePairing }) {
  if (view.engine_error) return engineNotice(view.engine_error);
  const networkError = [...view.snapshot.peers.map((peer) => peer.link_error), view.pairing_error].find((error) => error && LOCAL_NETWORK.test(error));
  if (isMac && networkError) return localNetworkNotice(networkError);
  if (updateNeedsAttention(view.update)) return { update: true };
  if (includePairing && view.pairing_error) return pairingNotice(view.pairing_error);
  return null;
}

export function renderNotice(slot, notice) {
  const key = notice ? (notice.update ? "update" : notice.title) : "";
  if (slot.dataset.key !== key) {
    slot.dataset.key = key;
    slot.innerHTML = "";
    if (notice?.update) {
      slot.innerHTML = `<div class="notice info"><span class="notice-icon">${icons.update()}</span><div class="notice-body update-control"></div></div>`;
    } else if (notice) {
      slot.innerHTML = `
        <div class="notice ${notice.tone}">
          <span class="notice-icon">${notice.tone === "error" ? icons.error() : icons.warning()}</span>
          <div class="notice-body">
            <b></b><p></p>
            <div class="notice-actions"></div>
            <details class="notice-detail"><summary>Ayrıntı</summary><code></code></details>
          </div>
        </div>`;
      slot.querySelector("b").textContent = notice.title;
      const actions = slot.querySelector(".notice-actions");
      for (const action of notice.actions) {
        const button = document.createElement("button");
        button.className = `btn small ${action.primary ? "primary" : "secondary"}`;
        button.textContent = action.label;
        button.addEventListener("click", action.run);
        actions.append(button);
      }
      actions.hidden = notice.actions.length === 0;
    }
  }
  if (notice?.update) {
    renderUpdate(slot.querySelector(".update-control"), store.view.update, store.version);
  } else if (notice) {
    slot.querySelector("p").textContent = notice.text;
    slot.querySelector("code").textContent = notice.detail || "";
    slot.querySelector(".notice-detail").hidden = !notice.detail;
  }
}
