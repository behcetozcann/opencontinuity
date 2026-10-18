// İki cihaz ve aralarındaki bağlantı çizgisi. Durum: connected | waiting |
// paused | error | idle — renk, ışık, çizgi biçimi ve animasyon birlikte değişir.
import { peerIcon, selfIcon } from "../icons.js";

export function renderDevices(element, { selfName, peerName, state, busy }) {
  if (!element.dataset.built) {
    element.innerHTML = `
      <div class="halo"></div>
      <div class="device self"><span class="device-icon">${selfIcon(30)}</span><span class="device-name"></span></div>
      <div class="link"><span class="line"></span><span class="flow"></span></div>
      <div class="device peer"><span class="device-icon">${peerIcon(30)}</span><span class="device-name"></span></div>`;
    element.dataset.built = "1";
  }
  element.dataset.state = state;
  // Süren görsel aktarımı: çizgide yönlü, sürekli akış.
  if (busy) element.dataset.busy = busy;
  else delete element.dataset.busy;
  element.querySelector(".self .device-name").textContent = selfName || "Bu bilgisayar";
  element.querySelector(".peer .device-name").textContent = peerName || "Diğer bilgisayar";
}

/** Aktarım anında çizgi boyunca yönlü bir paket (metin: nokta, görsel: kare) geçirir; hedef cihaz hafifçe parlar. */
export function flashTransfer(element, direction, content) {
  const link = element.querySelector(".link");
  if (!link) return;
  const packet = document.createElement("span");
  packet.className = `packet ${direction} ${content}`;
  link.append(packet);
  packet.addEventListener("animationend", () => packet.remove());
  const target = element.querySelector(direction === "sent" ? ".peer .device-icon" : ".self .device-icon");
  target.classList.remove("receive");
  void target.offsetWidth;
  target.classList.add("receive");
}
