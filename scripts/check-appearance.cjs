// Run against `npm run dev`. UI_ARTWORK_PATH optionally supplies official art.
const path = require("node:path");
const fs = require("node:fs");
const assert = require("node:assert/strict");
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const entry = require("./ui-fixture.cjs");
const out = path.resolve(__dirname, "../docs/design/reference-inspired");
fs.mkdirSync(out, { recursive: true });

async function textContrast(page) {
  return page.evaluate(() => {
    const rgba = (value) => {
      const parts = value.match(/[\d.]+/g)?.map(Number) || [0, 0, 0];
      return [...parts.slice(0, 3), parts[3] ?? 1];
    };
    const over = (fg, bg) =>
      fg.slice(0, 3).map((v, i) => v * fg[3] + bg[i] * (1 - fg[3]));
    const luminance = (rgb) =>
      rgb.reduce((sum, v, i) => {
        v /= 255;
        return (
          sum +
          (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4) *
            [0.2126, 0.7152, 0.0722][i]
        );
      }, 0);
    const failures = [];
    let count = 0,
      minimum = Infinity;
    for (const el of document.querySelectorAll("body *")) {
      if (
        !(el instanceof HTMLElement) ||
        !el.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })
      )
        continue;
      const label = ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName)
        ? el.value
        : [...el.childNodes]
            .filter((n) => n.nodeType === Node.TEXT_NODE)
            .map((n) => n.textContent)
            .join("")
            .trim();
      if (!label) continue;
      const rect = el.getBoundingClientRect();
      if (
        rect.bottom <= 0 ||
        rect.top >= innerHeight ||
        rect.width === 0 ||
        rect.height === 0
      )
        continue;
      const chain = [];
      for (let parent = el; parent; parent = parent.parentElement)
        chain.unshift(parent);
      if (
        chain.some((parent) => Number(getComputedStyle(parent).opacity) === 0)
      )
        continue;
      let bg = [255, 255, 255];
      // The translucent settings surface sits over arbitrary artwork. White is
      // the worst case for its light text, regardless of the current frame.
      const settingsIndex = chain.findIndex((parent) =>
        parent.classList.contains("settings-page"),
      );
      const surfaces = settingsIndex < 0 ? chain : chain.slice(settingsIndex);
      for (const parent of surfaces)
        bg = over(rgba(getComputedStyle(parent).backgroundColor), bg);
      const style = getComputedStyle(el);
      const fg = over(rgba(style.color), bg);
      const l1 = luminance(fg),
        l2 = luminance(bg);
      const ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
      const large =
        parseFloat(style.fontSize) >= 24 ||
        (parseFloat(style.fontSize) >= 18.66 &&
          Number(style.fontWeight) >= 700);
      const required = large ? 3 : 4.5;
      count++;
      minimum = Math.min(minimum, ratio);
      if (ratio < required)
        failures.push({
          label: label.slice(0, 100),
          className: el.className,
          foreground: style.color,
          background: bg,
          ratio: +ratio.toFixed(2),
          required,
        });
    }
    return { count, minimum: +minimum.toFixed(2), failures };
  });
}

