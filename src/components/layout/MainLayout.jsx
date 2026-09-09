import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createBackgroundPlayback } from "../../utils/backgroundPlayback";
import "./MainLayout.css";
function Background({ background, gameRunning }) {
  const [hidden, setHidden] = useState(document.hidden),
    [visible, setVisible] = useState(null),
    [focused, setFocused] = useState(document.hasFocus()),
    [epoch, setEpoch] = useState(0);
  const [reduced, setReduced] = useState(
    () => matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  const [imageLoaded, setImageLoaded] = useState(false),
    [videoLoaded, setVideoLoaded] = useState(false);
  const imageRef = useRef(null),
    videoRef = useRef(null);
  const imageUrl = background?.url,
    videoUrl = !reduced ? background?.video_url : null;
  useEffect(() => {
    // A cached image can load before this effect runs, particularly in WebKit.
    const image = imageRef.current;
    setImageLoaded(Boolean(image?.complete && image.naturalWidth > 0));
    setVideoLoaded(false);
  }, [imageUrl, videoUrl]);
  useEffect(() => {
    let disposed = false,
      checkId = 0;
    const win = getCurrentWindow();
    const sync = async () => {
      const id = ++checkId;
      try {
        const [shown, minimized, nativeFocused] = await Promise.all([
          win.isVisible(),
          win.isMinimized(),
          win.isFocused(),
        ]);
        if (!disposed && id === checkId) {
          if (typeof shown === "boolean" && typeof minimized === "boolean")
            setVisible(shown && !minimized);
          if (typeof nativeFocused === "boolean") setFocused(nativeFocused);
        }
      } catch {
        /* Browser previews don't implement native visibility. */
      }
    };
    const resume = () => {
      setEpoch((v) => v + 1);
      sync();
    };
    const onVisibility = () => {
      setHidden(document.hidden);
      setVisible(null);
      if (!document.hidden) resume();
      else sync();
    };
    const onFocus = () => {
      setFocused(true);
      resume();
    };
    const onBlur = () => {
      setFocused(false);
      sync();
    };
    const media = matchMedia("(prefers-reduced-motion: reduce)");
    const onMotion = () => setReduced(media.matches);
    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);
    media.addEventListener("change", onMotion);
    const pending = win.onFocusChanged(({ payload }) => {
      if (disposed) return;
      setFocused(payload);
      if (payload) resume();
      else sync();
    });
    // WebKitGTK can omit a visibility event when a tray-hidden window is shown.
    // This lightweight probe is a fallback; focus still resumes immediately.
    const timer = setInterval(() => {
      if (!disposed) sync();
    }, 2000);
    sync();
    return () => {
      disposed = true;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
      media.removeEventListener("change", onMotion);
      pending.then((fn) => fn()).catch(() => {});
    };
  }, []);
  // Native window state also handles WebKit's stale document.hidden value
  // after restoring a tray-hidden window. DOM visibility remains a fallback.
  const shouldPause =
    (visible === null ? hidden : !visible) || (gameRunning && !focused);
  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    if (shouldPause) {
      video.pause();
      return;
    }
    const playback = createBackgroundPlayback(video, {
      onRecover: () => setVideoLoaded(false),
    });
    const watchdog = setInterval(playback.check, 1500);
    return () => {
      clearInterval(watchdog);
      playback.stop();
    };
  }, [videoUrl, shouldPause, epoch]);
  return (
    <div
      className="main-layout__background"
      data-loaded={imageLoaded || videoLoaded}
      aria-hidden="true"
    >
      {imageUrl && (
        <img
          ref={imageRef}
          src={imageUrl}
          alt=""
          className={imageLoaded ? "loaded" : ""}
          onLoad={() => setImageLoaded(true)}
          onError={() => setImageLoaded(false)}
        />
      )}
      {videoUrl && (
        <video
          ref={videoRef}
          src={videoUrl}
          poster={imageUrl || undefined}
          className={videoLoaded ? "loaded" : ""}
          preload="metadata"
          loop
          muted
          playsInline
          onPlaying={() => setVideoLoaded(true)}
          onError={() => setVideoLoaded(false)}
        />
      )}
    </div>
  );
}
export default function MainLayout({ background, paused, children }) {
  return (
    <div className="main-layout">
      <Background background={background} gameRunning={paused} />
      <div className="main-layout__overlay" />
      <div className="main-layout__content">{children}</div>
    </div>
  );
}
