const path = require("node:path");
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const out = path.resolve(__dirname, "../.ui-screenshots");
fs.mkdirSync(out, { recursive: true });
const entry = require("./ui-fixture.cjs");

(async () => {
  const browser = await chromium.launch({
    headless: true,
    ...(process.env.CHROMIUM_PATH
      ? { executablePath: process.env.CHROMIUM_PATH }
      : {}),
  });
  const page = await browser.newPage({
    viewport: { width: 1280, height: 720 },
  });
  page.setDefaultTimeout(10000);
  const errors = [];
  page.on("pageerror", (e) => {
    errors.push(e.message);
    console.log("PAGE ERROR:", e.message);
  });
  page.on("console", (m) => {
    if (m.type() === "error" && !m.text().includes("net::"))
      console.log("console:", m.text());
  });
  await page.addInitScript(
    () =>
      (window.__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
      }),
  );
  await page.route("https://**/*", (r) => r.abort());
  await page.route("**/test-video.webm", (r) =>
    r.fulfill({
      status: 200,
      contentType: "video/webm",
      body: fs.readFileSync(
        path.resolve(__dirname, "../tests/fixtures/background.webm"),
      ),
    }),
  );
  await page.route("**/src/main.jsx*", (r) =>
    r.fulfill({ contentType: "text/javascript", body: entry }),
  );
  const go = async (state) => {
    await page.goto("http://127.0.0.1:1420/?state=" + state);
    await page
      .locator(".home-page__action-area")
      .waitFor()
      .catch(async (e) => {
        console.log((await page.locator("body").innerText()).slice(0, 2000));
        throw e;
      });
    await page.waitForTimeout(300);
  };
  const shot = async (name) => {
    await page.waitForTimeout(300);
    await page.screenshot({ path: out + "/" + name + ".png" });
  };
  const settings = () =>
    page.getByRole("button", { name: "Настройки", exact: true }).click();
  const nav = (name) =>
    page
      .locator(".settings-nav")
      .getByRole("button", { name, exact: true })
      .click();
  await go("ready");
  await page.getByRole("button", { name: "Играть", exact: true }).waitFor();
  // Use the actual serialized API key (tabName); blank tabs escaped older mocks.
  await page.getByRole("tab", { name: "Уведомления", exact: true }).waitFor();
  await page.getByRole("tab", { name: "События", exact: true }).click();
  await page.getByRole("tab", { name: "Уведомления", exact: true }).click();
  await page.waitForFunction(() => {
    const img = document.querySelector(".main-layout__background img");
    return img?.naturalWidth > 0 && img.classList.contains("loaded");
  });
  await shot("home-ready");
  assert(
    await page
      .locator(".social-sidebar__app-icon")
      .evaluate((img) => img.complete && img.naturalWidth > 0),
  );
  await page
    .getByRole("button", { name: "Следующее объявление", exact: true })
    .click();
  await page
    .getByRole("link", { name: "Открыть объявление 2", exact: true })
    .click();
  assert(
    await page.evaluate(() =>
      testState.calls.some(
        (call) =>
          call.cmd === "plugin:opener|open_url" &&
          call.args.url === "https://example.com/banner2",
      ),
    ),
  );
  await page
    .getByRole("button", { name: "Действия с игрой", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Запуск с модами", exact: true })
    .waitFor();
  await page.keyboard.press("Escape");
  assert.equal(await page.locator(".action-menu__items").count(), 0);
  await page
    .getByRole("button", { name: "Действия с игрой", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Запуск с модами", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Запускаем игру", exact: true })
    .waitFor();
  assert(
    await page.evaluate(() =>
      testState.calls.some(
        (call) => call.cmd === "launch_game" && call.args.withMods === true,
      ),
    ),
  );
  await go("ready");
  await settings();
  await shot("settings-general");
  await page
    .getByRole("switch", { name: "Запускать вместе с системой" })
    .click();
  assert.equal(await page.evaluate(() => testState.auto), false);
  await page.getByRole("button", { name: "Сохранить", exact: true }).click();
  await page.waitForTimeout(150);
  assert.equal(await page.evaluate(() => testState.auto), true);
  await page
    .getByRole("switch", { name: "Запускать вместе с системой" })
    .click();
  await page.evaluate(() => (testState.saveError = true));
  await page.getByRole("button", { name: "Сохранить", exact: true }).click();
  await page.getByRole("alert").waitFor();
  assert.equal(await page.evaluate(() => testState.auto), true);
  await page.evaluate(() => (testState.saveError = false));
  await page.getByRole("button", { name: "Сохранить", exact: true }).click();
  await nav("Запуск");
  await page.getByRole("switch", { name: "Счётчик FPS" }).click();
  await nav("Моды");
  await shot("settings-mods");
  assert(
    !/Внешность|Массовых банов|Minecraft|кадры просядут/.test(
      await page.locator(".settings-content").innerText(),
    ),
  );
  await nav("К игре");
  await page.getByRole("dialog").waitFor();
  assert(
    !(await page
      .getByRole("button", { name: "Не сохранять", exact: true })
      .evaluate((el) => el === document.activeElement)),
  );
  for (let i = 0; i < 8; i++) {
    await page.keyboard.press("Tab");
    assert(
      await page.evaluate(
        () => !!document.activeElement.closest("[role=dialog]"),
      ),
    );
  }
  await page.keyboard.press("Escape");
  assert.equal(await page.getByRole("dialog").count(), 0);
  await page.getByRole("button", { name: "Сохранить", exact: true }).click();
  await nav("Запуск");
  await shot("settings-launch");
  await page.getByText("Дополнительные параметры", { exact: true }).click();
  await page.getByRole("switch", { name: "Gamescope", exact: true }).click();
  await page.getByLabel("Режим окна", { exact: true }).waitFor();
  await nav("Игра и файлы");
  await shot("settings-files");
  await nav("Диагностика");
  await shot("settings-diagnostics");
  await page
    .getByRole("button", { name: "Посмотреть лог", exact: true })
    .click()
    .catch(() =>
      page
        .locator(".settings-content")
        .getByRole("button")
        .filter({ hasText: /лог|журнал/i })
        .first()
        .click(),
    );
  await page.getByRole("dialog").waitFor();
  await page.keyboard.press("Escape");
  await go("missing");
  await page
    .getByRole("button", { name: "Установить Proton", exact: true })
    .waitFor();
  assert(!(await page.getByText("Готова к игре", { exact: true }).count()));
  await shot("home-proton");
  await page
    .getByRole("button", { name: "Выбрать установленную сборку", exact: false })
    .click();
  await page.getByText("Другие сборки", { exact: true }).waitFor();
  await go("first");
  await page
    .getByRole("button", { name: "Установить игру и Proton", exact: true })
    .waitFor();
  await shot("home-install");
  await page.locator(".home-page__install-details > summary").click();
  await page.locator(".home-page__folder").waitFor();
  await page.keyboard.press("Escape");
  assert.equal(
    await page.locator(".home-page__install-details").getAttribute("open"),
    null,
  );
  await page
    .getByRole("button", { name: "Установить игру и Proton", exact: true })
    .click();
  await page.evaluate(() =>
    testState.progress("proton", {
      stage: "downloading",
      bytes_downloaded: 200,
      bytes_total: 400,
      speed_bps: 100,
    }),
  );
  await settings();
  await nav("Запуск");
  assert(await page.getByRole("progressbar").count());
  await nav("К игре");
  await page.evaluate(() => testState.complete("proton"));
  await page.waitForFunction(
    () => testState.transfers.game?.status === "running",
  );
  await page.evaluate(() =>
    testState.progress("game", {
      stage: "downloading",
      bytes_downloaded: 24000000000,
      bytes_total: 48000000000,
      speed_bps: 18000000,
    }),
  );
  await shot("home-download");
  await page.getByRole("button", { name: "Пауза", exact: true }).click();
  await page
    .getByRole("button", { name: "Продолжить загрузку", exact: true })
    .waitFor();
  await shot("home-paused");
  await settings();
  await nav("К игре");
  await page
    .getByRole("button", { name: "Продолжить загрузку", exact: true })
    .click();
  await page.evaluate(() =>
    testState.progress("game", {
      stage: "extracting",
      bytes_processed: 50,
      bytes_total: 100,
      percent: 50,
    }),
  );
  await shot("home-extract");
  assert.equal(
    await page.getByRole("button", { name: "Пауза", exact: true }).count(),
    0,
  );
  await page.evaluate(() => testState.complete("game"));
  await page.getByRole("button", { name: "Играть", exact: true }).waitFor();
  await go("offline");
  await page.getByRole("button", { name: "Повторить", exact: true }).waitFor();
  await shot("home-offline");
  await page.evaluate(() => (testState.offline = false));
  await page.getByRole("button", { name: "Повторить", exact: true }).click();
  await page.getByRole("button", { name: "Играть", exact: true }).waitFor();
  await page.getByRole("button", { name: "Играть", exact: true }).click();
  assert(
    await page
      .getByRole("button", { name: "Запускаем игру", exact: true })
      .isDisabled(),
  );
  assert.equal(await page.evaluate(() => testState.launches), 1);
  await page.evaluate(() => testState.launch.reject("Test launch failure"));
  await page.getByRole("alert").waitFor();
  await go("update");
  await shot("home-update");
  for (const [width, height] of [
    [1280, 720],
    [1024, 600],
    [1280, 800],
  ]) {
    await page.setViewportSize({ width, height });
    const box = await page.locator(".action-button").boundingBox();
    assert(
      box.y >= 0 && box.y + box.height <= height,
      JSON.stringify({ width, height, box }),
    );
  }
  await page.setViewportSize({ width: 1280, height: 720 });
  await go("windows");
  await settings();
  await nav("Запуск");
  assert.equal(await page.getByText("Proton", { exact: true }).count(), 0);
  await page
    .getByRole("switch", { name: "Запускать от администратора", exact: true })
    .waitFor()
    .catch(() => page.getByRole("switch").first().waitFor());
  await shot("settings-windows");
  await go("disk-full");
  assert(await page.locator(".action-button").isDisabled());
  await page.getByText("Недостаточно места", { exact: true }).waitFor();
  await shot("home-disk-full");
  await go("task-error");
  await page
    .getByText("Не удалось проверить загрузки", { exact: true })
    .waitFor();
  await page.evaluate(() => (testState.tasksRecovered = true));
  await page.getByRole("button", { name: "Повторить", exact: true }).click();
  await page.getByRole("button", { name: "Играть", exact: true }).waitFor();
  await go("paused");
  await page
    .getByRole("button", { name: "Продолжить загрузку", exact: true })
    .waitFor();
  assert(
    (await page.locator(".progress-bar__info").innerText()).includes("18.6 GB"),
  );
  await go("ready");
  await settings();
  await nav("Игра и файлы");
  await page
    .getByLabel("Папка игры", { exact: true })
    .fill("/home/player/Games/New");
  assert(
    await page
      .getByRole("button", { name: "Проверить…", exact: true })
      .isDisabled(),
  );
  await page
    .getByText("Сохраните новые пути перед работой с файлами.", { exact: true })
    .waitFor();
  await page.getByRole("button", { name: "Сохранить", exact: true }).click();
  assert(
    await page
      .getByRole("button", { name: "Проверить…", exact: true })
      .isEnabled(),
  );
  for (const [width, height] of [
    [1024, 600],
    [1280, 720],
  ]) {
    await page.setViewportSize({ width, height });
    await go("first");
    const box = await page.locator(".action-button").boundingBox();
    assert(
      box.y >= 0 && box.y + box.height <= height,
      JSON.stringify({ width, height, box }),
    );
    await shot("home-install-" + width);
    await settings();
    await nav("Моды");
    const save = await page
      .getByRole("button", { name: "Сохранить", exact: true })
      .boundingBox();
    assert(save.y + save.height <= height);
    assert(
      await page
        .locator(".settings-content")
        .evaluate((el) => el.scrollWidth <= el.clientWidth),
    );
  }
  await go("en");
  await page.getByRole("button", { name: "Play", exact: true }).waitFor();
  await shot("home-english");
  // Real media is mocked, so these test control flow and recovery from interruption.
  await go("ready");
  await page.evaluate(() => {
    window.testState.mediaCalls = [];
    window.dispatchEvent(new Event("focus"));
  });
  await page.waitForTimeout(100);
  assert((await page.evaluate(() => testState.mediaCalls)).includes("play"));
  for (let i = 0; i < 5; i++) {
    await page.evaluate(() => {
      testState.visible = false;
      testState.mediaCalls = [];
      Object.defineProperty(document, "hidden", {
        configurable: true,
        get: () => true,
      });
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await page.waitForTimeout(50);
    assert((await page.evaluate(() => testState.mediaCalls)).includes("pause"));
    await page.evaluate(() => {
      testState.visible = true;
      testState.mediaCalls = [];
      testState.abortNext = true;
      Object.defineProperty(document, "hidden", {
        configurable: true,
        get: () => false,
      });
      document.dispatchEvent(new Event("visibilitychange"));
      window.dispatchEvent(new Event("focus"));
    });
    await page.waitForTimeout(75);
    await page.evaluate(() => window.dispatchEvent(new Event("focus")));
    await page.waitForTimeout(75);
    assert.equal(await page.locator("video").count(), 1);
    assert((await page.evaluate(() => testState.mediaCalls)).includes("play"));
  }
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.locator("video").waitFor({ state: "detached" });
  assert.equal(await page.locator(".main-layout__background img").count(), 1);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await go("real-video");
  await page.waitForFunction(
    () => document.querySelector("video")?.currentTime > 0.1,
  );
  for (let i = 0; i < 5; i++) {
    await page.evaluate(() => {
      testState.minimized = true;
      window.dispatchEvent(new Event("blur"));
    });
    await page.waitForFunction(() => document.querySelector("video")?.paused);
    await page.evaluate(() => {
      testState.minimized = false;
      window.dispatchEvent(new Event("focus"));
    });
    await page.waitForFunction(() => !document.querySelector("video")?.paused);
    const before = await page.locator("video").evaluate((v) => v.currentTime);
    await page.waitForFunction(
      (before) => document.querySelector("video")?.currentTime !== before,
      before,
    );
  }
  // WebKit can keep document.hidden true after the native window is restored.
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      get: () => true,
    });
    document.dispatchEvent(new Event("visibilitychange"));
    window.dispatchEvent(new Event("focus"));
  });
  await page.waitForFunction(() => !document.querySelector("video")?.paused);
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      get: () => false,
    });
  });
  // Restore without a focus event: native visibility polling must resume playback.
  await page.evaluate(() => (testState.visible = false));
  await page.waitForFunction(() => document.querySelector("video")?.paused);
  await page.evaluate(() => (testState.visible = true));
  await page.waitForFunction(() => !document.querySelector("video")?.paused);
  assert.deepEqual(errors, []);
  console.log(
    "PASS: full UI flows, settings save/rollback, focus trap, Proton sequence, pause/resume across screens, errors/retry, platform/language, viewports, repeated video recovery; no page errors",
  );
  await browser.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
