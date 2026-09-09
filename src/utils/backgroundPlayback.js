// Some WebKit media pipelines stay "playing" after a window is restored while
// no frames advance. Recover in stages, and reload only after a real stall.
export function createBackgroundPlayback(
  video,
  { now = () => performance.now(), onRecover = () => {} } = {},
) {
  let stopped = false;
  let lastTime = video.currentTime;
  let lastFrames = frameCount();
  let lastMovement = now();
  let softRecovery = false;
  let reloadAttempts = 0;
  let nextReload = 0;
  let restorePosition;

  function frameCount() {
    const frames =
      video.getVideoPlaybackQuality?.().totalVideoFrames ??
      video.webkitDecodedFrameCount;
    return Number.isFinite(frames) && frames > 0 ? frames : null;
  }
  function play() {
    if (stopped) return;
    // A visibility change can interrupt a pending play(). The next check retries
    // it; an AbortError must not permanently remove the video element.
    try {
      video.play()?.catch(() => {});
    } catch {
      /* Retry on the next check. */
    }
  }
  function clearRestore() {
    if (restorePosition)
      video.removeEventListener("loadedmetadata", restorePosition);
    restorePosition = undefined;
  }
  function reload() {
    if (now() < nextReload) return;
    const position = video.currentTime;
    clearRestore();
    restorePosition = () => {
      clearRestore();
      if (stopped) return;
      if (
        Number.isFinite(position) &&
        position > 0 &&
        Number.isFinite(video.duration)
      ) {
        try {
          video.currentTime = Math.min(
            position,
            Math.max(0, video.duration - 0.1),
          );
        } catch {
          /* A stream may not be seekable yet. */
        }
      }
      play();
    };
    video.addEventListener("loadedmetadata", restorePosition, { once: true });
    nextReload =
      now() + Math.min(60000, 10000 * 2 ** Math.min(reloadAttempts++, 3));
    lastMovement = now();
    onRecover();
    video.load();
    play();
  }
  function check() {
    if (stopped) return;
    if (video.error) {
      reload();
      return;
    }
    const frames = frameCount();
    const advanced =
      frames !== null && lastFrames !== null
        ? frames !== lastFrames
        : video.currentTime !== lastTime;
    lastTime = video.currentTime;
    lastFrames = frames;
    if (advanced) {
      lastMovement = now();
      softRecovery = false;
      reloadAttempts = 0;
    }
    if (video.paused) play();
    const stalledFor = now() - lastMovement;
    if (stalledFor >= 9000) reload();
    else if (stalledFor >= 4500 && !softRecovery) {
      softRecovery = true;
      video.pause();
      play();
    }
  }
  play();
  return {
    check,
    stop() {
      stopped = true;
      clearRestore();
      video.pause();
    },
  };
}