(async () => {
  const browser = await chromium.launch({
    headless: true,
    ...(process.env.CHROMIUM_PATH
      ? { executablePath: process.env.CHROMIUM_PATH }
      : {}),
  });
  try {
    const page = await browser.newPage({
      viewport: { width: 1280, height: 720 },
      reducedMotion: "reduce",
    });
    const errors = [],
      report = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.addInitScript(() => {
      window.__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
      };
    });
    await page.route("https://**/*", (route) => route.abort());
    await page.route("**/src/main.jsx*", (route) =>
      route.fulfill({ contentType: "text/javascript", body: entry }),
    );
    let artwork = "official";
    await page.route("**/appearance-artwork", (route) => {
      if (artwork === "missing") return route.abort();
      if (artwork === "official" && process.env.UI_ARTWORK_PATH) {
        return route.fulfill({
          contentType: "image/png",
          body: fs.readFileSync(process.env.UI_ARTWORK_PATH),
        });
      }
      return route.fulfill({
        contentType: "image/svg+xml",
        body: `<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="720"><path fill="${artwork === "black" ? "#000" : "#fff"}" d="M0 0h1280v720H0z"/></svg>`,
      });
    });
    const go = async (state) => {
      await page.goto("http://127.0.0.1:1420/?state=" + state);
      await page.locator(".home-page__action-area").waitFor();
      await page.waitForTimeout(150);
      await page.locator(".main-layout__background img").evaluate((img) => {
        img.src = "/appearance-artwork";
      });
      if (artwork !== "missing")
        await page.waitForFunction(() => {
          const img = document.querySelector(".main-layout__background img");
          return img.complete && img.naturalWidth > 0;
        });
    };
    const check = async (name, screenshot = false) => {
      await page.waitForTimeout(80);
      const result = await textContrast(page);
      assert(result.count > 5, `${name}: no visible text checked`);
      assert.deepEqual(result.failures, [], `${name}: insufficient contrast`);
      assert(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
        `${name}: horizontal overflow`,
      );
      const geometry = await page.evaluate(() => {
        const home = document.querySelector(".home-page");
        if (!home || home.hidden) return [];
        const failures = [];
        const news = home
          .querySelector(".home-page__news")
          .getBoundingClientRect();
        const action = home
          .querySelector(".home-page__action-area")
          .getBoundingClientRect();
        const banner = home.querySelector(".banner-carousel__viewport");
        if (
          banner &&
          getComputedStyle(banner.closest(".banner-carousel")).display !==
            "none"
        ) {
          const b = banner.getBoundingClientRect();
          if (b.top < news.top - 1 || b.bottom > news.bottom + 1)
            failures.push("banner escapes news row");
        }
        if (
          Math.min(news.right, action.right) -
            Math.max(news.left, action.left) >
            1 &&
          Math.min(news.bottom, action.bottom) -
            Math.max(news.top, action.top) >
            1
        )
          failures.push("news overlaps primary action");
        if (
          [...home.querySelectorAll('[role="tab"]')].some(
            (tab) => !tab.textContent.trim(),
          )
        )
          failures.push("empty news tab");
        return failures;
      });
      assert.deepEqual(geometry, [], `${name}: home layout`);
      report.push({ name, ...result });
      if (screenshot)
        await page.screenshot({ path: path.join(out, name + ".png") });
    };
    const settings = () =>
      page.getByRole("button", { name: "Настройки", exact: true }).click();
    const nav = (name) =>
      page
        .locator(".settings-nav")
        .getByRole("button", { name, exact: true })
        .click();
    for (const state of [
      "ready",
      "first",
      "missing",
      "update",
      "paused",
      "disk-full",
      "offline",
      "en",
    ]) {
      await go(state);
      await check(
        "home-" + state,
        ["ready", "first", "paused"].includes(state),
      );
    }
    await go("ready");
    await page.locator(".action-button").hover();
    await check("play-hover");
    await page.locator(".action-button").focus();
    await page.keyboard.press("Tab");
    assert(
      await page.evaluate(
        () => getComputedStyle(document.activeElement).outlineStyle !== "none",
      ),
    );
    await page
      .getByRole("button", { name: "Действия с игрой", exact: true })
      .click();
    await check("game-actions-open", true);
    await page.keyboard.press("Escape");
    await page.getByRole("link", { name: "Discord", exact: true }).hover();
    await check("sidebar-hover");
    await go("first");
    await page.locator(".home-page__install-details > summary").click();
    await check("install-details", true);
    await page.keyboard.press("Escape");
    await settings();
    for (const section of [
      "Общие",
      "Игра и файлы",
      "Запуск",
      "Моды",
      "Диагностика",
    ]) {
      await nav(section);
      await check(
        "settings-" +
          {
            Общие: "general",
            "Игра и файлы": "files",
            Запуск: "launch",
            Моды: "mods",
            Диагностика: "diagnostics",
          }[section],
        true,
      );
    }
    await nav("Запуск");
    await page.getByText("Дополнительные параметры", { exact: true }).click();
    await page.getByRole("switch", { name: "Gamescope", exact: true }).click();
    await page
      .getByLabel("Режим окна", { exact: true })
      .scrollIntoViewIfNeeded();
    await check("advanced-inputs");
    assert(
      await page
        .locator("select")
        .evaluateAll((selects) =>
          selects.every((el) => getComputedStyle(el).colorScheme === "dark"),
        ),
    );
    await nav("Общие");
    await page.getByRole("button", { name: "Сохранить", exact: true }).hover();
    await check("save-hover");
    await nav("К игре");
    await page.getByRole("dialog").waitFor();
    await check("confirm-dialog", true);
    await page
      .getByRole("button", { name: "Не сохранять", exact: true })
      .hover();
    await check("confirm-danger-hover");
    await page.keyboard.press("Escape");
    for (const mode of ["white", "black", "missing"]) {
      artwork = mode;
      await go("ready");
      await check("background-" + mode, true);
      if (mode === "missing")
        assert.equal(
          await page
            .locator(".home-page__hero")
            .evaluate((el) => getComputedStyle(el).opacity),
          "1",
        );
    }
    artwork = "official";
    for (const [width, height] of [
      [1024, 600],
      [1280, 720],
    ]) {
      await page.setViewportSize({ width, height });
      await go("first");
      await check("install-" + width, true);
      const box = await page.locator(".action-button").boundingBox();
      assert(box.y >= 0 && box.y + box.height <= height);
      await settings();
      await nav("Моды");
      await check("mods-" + width);
    }
    assert.deepEqual(errors, []);
    fs.writeFileSync(
      path.join(out, "contrast.json"),
      JSON.stringify(
        {
          artwork: process.env.UI_ARTWORK_PATH
            ? "Official launcher artwork, supplied locally"
            : "Synthetic white and black backgrounds",
          scope:
            "Visible text and control values on their composited CSS surfaces; Chromium with mocked Tauri IPC. Does not measure text embedded in artwork or native OS dialogs.",
          checks: report,
        },
        null,
        2,
      ) + "\n",
    );
    console.log(
      `PASS: ${report.length} appearance checks; text contrast, hover, inputs, dialogs, light/dark/missing artwork, window sizes; no page errors.`,
    );
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
