// Captures Curvyo's Trace UI as Tracyo's look reference (specs/0001-trace-and-save/ux-notes.md §12).
// Needs Curvyo's dev server (`npm run dev --prefix frontend` in ~/curvyo, port 1420), Chromium and
// playwright-core. Usage:
//   PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core CHROMIUM=/usr/bin/chromium node capture.cjs
// Writes curvyo-*.png (Curvyo as it is, 1280 x 800, panel open) and px-*.png (the same bar with the
// Mask group and Parts hidden and Tracyo's labels, panel hidden, 1360 x 800: the bar then has
// exactly the 1196 px Tracyo's bar has in a 1280 x 800 window). Device scale factor 1. The trace
// worker is wrapped so "running" (42 %) and "failed" can be staged; everything drawn is Curvyo's.
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const HERE = __dirname;
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

async function run(browser, set) {
  const tracyo = set === 'px';
  const ctx = await browser.newContext({ viewport: { width: tracyo ? 1360 : 1280, height: 800 }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  await page.addInitScript(staging);
  await page.goto(URL, { waitUntil: 'networkidle' });
  await page.waitForTimeout(2500);
  if (tracyo) {
    await page.addStyleTag({ content: '[data-trace-mask],[data-trace-parts]{display:none!important}' });
    await page.getByRole('button', { name: 'Hide panel' }).click();
  }
  const [chooser] = await Promise.all([page.waitForEvent('filechooser'), page.getByRole('button', { name: 'Import', exact: true }).click()]);
  await chooser.setFiles(path.join(HERE, 'input-logo.png'));
  await page.waitForTimeout(2000);
  await page.keyboard.press('i');
  await page.waitForTimeout(2500);
  const bar = page.locator('[role=toolbar][aria-label=Trace]');
  const relabel = () => tracyo && page.evaluate(() => { const b = document.querySelector('[data-trace-slot=none]'); if (b) b.textContent = 'Save as…'; });
  const clip = async (below = 0) => { const b = await bar.boundingBox(); return { x: b.x - 12, y: b.y - 12, width: Math.ceil(b.width) + 24, height: Math.ceil(b.height) + 24 + below }; };
  const shot = async (name, c) => { await relabel(); await page.screenshot({ path: path.join(HERE, `${set}-${name}.png`), clip: c ?? (await clip()) }); };
  const rest = () => page.mouse.move(700, 760);
  const view = async () => { const b = await bar.boundingBox(); return { x: b.x - 12, y: b.y - 12, width: tracyo ? 1220 : 830, height: 620 }; };
  await page.getByRole('button', { name: 'Method' }).click();
  await page.waitForTimeout(300);
  await shot('09-method-menu', { ...(await clip()), height: 260 });
  await page.getByRole('menuitemradio', { name: /Brightness/ }).click();
  await page.waitForTimeout(2000);
  await rest();
  await shot('00-bar-closed');
  await shot('02-preview', await view());
  await page.getByRole('button', { name: 'Preset' }).click();
  await page.waitForTimeout(300);
  await shot('08-preset-menu', { ...(await clip()), height: 260 });
  await page.keyboard.press('Escape');
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
  await page.getByRole('button', { name: 'Fewer trace options' }).focus();
  await page.keyboard.press('Tab');
  await page.waitForTimeout(300);
  await shot('13-trace-button-focus');
  await page.evaluate(() => document.activeElement.blur());
  await page.getByRole('button', { name: 'Preset' }).hover();
  await page.waitForTimeout(200);
  await shot('14-preset-hover');
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
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM || '/usr/bin/chromium', headless: true });
  for (const set of ['curvyo', 'px']) await run(browser, set);
  await browser.close();
})().catch((e) => { console.error(e); process.exit(1); });
