// Patches Curvyo's rendered Trace bar into Tracyo's layout, in the DOM only (no Curvyo file is
// touched). Used by this folder's capture.cjs and by specs/0002-trace-parts/reference/capture.cjs.
//
// Tracyo's window is the rectangle WINDOW of the Chromium viewport (1280 x 800 at (24, 24), right of
// and below Curvyo's rulers). Its bar floats 12 px below the window top, centred, at most
// 1280 - 24 = 1256 px wide (ux-notes 0001 section 3.1). The patch:
// - hides the tool rail and every other toolbar: Tracyo has no rail;
// - pins Curvyo's overlay row to the window's top band (x 36 .. 1292, y 36) and centres the bar in it
//   on whole pixels (left margin floor((1256 - width) / 2));
// - puts the Open button (a clone of About, lucide `import` 16 px, name "Open image") first in the bar;
// - moves the Mask group out of the first row into the Parts row of Advanced, its own last line:
//   Parts, chips, Add part, trash, a 1 x 20 px divider in a 25 px gap, then Rectangle (Curvyo's
//   Pick mask paths button, relabelled), Lasso and Subtract (inert clones of it), Inside | Outside,
//   Clear mask; Analyse stays at the far right of the line above;
// - names the finish button "Save as…".
const path = require('node:path');

const WINDOW = { x: 24, y: 24, width: 1280, height: 800 };
const CAP = WINDOW.width - 24;
const LUCIDE = process.env.LUCIDE || path.join(process.env.HOME, 'curvyo/frontend/node_modules/lucide-react/dist/esm/icons');

/** A lucide icon as an SVG string, 16 px, stroke 1.5, as lucide-react renders it in the bar. */
async function icon(name) {
  const { __iconData } = await import(path.join(LUCIDE, `${name}.mjs`));
  const body = __iconData.node.map(([tag, attrs]) => `<${tag} ${Object.entries(attrs).filter(([k]) => k !== 'key').map(([k, v]) => `${k}="${v}"`).join(' ')}/>`).join('');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" class="lucide lucide-${name}">${body}</svg>`;
}

/** The icons the patch draws: Open, then the three region buttons. */
async function icons() {
  const [open, rectangle, lasso, subtract] = await Promise.all(['import', 'square', 'lasso', 'square-minus'].map(icon));
  return { open, tools: [rectangle, lasso, subtract] };
}

const ICON_BUTTON = 'flex size-7 shrink-0 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]';
/** Curvyo's icon-button classes at rest and pressed (the pressed pipette's). */
const REST = `${ICON_BUTTON} text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]`;
const PRESSED = `${ICON_BUTTON} bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]`;

/** CSS added once per page, after the picture is imported (the Import card is a rail card). */
const STYLE = '[role=toolbar]:not([aria-label=Trace]){visibility:hidden!important}';

/**
 * Applies the patch. `pressed` lists the pressed region buttons by index (0 Rectangle, 1 Lasso,
 * 2 Subtract). Idempotent; call it before every screenshot, after the state is set and before a
 * pointer hover or a keyboard focus (it rebuilds nothing that holds hover or focus).
 */
function sync(page, icons, pressed = []) {
  return page.evaluate(({ icons, REST, PRESSED, CAP, WINDOW, pressed }) => {
    const rail = document.querySelector('[aria-label^="Select tool"]');
    if (rail) rail.closest('.z-30').style.visibility = 'hidden';
    const bar = document.querySelector('[role=toolbar][aria-label=Trace]');
    if (!bar) return;
    const overlay = bar.parentElement.parentElement;
    Object.assign(overlay.style, { position: 'fixed', left: `${WINDOW.x + 12}px`, top: `${WINDOW.y + 12}px`, right: 'auto', width: `${CAP}px`, justifyContent: 'flex-start' });
    // Centred on whole pixels: x = floor((cap - width) / 2) inside the 12 px margins.
    const centre = () => {
      const column = bar.parentElement;
      column.style.marginLeft = '0px';
      column.style.marginLeft = `${Math.floor((CAP - bar.getBoundingClientRect().width) / 2)}px`;
    };
    const finish = bar.querySelector('[data-trace-slot=none]');
    if (finish) finish.textContent = 'Save as…';
    const about = bar.querySelector('[aria-label="About Trace"]');
    if (about && !bar.querySelector('[data-tracyo-open]')) {
      const slot = about.parentElement.cloneNode(true);
      const open = slot.querySelector('button');
      open.dataset.tracyoOpen = '';
      open.setAttribute('aria-label', 'Open image');
      open.removeAttribute('data-state');
      open.innerHTML = icons.open;
      about.parentElement.before(slot);
    }
    const mask = document.querySelector('[data-trace-mask]');
    const advanced = bar.querySelector('[role=group][aria-label=Advanced]');
    if (!mask) return centre();
    if (!advanced) {
      mask.style.display = 'none';
      return centre();
    }
    const parts = advanced.querySelector('[data-trace-parts]');
    if (parts) parts.style.order = '99';
    if (!advanced.querySelector('[data-tracyo-break]')) {
      const line = document.createElement('div');
      line.dataset.tracyoBreak = '';
      Object.assign(line.style, { flexBasis: '100%', height: '0', order: '98' });
      advanced.append(line);
    }
    if (mask.parentElement !== advanced) {
      advanced.append(mask);
      // The 25 px gap of Curvyo's bar dividers (8 of it is the block's own gap), the 1 x 20 px
      // divider 13 px left of the group (SelectToolbar `Divider`).
      Object.assign(mask.style, { display: 'flex', order: '100', position: 'relative', marginLeft: '17px' });
      const divider = document.createElement('span');
      divider.setAttribute('aria-hidden', 'true');
      divider.className = 'pointer-events-none absolute top-2 -left-[13px] h-5 w-px';
      divider.style.background = 'color-mix(in srgb, var(--toolbar-icon) 25%, transparent)';
      mask.append(divider);
    }
    const pick = mask.querySelector('[data-trace-mask-pick]');
    if (!pick.dataset.tracyo) {
      pick.dataset.tracyo = 'rectangle';
      pick.setAttribute('aria-label', 'Rectangle');
      let last = pick;
      for (const name of ['Lasso', 'Subtract']) {
        const b = pick.cloneNode(true);
        b.removeAttribute('data-trace-mask-pick');
        b.dataset.tracyo = name.toLowerCase();
        b.setAttribute('aria-label', name);
        last.after(b);
        last = b;
      }
    }
    [...mask.querySelectorAll('[data-tracyo]')].forEach((b, i) => {
      b.innerHTML = icons.tools[i];
      b.className = pressed.includes(i) ? PRESSED : REST;
    });
    return centre();
  }, { icons, REST, PRESSED, CAP, WINDOW, pressed });
}

/** The clip of the bar with 12 px around it (and `below`, `right` px more). */
async function barClip(page, below = 0, right = 0) {
  const b = await page.locator('[role=toolbar][aria-label=Trace]').boundingBox();
  return { x: b.x - 12, y: b.y - 12, width: Math.ceil(b.width) + 24 + right, height: Math.ceil(b.height) + 24 + below };
}

/** The top 620 px of Tracyo's window: the bar and the picture under it. */
const VIEW = { x: WINDOW.x, y: WINDOW.y, width: WINDOW.width, height: 620 };

module.exports = { WINDOW, CAP, VIEW, STYLE, icons, sync, barClip };
