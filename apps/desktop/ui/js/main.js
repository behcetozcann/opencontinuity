// Açılış ve yönlendirme: eşleşme yoksa ya da cihaz eklenirken sihirbaz, yoksa ana ekran; Ayarlar üstte kayar.
import { isMac, setUi, start, store, subscribe } from "./api.js";
import { rememberUpdate } from "./components/update.js";
import { fillIcons } from "./icons.js";
import { bindHome, renderHome } from "./views/home.js";
import { bindOnboarding, renderOnboarding } from "./views/onboarding.js";
import { bindSettings, renderSettings } from "./views/settings.js";

const CELEBRATE_MS = 1800;

function render(state) {
  const view = state.view;
  // Motor ilk durumu bildirmeden çizilmez; eşleşmiş kullanıcı sihirbazı görüp kaybetmesin.
  if (!view || (!view.snapshot.device_name && !view.engine_error)) return;
  document.getElementById("app").hidden = false;
  rememberUpdate(view.update);

  const snapshot = view.snapshot;
  // Kod onaylandıktan sonra motor önce "eşleşti", sonra "eşleştirme bitti" ve
  // "eşler" der; arada hoş geldiniz ekranı görünmesin ve sonunda "Hazır" gösterilsin.
  // Gruba cihaz eklerken eş listesi zaten dolu; bitişi eşleşme sayacı söyler.
  const stage = snapshot.pairing?.stage;
  if ((stage === "confirm" || stage === "waiting_peer") && !state.ui.awaitingPeer) {
    state.ui.awaitingPeer = true;
    state.ui.pairedSeen = view.paired_count;
  }
  if (stage === "searching" || view.pairing_error) state.ui.awaitingPeer = false;
  if (state.ui.awaitingPeer && view.paired_count > state.ui.pairedSeen && !snapshot.pairing) {
    state.ui.awaitingPeer = false;
    state.ui.celebrating = true;
    setTimeout(() => setUi({ celebrating: false }), CELEBRATE_MS);
  }

  const onboarding = snapshot.peers.length === 0 || Boolean(snapshot.pairing) || state.ui.celebrating || state.ui.awaitingPeer;
  document.getElementById("onboarding").hidden = !onboarding;
  document.getElementById("home").hidden = onboarding;
  if (onboarding) renderOnboarding(state);
  else renderHome(state);
  renderSettings(state);
}

document.documentElement.dataset.platform = isMac ? "mac" : "windows";
fillIcons();
bindOnboarding();
bindHome();
bindSettings();
subscribe(render);
// "3 dk önce" gibi göreli zamanlar tazelensin.
setInterval(() => render(store), 30000);
start();
