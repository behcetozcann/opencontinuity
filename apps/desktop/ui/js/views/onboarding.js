// Kurulum sihirbazı: Hoş geldiniz → Aranıyor → Kodu doğrula → Onay bekleniyor → Hazır.
// Adım motorun eşleştirme aşamasından türetilir; "Hazır" yalnız arayüzde 1,8 sn gösterilir.
import { invoke, run, setUi } from "../api.js";
import { renderDevices } from "../components/devices.js";
import { pickNotice, renderNotice } from "../components/notice.js";

const $ = (id) => document.getElementById(id);
const STEPS = ["welcome", "searching", "confirm", "waiting", "done"];

function currentStep(view, ui) {
  if (ui.celebrating) return "done";
  if (ui.awaitingPeer && !view.snapshot.pairing) return "waiting";
  switch (view.snapshot.pairing?.stage) {
    case "searching":
      return "searching";
    case "confirm":
      return "confirm";
    case "waiting_peer":
      return "waiting";
    default:
      return "welcome";
  }
}

export function bindOnboarding() {
  $("ob-start").addEventListener("click", (event) => {
    event.currentTarget.disabled = true;
    invoke("start_pairing", { address: null });
  });
  // Vazgeçince motor hata bildirmez; bekleme adımında takılı kalınmasın.
  const cancel = () => {
    setUi({ awaitingPeer: false });
    invoke("cancel_pairing");
  };
  $("ob-cancel").addEventListener("click", cancel);
  $("ob-waiting-cancel").addEventListener("click", cancel);
  $("ob-manual").addEventListener("submit", async (event) => {
    event.preventDefault();
    const address = $("ob-address").value.trim();
    if (!address) return;
    // Önce aramayı bitir; motor komutları sırayla işler.
    await run("cancel_pairing");
    await run("start_pairing", { address });
  });
  const answer = (accepted) => {
    $("ob-yes").disabled = true;
    $("ob-no").disabled = true;
    invoke("confirm_code", { accepted });
  };
  $("ob-yes").addEventListener("click", () => answer(true));
  $("ob-no").addEventListener("click", () => answer(false));
}

// "Kodlar aynı"ya çift tıklamanın ikincisi, aynı yere gelen Vazgeç'e düşüp
// eşleştirmeyi iptal ediyordu (Mac'te onaydan 0,2 sn sonra iptal).
function holdWaitingCancel() {
  const cancel = $("ob-waiting-cancel");
  cancel.disabled = true;
  setTimeout(() => (cancel.disabled = false), 800);
}

export function renderOnboarding({ view, ui }) {
  const snapshot = view.snapshot;
  const step = currentStep(view, ui);
  const screen = $("onboarding");
  if (screen.dataset.step !== step) {
    screen.dataset.step = step;
    STEPS.forEach((name) => ($(`ob-${name}`).hidden = name !== step));
    $("ob-start").disabled = false;
    $("ob-yes").disabled = false;
    $("ob-no").disabled = false;
    if (step === "waiting") holdWaitingCancel();
  }
  const pairing = snapshot.pairing || {};
  renderDevices($("ob-devices"), { selfName: snapshot.device_name, peerName: null, state: "idle" });
  $("ob-self-name").textContent = snapshot.device_name;
  $("ob-self-ip").textContent = pairing.local_ip || "";
  $("ob-confirm-self").textContent = snapshot.device_name;
  $("ob-confirm-peer").textContent = pairing.peer || "";
  const code = $("ob-code");
  if (code.dataset.code !== (pairing.code || "")) {
    code.dataset.code = pairing.code || "";
    code.replaceChildren(
      ...[...(pairing.code || "")].map((char, index) => {
        const digit = document.createElement("span");
        digit.textContent = char;
        digit.style.animationDelay = `${index * 45}ms`;
        if (char === " ") digit.className = "gap";
        return digit;
      }),
    );
  }
  $("ob-waiting-peer").textContent = pairing.peer || "diğer bilgisayar";
  // Gruba cihaz eklerken "Vazgeç" ana ekrana döner; yazılar yeni cihazı anlatır.
  const adding = snapshot.peers.length > 0;
  $("ob-searching-title").textContent = adding ? "Yeni cihaz aranıyor" : "Diğer bilgisayar aranıyor";
  $("ob-done-lead").textContent = adding
    ? "Yeni cihaz gruba katıldı; diğer cihazlar onu kendiliğinden tanır."
    : "Artık bir bilgisayarda kopyaladığınız metni diğerinde yapıştırabilirsiniz.";
  renderNotice($("ob-notice"), step === "welcome" ? pickNotice(view, { includePairing: true }) : null);
  $("ob-start").textContent = view.pairing_error ? "Tekrar dene" : "Eşleştirmeye başla";
  $("ob-start").disabled ||= Boolean(view.engine_error);
}
