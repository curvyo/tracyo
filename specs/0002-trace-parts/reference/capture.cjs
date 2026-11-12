// Captures Curvyo's Mask group, Parts row and mask canvas look as Tracyo's reference
// (specs/0002-trace-parts/ux-notes.md section 12). Needs Curvyo's dev server (`npm run dev --prefix
// frontend` in ~/curvyo, port 1420), Chromium and playwright-core. Usage:
//   PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core CHROMIUM=/usr/bin/chromium \
//   LUCIDE=~/curvyo/frontend/node_modules/lucide-react/dist/esm/icons node capture.cjs
// Writes curvyo-*.png (Curvyo as it is, 1280 x 800, panel open) and px-*.png (the bar as Tracyo
// draws it: panel hidden, 1360 x 800, so the bar has the 1164 px cap of a 1248 x 800 Tracyo window;
// Pick mask paths replaced by Rectangle and Subtract (lucide square, square-minus) in Curvyo's own
// icon-button look; "Save as…" for Trace). Device scale factor 1. The picture is 0001's
// input-logo.png; the mask paths are drawn with Curvyo's real Rectangle tool (the curvyo-* set also
// with its Ellipse tool for E1, which px-* draws as the rectangle R4) and picked with Pick mask paths.
// SETS=px re-captures only the px-* pictures.
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const HERE = __dirname;
const URL = process.env.CURVYO_URL || 'http://localhost:1420/';
const PICTURE = path.join(HERE, '..', '..', '0001-trace-and-save', 'reference', 'input-logo.png');
const LUCIDE = process.env.LUCIDE || path.join(process.env.HOME, 'curvyo/frontend/node_modules/lucide-react/dist/esm/icons');

/** A lucide icon as an SVG string, 16 px, stroke 1.5, as lucide-react renders it in the bar. */
async function icon(name) {
  const { __iconData } = await import(path.join(LUCIDE, `${name}.mjs`));
  const body = __iconData.node.map(([tag, attrs]) => `<${tag} ${Object.entries(attrs).filter(([k]) => k !== 'key').map(([k, v]) => `${k}="${v}"`).join(' ')}/>`).join('');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" class="lucide lucide-${name}">${body}</svg>`;
}

/** The picture's box on screen: the first and last rows with a 300 px run of #FFFFFF left of x = 940. */
async function pictureBox(page) {
  const png = (await page.screenshot()).toString('base64');
  return page.evaluate(async (data) => {
    const img = new Image();
    img.src = `data:image/png;base64,${data}`;
    await img.decode();
    const c = document.createElement('canvas');
    c.width = img.width; c.height = img.height;
    const g = c.getContext('2d');
    g.drawImage(img, 0, 0);
    const px = g.getImageData(0, 0, c.width, c.height).data;
    let box = null;
    for (let y = 120; y < c.height - 30; y++) {
      let run = 0;
      for (let x = 150; x < Math.min(c.width, 1300); x++) {
        const i = (y * c.width + x) * 4;
        if (px[i] === 255 && px[i + 1] === 255 && px[i + 2] === 255) {
          run++;
          if (run === 300) {
            const x0 = x - 299;
            box = box ? { ...box, y1: y } : { x0, y0: y, y1: y };
          }
        } else run = 0;
      }
    }
    return box;
  }, png);
}

