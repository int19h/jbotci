// Serve a debug web build and set JBOTCI_TEST_URL to its address.
// Install playwright-core, or set JBOTCI_PLAYWRIGHT_MODULE to its module path.
// Set JBOTCI_TEST_BROWSER and JBOTCI_TEST_EXECUTABLE for the browser under test.
// Set JBOTCI_TEST_SCREENSHOTS to save images of the stack before and after scrolling.
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { createRequire } from "node:module";
import { join } from "node:path";
import test from "node:test";

const baseUrl = process.env.JBOTCI_TEST_URL;

test("reference component cards share a bounded scroll area", { skip: !baseUrl }, async (t) => {
  const require = createRequire(import.meta.url);
  const drivers = require(process.env.JBOTCI_PLAYWRIGHT_MODULE || "playwright-core");
  const engine = process.env.JBOTCI_TEST_BROWSER || "chromium";
  const browser = await drivers[engine].launch({
    headless: true,
    ...(process.env.JBOTCI_TEST_EXECUTABLE ? { executablePath: process.env.JBOTCI_TEST_EXECUTABLE } : {}),
  });
  const screenshots = process.env.JBOTCI_TEST_SCREENSHOTS;
  if (screenshots) await mkdir(screenshots, { recursive: true });
  try {
    for (const fixture of [
      { word: "klaklama", height: 400, cards: ["klaklama", "klama", "klama"] },
      { word: "klabajra", height: 844, cards: ["klabajra", "klama", "bajra"] },
      { word: "klama", height: 400, cards: ["klama"] },
    ]) for (const role of ["reference", "referent"]) for (const touch of engine === "firefox" ? [false] : [false, true]) {
      const name = `${engine}-${fixture.word}-${role}-${touch ? "touch" : "mouse"}`;
      await t.test(name, async () => {
        const context = await browser.newContext({ viewport: { width: 390, height: fixture.height }, hasTouch: touch });
        const page = await context.newPage();
        const errors = [];
        page.on("pageerror", (error) => errors.push(error.message));
        try {
          const url = new URL("/gentufa", baseUrl);
          url.search = new URLSearchParams({ text: `mi ${fixture.word}`, dialect: "()", glosses: "true" });
          await page.goto(url.href);
          const label = page.locator(`.blocks-grid .reference-tooltip-trigger:has([data-ref-role="${role}"])`).first();
          await label.waitFor({ timeout: 120000 });
          await page.evaluate(() => document.fonts.ready);
          if (touch) await label.tap(); else await label.hover();
          const root = page.locator("#reference-tooltip-overlay");
          await page.waitForFunction(() => document.getElementById("reference-tooltip-overlay")?.hasAttribute("data-jbotci-position-ready"));
          const geometry = await root.evaluate((el) => ({
            rect: el.getBoundingClientRect().toJSON(),
            topbar: document.querySelector(".app-topbar").getBoundingClientRect().bottom,
            maxHeight: getComputedStyle(el).maxHeight,
          }));
          const source = await label.boundingBox();
          assert.ok(geometry.rect.top >= geometry.topbar && geometry.rect.bottom <= fixture.height);
          assert.ok(geometry.rect.left >= 0 && geometry.rect.right <= 390);
          assert.ok(geometry.rect.bottom <= source.y || geometry.rect.top >= source.y + source.height, "Stack covers its label");
          assert.notEqual(geometry.maxHeight, "none");
          assert.deepEqual(await root.locator(".tooltip-word").allTextContents(), fixture.cards);
          assert.deepEqual(await root.locator(".reference-row-target").allTextContents(), role === "reference" ? ["mi"] : []);
          if (screenshots) await page.screenshot({ path: join(screenshots, `${name}-top.png`) });
          if (!touch) await label.click();
          const before = await root.evaluate((el) => ({ height: el.clientHeight, content: el.scrollHeight }));
          if (before.content > before.height) {
            const rect = await root.boundingBox();
            await page.mouse.move(rect.x + rect.width / 2, rect.y + rect.height / 2);
            await page.mouse.wheel(0, before.content);
            await page.waitForFunction((el) => el.scrollTop + el.clientHeight >= el.scrollHeight - 1, await root.elementHandle());
          }
          const reached = await root.evaluate((el) => {
            const frame = el.getBoundingClientRect();
            const last = [...el.querySelectorAll(".reference-component-card")].at(-1) || el.querySelector(".reference-definition-card");
            const rect = last.getBoundingClientRect();
            const x = rect.left + rect.width / 2;
            const y = (Math.max(rect.top, frame.top) + Math.min(rect.bottom, frame.bottom)) / 2;
            const hit = document.elementFromPoint(x, y);
            return { bottom: rect.bottom, frameBottom: frame.bottom, intersects: rect.bottom > frame.top && rect.top < frame.bottom, painted: last === hit || last.contains(hit), rows: [...el.querySelectorAll(".reference-resolution-tooltip")].map((row) => { const r = row.getBoundingClientRect(); return r.top >= frame.top - 1 && r.bottom <= frame.bottom + 1; }) };
          });
          assert.ok(reached.bottom <= reached.frameBottom + 1 && reached.intersects && reached.painted, "Last component is unreachable");
          assert.ok(reached.rows.every(Boolean), "Reference row is clipped after scrolling");
          await page.keyboard.press("Escape");
          await root.waitFor({ state: "detached" });
          await page.waitForFunction((el) => document.activeElement === el, await label.elementHandle());
          assert.deepEqual(errors, []);
          if (screenshots) {
            if (touch) await label.tap(); else await label.click();
            await label.press("ArrowDown");
            await root.press("End");
            await page.waitForTimeout(250);
            await page.screenshot({ path: join(screenshots, `${name}-bottom.png`) });
          }
        } finally { await context.close(); }
      });
    }
  } finally { await browser.close(); }
});
