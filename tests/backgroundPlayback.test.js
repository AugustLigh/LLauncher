import test from "node:test";
import assert from "node:assert/strict";
import { createBackgroundPlayback } from "../src/utils/backgroundPlayback.js";

function setup({ frames = false, abort = false } = {}) {
  let time = 0;
  class Video extends EventTarget {
    currentTime = 12;
    duration = 30;
    paused = true;
    calls = [];
    frames = 10;
    play() {
      this.calls.push("play");
      if (abort) {
        abort = false;
        return Promise.reject(new DOMException("Interrupted", "AbortError"));
      }
      this.paused = false;
      return Promise.resolve();
    }
    pause() {
      this.calls.push("pause");
      this.paused = true;
    }
    load() {
      this.calls.push("load");
      this.currentTime = 0;
    }
  }
  const video = new Video();
  if (frames)
    video.getVideoPlaybackQuality = () => ({ totalVideoFrames: video.frames });
  const playback = createBackgroundPlayback(video, { now: () => time });
  return {
    video,
    playback,
    tick(ms) {
      time += ms;
      playback.check();
    },
  };
}

test("interrupted play is retried without removing or reloading the video", async () => {
  const { video, tick, playback } = setup({ abort: true });
  await Promise.resolve();
  assert.equal(video.paused, true);
  tick(1500);
  assert.equal(video.paused, false);
  assert.deepEqual(video.calls, ["play", "play"]);
  playback.stop();
});

test("a playing video with stalled time is restarted, then reloaded and seeks back", () => {
  const { video, tick, playback } = setup();
  tick(4500);
  assert.deepEqual(video.calls, ["play", "pause", "play"]);
  tick(4500);
  assert.equal(video.calls.filter((c) => c === "load").length, 1);
  video.dispatchEvent(new Event("loadedmetadata"));
  assert.equal(video.currentTime, 12);
  playback.stop();
});

test("frozen decoded frames are detected even if the media clock still advances", () => {
  const { video, tick, playback } = setup({ frames: true });
  video.currentTime += 5;
  tick(4500);
  assert(video.calls.includes("pause"));
  video.frames++;
  tick(4500);
  assert(!video.calls.includes("load"));
  playback.stop();
});

test("normal playback and loops never cause a reload", () => {
  const { video, tick, playback } = setup();
  for (let i = 0; i < 50; i++) {
    video.currentTime = (video.currentTime + 1.5) % 30;
    tick(1500);
  }
  assert.deepEqual(video.calls, ["play"]);
  playback.stop();
});

test("persistent media errors back off and cleanup cancels metadata recovery", () => {
  const { video, tick, playback } = setup();
  video.error = { code: 3 };
  tick(1500);
  for (let i = 0; i < 6; i++) tick(1500);
  assert.equal(video.calls.filter((c) => c === "load").length, 1);
  tick(1500);
  assert.equal(video.calls.filter((c) => c === "load").length, 2);
  playback.stop();
  const before = video.calls.length;
  video.dispatchEvent(new Event("loadedmetadata"));
  tick(60000);
  assert.equal(video.calls.length, before);
  assert.equal(video.paused, true);
});
