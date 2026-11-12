// Captures Curvyo's Trace UI as Tracyo's look reference (specs/0001-trace-and-save/ux-notes.md §12).
// Needs Curvyo's dev server (`npm run dev --prefix frontend` in ~/curvyo, port 1420), Chromium and
// playwright-core. Usage:
//   PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core CHROMIUM=/usr/bin/chromium \
//   LUCIDE=~/curvyo/frontend/node_modules/lucide-react/dist/esm/icons SETS=px node capture.cjs
// Writes curvyo-*.png (Curvyo as it is, 1280 x 800, panel open) and px-*.png (the bar as Tracyo
// draws it, patched in the DOM by tracyo-bar.cjs: no tool rail, the bar centred in a 1280 x 800
// window and at most 1256 px wide, Open first, the Mask group in the Parts row of Advanced,
// "Save as…"; the viewport is 1360 x 824 so the window sits right of and below Curvyo's rulers).
// Device scale factor 1. The trace worker is wrapped so "running" (42 %) and "failed" can be staged;
// everything drawn is Curvyo's. SETS (default "curvyo,px") picks the sets.
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const HERE = __dirname;
const tracyoBar = require('./tracyo-bar.cjs');
const URL = process.env.CURVYO_URL || 'http://localhost:1420/';

function staging() {
  window.__traceMode = 'normal';
  const Real = window.Worker;
  window.Worker = class extends Real {
    constructor(url, opts) { super(url, opts); this.__trace = String(url).includes('traceWorker'); }
    postMessage(msg, transfer) {
      if (this.__trace && msg && msg.type === 'trace' && window.__traceMode !== 'normal') {
        const data = window.__traceMode === 'fail'
          ? { id: msg.id, kind: 'error', message: 'RangeError: could not allocate memory' }
          : { id: msg.id, kind: 'progress', percent: 42 };
        setTimeout(() => this.dispatchEvent(new MessageEvent('message', { data })), 80);
        return;
      }
      return super.postMessage(msg, transfer);
    }
  };
}

