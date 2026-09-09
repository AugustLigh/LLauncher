import test from "node:test";
import assert from "node:assert/strict";
import { launcherState } from "../src/utils/launcherState.js";

const ready = {
  gameState: { status: "ready" },
  systemCheck: { platform: "linux", has_proton: true },
};
test("Play is gated by environment readiness only on Linux", () => {
  assert.equal(launcherState(ready), "ready");
  assert.equal(
    launcherState({
      ...ready,
      systemCheck: { platform: "linux", has_proton: false },
    }),
    "needsProton",
  );
  assert.equal(
    launcherState({ ...ready, systemCheck: { platform: "windows" } }),
    "ready",
  );
  assert.equal(
    launcherState({ ...ready, systemLoading: true }),
    "systemChecking",
  );
  assert.equal(
    launcherState({ ...ready, systemError: "Unavailable" }),
    "systemError",
  );
});
test("active and paused work survives version refreshes and takes priority over Play", () => {
  assert.equal(
    launcherState({
      ...ready,
      gameLoading: true,
      task: { status: "running", progress: { stage: "extracting" } },
    }),
    "extracting",
  );
  assert.equal(
    launcherState({ ...ready, task: { status: "paused" } }),
    "paused",
  );
  assert.equal(
    launcherState({ ...ready, task: { status: "error" } }),
    "downloadError",
  );
  assert.equal(
    launcherState({ ...ready, protonTask: { status: "running" } }),
    "protonDownloading",
  );
});
test("launch cannot be offered during startup, launch or unknown task state", () => {
  assert.equal(launcherState({ ...ready, launching: true }), "launching");
  assert.equal(launcherState({ ...ready, running: true }), "running");
  assert.equal(launcherState({ ...ready, tasksLoading: true }), "checking");
  assert.equal(
    launcherState({ ...ready, tasksError: "Unavailable" }),
    "taskError",
  );
});
