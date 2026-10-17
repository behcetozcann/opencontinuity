// Satır içi SVG ikonlar (dış kaynak yok). `currentColor` ile temaya uyar.
import { isMac } from "./api.js";

const stroke = (body, size = 24) =>
  `<svg viewBox="0 0 24 24" width="${size}" height="${size}" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

export const icons = {
  laptop: (size) => stroke('<rect x="4" y="5" width="16" height="11" rx="1.5"/><path d="M2 19h20"/>', size),
  desktop: (size) => stroke('<rect x="3" y="4" width="18" height="12" rx="1.5"/><path d="M9 20h6M12 16v4"/>', size),
  gear: (size = 16) =>
    stroke('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>', size),
  back: (size = 18) => stroke('<path d="M15 18l-6-6 6-6"/>', size),
  check: (size = 30) => stroke('<path class="draw" pathLength="1" d="M5 12.5l4.5 4.5L19 7.5"/>', size),
  sent: (size = 16) => stroke('<path d="M7 17L17 7M9 7h8v8"/>', size),
  received: (size = 16) => stroke('<path d="M17 7L7 17M15 17H7V9"/>', size),
  warning: (size = 18) => stroke('<path d="M12 3l9.5 17h-19z"/><path d="M12 10v4M12 17.5v.01"/>', size),
  error: (size = 18) => stroke('<circle cx="12" cy="12" r="9"/><path d="M12 8v5M12 16.5v.01"/>', size),
  update: (size = 18) => stroke('<path d="M12 4v11M7 10l5 5 5-5M5 20h14"/>', size),
};

/** Bu bilgisayarın ve karşı bilgisayarın ikonu (ürün Mac ↔ Windows için). */
export const selfIcon = (size) => (isMac ? icons.laptop(size) : icons.desktop(size));
export const peerIcon = (size) => (isMac ? icons.desktop(size) : icons.laptop(size));

/** `<i data-icon="...">` yer tutucularını doldurur. */
export function fillIcons(root = document) {
  root.querySelectorAll("i[data-icon]").forEach((slot) => {
    const name = slot.dataset.icon;
    slot.innerHTML = name === "self" ? selfIcon(20) : name === "peer" ? peerIcon(20) : icons[name]();
  });
}