async function run(browser, set, icons) {
  const tracyo = set === 'px';
  const ctx = await browser.newContext({ viewport: tracyo ? { width: 1360, height: 824 } : { width: 1280, height: 800 }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  await page.addInitScript(staging);
  await page.goto(URL, { waitUntil: 'networkidle' });
  await page.waitForTimeout(2500);
  if (tracyo) await page.getByRole('button', { name: 'Hide panel' }).click();
  const [chooser] = await Promise.all([page.waitForEvent('filechooser'), page.getByRole('button', { name: 'Import', exact: true }).click()]);
  await chooser.setFiles(path.join(HERE, 'input-logo.png'));
  await page.waitForTimeout(2000);
  await page.keyboard.press('i');
  await page.waitForTimeout(2500);
  if (tracyo) await page.addStyleTag({ content: tracyoBar.STYLE });
  const bar = page.locator('[role=toolbar][aria-label=Trace]');
  // Tracyo's layout is applied before the pointer or focus is placed, so hover and focus stay.
  const relabel = () => tracyo && tracyoBar.sync(page, icons);
  const clip = async (below = 0) => { if (tracyo) { await relabel(); return tracyoBar.barClip(page, below); } const b = await bar.boundingBox(); return { x: b.x - 12, y: b.y - 12, width: Math.ceil(b.width) + 24, height: Math.ceil(b.height) + 24 + below }; };
  const shot = async (name, c) => { await page.waitForTimeout(50); await page.screenshot({ path: path.join(HERE, `${set}-${name}.png`), clip: c ?? (await clip()) }); };
  const rest = async () => { await relabel(); await page.mouse.move(700, 760); };
  const view = async () => { if (tracyo) { await relabel(); return tracyoBar.VIEW; } const b = await bar.boundingBox(); return { x: b.x - 12, y: b.y - 12, width: 830, height: 620 }; };
  await relabel();
  await page.getByRole('button', { name: 'Method' }).click();
  await page.waitForTimeout(300);
  await shot('09-method-menu', { ...(await clip()), height: 260 });
  await page.getByRole('menuitemradio', { name: /Brightness/ }).click();
  await page.waitForTimeout(2000);
  await rest();
  await shot('00-bar-closed');
  await shot('02-preview', await view());
  await relabel();
  await page.getByRole('button', { name: 'Preset' }).click();
  await page.waitForTimeout(300);
  await shot('08-preset-menu', { ...(await clip()), height: 260 });
  await page.keyboard.press('Escape');
  await relabel();
  await page.getByRole('spinbutton', { name: 'Smooth' }).hover();
  await page.waitForTimeout(800);
  await shot('12-value-field-tooltip', await clip(70));
  await rest();
  await page.getByRole('button', { name: 'More trace options' }).click();
  await page.waitForTimeout(1200);
  await rest();
  await shot('03-advanced-open');
  await page.getByRole('radio', { name: 'Multicolour' }).click();
  await page.waitForTimeout(3000);
  await rest();
  await page.waitForTimeout(400);
  await shot('04-multicolour-palette');
  const swatch = page.locator('[data-trace-palette-swatch]').nth(1);
  await relabel();
  await swatch.hover();
  await page.waitForTimeout(900);
  await shot('10-palette-swatch-hover', await clip(60));
  await swatch.click({ modifiers: ['Control'] });
  await page.waitForTimeout(2500);
  await rest();
  await page.waitForTimeout(400);
  await shot('11-palette-excluded');
  await page.evaluate(() => { window.__traceMode = 'slow'; });
  await page.getByRole('spinbutton', { name: 'Smooth' }).focus();
  await page.keyboard.press('ArrowUp');
  await page.evaluate(() => document.activeElement.blur());
  await page.waitForTimeout(900);
  await shot('05-running-42');
  await page.getByRole('button', { name: 'Stop (Esc)' }).click();
  await rest();
  await page.waitForTimeout(500);
  await shot('07-stopped');
  await page.evaluate(() => { window.__traceMode = 'fail'; });
  await page.getByRole('button', { name: 'Retry' }).click();
  await page.waitForTimeout(1200);
  await page.evaluate(() => document.activeElement.blur());
  await rest();
  await page.waitForTimeout(300);
  await shot('06-failed');
  await page.evaluate(() => { window.__traceMode = 'normal'; });
  await page.getByRole('button', { name: 'Retry' }).click();
  await page.waitForTimeout(2000);
  await rest();
  await relabel();
  await page.getByRole('button', { name: 'Fewer trace options' }).focus();
  await page.keyboard.press('Tab');
  await page.waitForTimeout(300);
  await shot('13-trace-button-focus');
  await page.evaluate(() => document.activeElement.blur());
  await relabel();
  await page.getByRole('button', { name: 'Preset' }).hover();
  await page.waitForTimeout(200);
  await shot('14-preset-hover');
  await relabel();
  await page.getByRole('button', { name: 'About Trace' }).hover();
  await page.waitForTimeout(800);
  await shot('15-tooltip', await clip(40));
  // Empty: a click on the empty canvas clears the selection; Tracyo's empty bar has no sentence.
  await page.mouse.click(tracyo ? 1300 : 900, 740);
  await page.waitForTimeout(800);
  if (tracyo) {
    await page.evaluate(() => {
      const t = document.querySelector('[role=toolbar][aria-label=Trace]');
      for (const s of t.querySelectorAll('span')) if (/Click an image/.test(s.textContent)) s.remove();
      for (const b of t.querySelectorAll('button')) if (/Import image/.test(b.textContent)) b.textContent = 'Open image…';
    });
  }
  await shot('01-empty');
  await ctx.close();
}

(async () => {
  const icons = await tracyoBar.icons();
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM || '/usr/bin/chromium', headless: true });
  for (const set of (process.env.SETS || 'curvyo,px').split(',')) await run(browser, set, icons);
  await browser.close();
})().catch((e) => { console.error(e); process.exit(1); });