async function run(browser, set, icons) {
  const tracyo = set === 'px';
  const ctx = await browser.newContext({ viewport: { width: tracyo ? 1360 : 1280, height: 800 }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  await page.goto(URL, { waitUntil: 'networkidle' });
  await page.waitForTimeout(2500);
  if (tracyo) await page.getByRole('button', { name: 'Hide panel' }).click();
  const [chooser] = await Promise.all([page.waitForEvent('filechooser'), page.getByRole('button', { name: 'Import', exact: true }).click()]);
  await chooser.setFiles(PICTURE);
  await page.waitForTimeout(2000);
  await page.keyboard.press('Escape');
  await page.mouse.move(5, 790);
  await page.waitForTimeout(300);
  const box = await pictureBox(page);
  if (!box) throw new Error('picture not found');
  const at = (x, y) => [box.x0 + x, box.y0 + y];
  const drag = async (tool, a, b, hold = false, selected = null) => {
    await page.keyboard.press(tool);
    await page.mouse.move(...at(...a));
    await page.mouse.down();
    await page.mouse.move(...at((a[0] + b[0]) / 2, (a[1] + b[1]) / 2), { steps: 5 });
    await page.mouse.move(...at(...b), { steps: 5 });
    if (hold) return;
    await page.mouse.up();
    await page.waitForTimeout(200);
    if (selected) await selected();
    await page.keyboard.press('Escape');
    await page.waitForTimeout(100);
  };
  // The mask paths, in picture pixels: R1 round the LOGO block, R2 round the green bar, E1 round the
  // disc (px-*: the rectangle R4 on the same box), R3 below the picture (a mask outside the image).
  const R1 = [[24, 25], [324, 235]], R2 = [[344, 315], [619, 405]], E1 = [[330, 30], [610, 310]], R3 = [[100, 450], [300, 520]];
  const crop = (x0, y0, x1, y1) => { const [a, b] = at(x0, y0); const [c, d] = at(x1, y1); return { x: a, y: b, width: c - a, height: d - b }; };
  // The canvas look of a selected, hovered and drawn object in Curvyo's Select, Pen and Node tools.
  const canvasShot = (name, c) => page.screenshot({ path: path.join(HERE, `curvyo-${name}.png`), clip: c });
  const selectedShot = (name, c) => tracyo ? null : async () => { await page.mouse.move(...at(700, 470)); await page.waitForTimeout(300); await canvasShot(name, c); };
  await drag('r', ...R1, false, selectedShot('20-select-rectangle', crop(-30, -35, 364, 275)));
  await drag('r', ...R2);
  await drag(tracyo ? 'r' : 'e', ...E1, false, selectedShot('22-select-ellipse', crop(290, -10, 650, 350)));
  await drag('r', ...R3);
  if (!tracyo) {
    await page.mouse.move(...at(344, 360));
    await page.waitForTimeout(400);
    await canvasShot('21-hover-rectangle', crop(304, 275, 659, 445));
    await page.keyboard.down('Control');
    await page.mouse.move(...at(720, 300));
    await page.mouse.move(...at(722, 302));
    await page.waitForTimeout(300);
    await canvasShot('27-remove-badge', crop(660, 250, 780, 360));
    await page.keyboard.up('Control');
    await drag('r', [40, 260], [200, 400], true);
    await page.waitForTimeout(200);
    await canvasShot('23-rectangle-drawing', crop(0, 220, 240, 440));
    await page.mouse.up();
    await page.keyboard.press('Control+z');
    await page.keyboard.press('Escape');
    await page.keyboard.press('b');
    for (const p of [[60, 270], [230, 290], [200, 400]]) { await page.mouse.click(...at(...p)); await page.waitForTimeout(150); }
    await page.mouse.move(...at(90, 380), { steps: 4 });
    await page.waitForTimeout(200);
    await canvasShot('24-pen-drawing', crop(20, 230, 270, 440));
    await page.mouse.move(...at(61, 271), { steps: 4 });
    await page.waitForTimeout(200);
    await canvasShot('25-pen-close-target', crop(20, 230, 270, 440));
    await page.mouse.click(...at(60, 270));
    await page.waitForTimeout(200);
    await page.keyboard.press('Escape');
    await page.mouse.click(...at(145, 280));
    await page.keyboard.press('n');
    await page.waitForTimeout(200);
    await page.mouse.click(...at(230, 290));
    await page.mouse.move(...at(700, 470));
    await page.waitForTimeout(300);
    await canvasShot('26-node-polygon', crop(20, 230, 270, 440));
    // Undo the pen polygon so it does not show in the trace pictures.
    for (let i = 0; i < 3; i++) await page.keyboard.press('Escape');
    await page.keyboard.press('Control+z');
    await page.waitForTimeout(300);
  }
  // Enter the Trace tool with the picture alone, Method Brightness.
  await page.mouse.click(...at(120, 330));
  await page.waitForTimeout(300);
  await page.keyboard.press('i');
  await page.waitForTimeout(2500);
  await page.getByRole('button', { name: 'Method' }).click();
  await page.waitForTimeout(300);
  await page.getByRole('menuitemradio', { name: /Brightness/ }).click();
  await page.waitForTimeout(2000);
  const bar = page.locator('[role=toolbar][aria-label=Trace]');
  const REST = 'flex size-7 shrink-0 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]';
  const PRESSED = 'flex size-7 shrink-0 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]';
  let pressed = -1;
  const tracyoBar = () => tracyo && page.evaluate(({ icons, REST, PRESSED, pressed }) => {
    const pick = document.querySelector('[data-trace-mask-pick]');
    if (!pick) return;
    if (!pick.dataset.tracyo) {
      pick.dataset.tracyo = 'rectangle';
      pick.setAttribute('aria-label', 'Rectangle');
      const b = pick.cloneNode(true);
      b.removeAttribute('data-trace-mask-pick');
      b.dataset.tracyo = 'subtract';
      b.setAttribute('aria-label', 'Subtract');
      pick.after(b);
    }
    const buttons = [...document.querySelectorAll('[data-tracyo]')];
    buttons.forEach((b, i) => {
      b.innerHTML = icons[i];
      b.className = i === pressed ? PRESSED : REST;
    });
    const finish = document.querySelector('[data-trace-slot=none]');
    if (finish) finish.textContent = 'Save as…';
  }, { icons, REST, PRESSED, pressed });
  const vw = tracyo ? 1360 : 1280;
  const clip = async (below = 0, right = 0) => { const b = await bar.boundingBox(); return { x: b.x - 12, y: b.y - 12, width: Math.min(vw - b.x + 12, Math.ceil(b.width) + 24 + right), height: Math.ceil(b.height) + 24 + below }; };
  // A tooltip opens only after the pointer has left every trigger for a while (Radix skips the delay otherwise).
  const hover = async (locator) => { await rest(); await page.waitForTimeout(700); await locator.hover(); await page.waitForTimeout(900); };
  const shot = async (name, c) => { await tracyoBar(); await page.waitForTimeout(50); await page.screenshot({ path: path.join(HERE, `${set}-${name}.png`), clip: c ?? (await clip()) }); };
  const view = async () => { const b = await bar.boundingBox(); return { x: b.x - 12, y: b.y - 12, width: tracyo ? 1220 : 830, height: 620 }; };
  const rest = () => page.mouse.move(tracyo ? 1300 : 900, 760);
  const pick = page.locator('[data-trace-mask-pick]');
  const pickToggle = async () => { await pick.click(); await page.waitForTimeout(300); };
  await rest();
  await page.waitForTimeout(400);
  await shot('00-mask-group');
  // The region tool pressed (Tracyo) = Pick mask paths armed (Curvyo), the hover box on a mask path.
  await pickToggle();
  pressed = 0;
  await page.mouse.move(...at(174, 130));
  await page.waitForTimeout(400);
  await shot('01-pick-armed', await view());
  await page.mouse.click(...at(174, 130));
  await page.waitForTimeout(1200);
  await page.mouse.click(...at(480, 360));
  await page.waitForTimeout(1500);
  await pickToggle();
  pressed = -1;
  await rest();
  await page.waitForTimeout(1500);
  await shot('02-mask-inside', await view());
  await shot('03-mask-inside-bar');
  await page.getByRole('radio', { name: 'Outside' }).hover();
  await page.waitForTimeout(300);
  await shot('04-outside-hover');
  await page.getByRole('radio', { name: 'Outside' }).click();
  await rest();
  await page.waitForTimeout(2500);
  await shot('05-mask-outside', await view());
  await page.getByRole('radio', { name: 'Inside' }).click();
  await page.waitForTimeout(1500);
  await hover(page.getByRole('button', { name: 'Clear mask' }));
  await shot('06-clear-mask-tooltip', await clip(40));
  await page.getByRole('radio', { name: 'Inside' }).focus();
  await page.keyboard.press('Shift+Tab');
  await page.waitForTimeout(200);
  await rest();
  await shot('07-region-tool-focus');
  await page.evaluate(() => document.activeElement.blur());
  await page.getByRole('button', { name: 'More trace options' }).click();
  await page.waitForTimeout(1500);
  await rest();
  await page.waitForTimeout(300);
  await shot('08-parts-one');
  // Part 2: E1 Outside; part 3: no paths; part 2 active.
  await page.locator('[data-trace-add-part]').click();
  await page.waitForTimeout(800);
  await pickToggle();
  await page.mouse.click(...at(470, 170));
  await page.waitForTimeout(1500);
  await pickToggle();
  await page.getByRole('radio', { name: 'Outside' }).click();
  await page.waitForTimeout(1500);
  await page.locator('[data-trace-add-part]').click();
  await page.waitForTimeout(1500);
  await page.locator('[data-trace-part="2"]').click();
  await rest();
  await page.waitForTimeout(2500);
  await shot('09-parts-three');
  await shot('10-parts-preview', await view());
  await hover(page.locator('[data-trace-part="1"]'));
  await shot('11-chip-hover', await clip(40));
  await hover(page.locator('[data-trace-part="3"]'));
  await shot('12-muted-chip-hover', await clip(40));
  await hover(page.locator('[data-trace-add-part]'));
  await shot('13-add-part-tooltip', await clip(40));
  await page.locator('[data-trace-remove-part]').click();
  await page.mouse.move(...(await (async () => { const b = await page.locator('[data-trace-remove-part]').boundingBox(); return [b.x + 14, b.y + 14]; })()));
  await page.waitForTimeout(900);
  await shot('14-trash-armed', await clip(40));
  await page.keyboard.press('Escape');
  await rest();
  await page.waitForTimeout(700);
  await page.locator('[data-trace-part="2"]').focus();
  await page.waitForTimeout(200);
  await shot('15-chip-focus');
  // Part 3 picks R3, below the picture; parts 1 and 2 removed, so it is the only part:
  // "0 paths: the mask is outside the image."; then an empty part 2: "0 paths: no part has anything to trace."
  await page.locator('[data-trace-part="3"]').click();
  await page.waitForTimeout(800);
  await pickToggle();
  await page.mouse.click(...at(200, 485));
  await page.waitForTimeout(1500);
  await pickToggle();
  for (let i = 0; i < 2; i++) {
    await page.locator('[data-trace-part="1"]').click();
    await page.waitForTimeout(800);
    await page.locator('[data-trace-remove-part]').click();
    await page.waitForTimeout(600);
    await page.locator('[data-trace-remove-part]').click();
    await page.waitForTimeout(1500);
  }
  await page.waitForTimeout(1500);
  await hover(page.locator('[data-trace-slot=none]'));
  await shot('16-mask-outside-image', await clip(40, 280));
  await page.locator('[data-trace-add-part]').click();
  await page.waitForTimeout(2500);
  await hover(page.locator('[data-trace-slot=none]'));
  await shot('17-no-part-to-trace', await clip(40, 280));
  await ctx.close();
}

(async () => {
  const icons = await Promise.all(['square', 'square-minus'].map(icon));
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM || '/usr/bin/chromium', headless: true });
  for (const set of (process.env.SETS || 'curvyo,px').split(',')) await run(browser, set, icons);
  await browser.close();
})().catch((e) => { console.error(e); process.exit(1); });
