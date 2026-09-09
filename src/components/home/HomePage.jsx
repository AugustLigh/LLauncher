import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import ActionButton from "./ActionButton";
import ProgressBar from "./ProgressBar";
import NewsPanel from "./NewsPanel";
import BannerCarousel from "./BannerCarousel";
import ActionMenu from "./ActionMenu";
import ConfirmDialog from "../common/ConfirmDialog";
import ErrorNotice from "../common/ErrorNotice";
import { Button, Status } from "../common/Controls";
import Icon from "../common/Icon";
import useTasks, { taskActive } from "../../hooks/useTasks";
import useGameStats from "../../hooks/useGameStats";
import { launcherState } from "../../utils/launcherState";
import { formatSize, formatPlaytime, formatDate } from "../../utils/format";
import { useTranslation } from "../../i18n";
import "./HomePage.css";
export default function HomePage({
  content,
  contentLoading,
  contentError,
  onRetryContent,
  settings,
  systemCheck,
  systemLoading,
  systemError,
  onRetrySystem,
  gameState,
  gameLoading,
  gameError,
  onRetryGameState,
  gameRunning,
  markRunning,
  onSync,
  onSaveSettings,
  onOpenSettings,
  hidden,
}) {
  const { t, locale } = useTranslation(),
    stats = useGameStats();
  const {
    tasks,
    loading: tasksLoading,
    loadError: tasksError,
    start,
    stop,
    refresh: refreshTasks,
    busy,
  } = useTasks();
  const [launching, setLaunching] = useState(false),
    [importing, setImporting] = useState(false),
    [localError, setLocalError] = useState(null),
    [confirmation, setConfirmation] = useState(null),
    [plan, setPlan] = useState(null),
    [planError, setPlanError] = useState(null),
    [planLoading, setPlanLoading] = useState(false);
  const launchLock = useRef(false),
    installLock = useRef(false),
    planRequest = useRef(0),
    latestSync = useRef(onSync);
  const installDetails = useRef(null);
  useEffect(() => {
    const close = (event) => {
      const details = installDetails.current;
      if (details?.open && !details.contains(event.target))
        details.open = false;
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, []);
  useEffect(() => {
    if (hidden && installDetails.current) installDetails.current.open = false;
  }, [hidden]);
  latestSync.current = onSync;
  const task =
    tasks.game &&
    (!tasks.game.game_dir ||
      tasks.game.game_dir === settings?.game_dir ||
      taskActive(tasks.game))
      ? tasks.game
      : null;
  const protonTask = tasks.proton?.status === "completed" ? null : tasks.proton;
  const state = launcherState({
    running: gameRunning,
    launching,
    importing,
    gameLoading,
    gameError,
    systemLoading,
    systemError,
    systemCheck,
    gameState,
    task,
    protonTask,
    tasksLoading,
    tasksError,
  });
  const isLinux = systemCheck?.platform !== "windows";
  const needsProton = isLinux && systemCheck && !systemCheck.has_proton;
  const loadPlan = useCallback(async () => {
    const id = ++planRequest.current;
    setPlanLoading(true);
    setPlanError(null);
    setPlan(null);
    try {
      const next = await invoke("get_install_plan");
      if (id === planRequest.current) setPlan(next);
    } catch (e) {
      if (id === planRequest.current) setPlanError(e);
    } finally {
      if (id === planRequest.current) setPlanLoading(false);
    }
  }, [settings?.game_dir, settings?.download_dir]);
  useEffect(() => {
    if (gameState?.status === "not_installed" && settings) loadPlan();
    return () => {
      planRequest.current++;
    };
  }, [gameState?.status, loadPlan]);
  useEffect(() => {
    const pending = listen("launch://update-required", () =>
      latestSync.current(),
    );
    return () => {
      pending.then((u) => u());
    };
  }, []);
  const chooseFolder = async (existing = false) => {
    if (busy || importing) return;
    setLocalError(null);
    try {
      const dir = await open({
        directory: true,
        title: t(existing ? "ui.existingGame" : "ui.chooseFolder"),
      });
      if (!dir) return;
      setImporting(true);
      if (existing) await invoke("import_existing_game", { path: dir });
      else {
        const fresh = await invoke("get_settings");
        const sep = dir.includes("\\") ? "\\" : "/";
        const oldDefault =
          fresh.download_dir.replace(/\\/g, "/") ===
          `${fresh.game_dir.replace(/\\/g, "/")}/_download`;
        await onSaveSettings({
          ...fresh,
          game_dir: dir,
          download_dir: oldDefault
            ? `${dir}${sep}_download`
            : fresh.download_dir,
        });
      }
      await onSync();
    } catch (e) {
      setLocalError({ title: t("errors.importFailed"), error: e });
    } finally {
      setImporting(false);
    }
  };
  const launch = async (withMods = false) => {
    if (launchLock.current || state !== "ready") return;
    launchLock.current = true;
    setLaunching(true);
    setLocalError(null);
    try {
      await invoke("launch_game", { withMods });
      markRunning();
      if (["hide", "close"].includes(settings?.on_launch_action || "hide"))
        await getCurrentWindow().hide();
    } catch (e) {
      setLocalError({ title: t("ui.launchFailed"), error: e });
      await onSync();
    } finally {
      launchLock.current = false;
      setLaunching(false);
    }
  };
  const install = async () => {
    if (installLock.current || busy) return;
    installLock.current = true;
    setLocalError(null);
    try {
      if (needsProton) {
        if (!(await start("proton"))) return;
        await onSync();
      }
      await start("install");
    } finally {
      installLock.current = false;
    }
  };
  const stopTask = async (slot, discard = false) => {
    try {
      await stop(slot, discard);
    } catch (e) {
      setLocalError({ title: t("ui.downloadFailed"), error: e });
    }
  };
  const confirmCancel = (slot) =>
    setConfirmation({
      title: t(slot === "proton" ? "ui.protonCancel" : "ui.cancelTitle"),
      message: t(slot === "proton" ? "ui.protonCancelBody" : "ui.cancelBody"),
      label: t(slot === "proton" ? "ui.stop" : "ui.cancelConfirm"),
      run: () => stopTask(slot, true),
    });
  const resume = () =>
    start(
      task?.kind ||
        (gameState?.status === "update_available" ? "update" : "install"),
    );
  const version =
    gameState?.version ||
    gameState?.installed_version ||
    gameState?.latest_version;
  const labels = {
    versionError: "versionFailed",
    systemError: "systemFailed",
    downloadError: "downloadFailed",
    protonError: "needsProton",
    taskError: "tasksFailed",
    update: "updateTitle",
  };
  const progressState = [
    "downloading",
    "fetching",
    "verifying",
    "extracting",
    "pausing",
    "paused",
  ].includes(state);
  const errorState = [
    "versionError",
    "systemError",
    "downloadError",
    "protonError",
    "taskError",
  ].includes(state);
  const primaryAction =
    state === "ready"
      ? () => launch(false)
      : state === "notInstalled"
        ? install
        : state === "update"
          ? () => start("update")
          : state === "needsProton" || state === "protonError"
            ? () => start("proton")
            : state === "downloadError"
              ? resume
              : state === "versionError"
                ? onRetryGameState
                : state === "systemError"
                  ? onRetrySystem
                  : state === "taskError"
                    ? refreshTasks
                    : null;
  const primaryLabel =
    state === "ready"
      ? t("home.action.launch")
      : state === "notInstalled"
        ? t(needsProton ? "ui.installBoth" : "home.action.install")
        : state === "update"
          ? t("ui.updateAction")
          : ["needsProton", "protonError"].includes(state)
            ? t("ui.installProton")
            : errorState
              ? t("common.retry")
              : t(`ui.${labels[state] || state}`);
  const menuItems = [
    ...(state === "ready" && settings?.mods_enabled
      ? [
          {
            label: t("home.action.launchMods"),
            icon: "mods",
            onSelect: () => launch(true),
          },
        ]
      : []),
    {
      label: t("ui.gameSettings"),
      icon: "settings",
      onSelect: () => onOpenSettings("files"),
    },
    ...(state === "notInstalled"
      ? [
          {
            label: t("ui.existingGame"),
            icon: "folder",
            onSelect: () => chooseFolder(true),
            disabled: busy,
          },
        ]
      : []),
  ];
  return (
    <main className="home-page" data-state={state} hidden={hidden}>
      <h1 className="home-page__hero">Arknights: Endfield</h1>
      <div className="home-page__news">
        <BannerCarousel banners={content?.banners} />
        <NewsPanel
          tabs={content?.news_tabs}
          loading={contentLoading}
          error={contentError}
          onRetry={onRetryContent}
        />
      </div>
      <section
        className="home-page__action-area"
        data-state={state}
        aria-label={t("ui.gameSettings")}
      >
        {!progressState &&
          !errorState &&
          !["update", "protonDownloading"].includes(state) && (
            <div className="home-page__card-meta">
              <Status
                kind={
                  errorState
                    ? "error"
                    : state === "ready"
                      ? "success"
                      : ["needsProton", "paused", "update"].includes(state)
                        ? "warning"
                        : "neutral"
                }
                busy={[
                  "checking",
                  "systemChecking",
                  "importing",
                  "launching",
                ].includes(state)}
              >
                {t(`ui.${labels[state] || state}`)}
              </Status>
              {version && (
                <span className="home-page__version">v{version}</span>
              )}
            </div>
          )}
        {localError && <ErrorNotice {...localError} />}
        {state === "notInstalled" && (
          <details
            ref={installDetails}
            className="home-page__install-details"
            open={plan?.blocked || !!planError || undefined}
            onKeyDown={(event) => {
              if (event.key === "Escape" && event.currentTarget.open) {
                event.stopPropagation();
                event.currentTarget.open = false;
                event.currentTarget.querySelector("summary").focus();
              }
            }}
          >
            <summary>
              <span>
                {t("ui.installTitle")}
                {plan?.download_bytes != null &&
                  ` · ${formatSize(plan.download_bytes)}`}
              </span>
              <Icon name="chevron" size={14} />
            </summary>
            <div className="home-page__install-popover">
              <h2>{t("ui.installTitle")}</h2>
              <div className="home-page__folder">
                <Icon name="folder" />
                <div>
                  <span>
                    {settings?.game_dir?.split(/[\\/]/).filter(Boolean).pop()}
                  </span>
                  <small title={settings?.game_dir}>{settings?.game_dir}</small>
                </div>
                <Button
                  variant="ghost"
                  onClick={() => chooseFolder(false)}
                  disabled={busy}
                >
                  {t("ui.change")}
                </Button>
              </div>
              {planLoading && <small>{t("ui.sizeUnknown")}</small>}
              {plan && (
                <div className="home-page__installation">
                  <div>
                    <span>{t("ui.downloadSize")}</span>
                    <span>{formatSize(plan.download_bytes)}</span>
                  </div>
                  {plan.unpacked_bytes != null && (
                    <div>
                      <span>{t("ui.gameSpace")}</span>
                      <span>{formatSize(plan.unpacked_bytes)}</span>
                    </div>
                  )}
                  {plan.disks?.map((disk, i) => (
                    <div className="home-page__disk" key={i} title={disk.path}>
                      <span>{t("ui.diskFree")}</span>
                      <span>
                        {disk.available == null
                          ? t("ui.unknown")
                          : formatSize(disk.available)}
                      </span>
                      <div className="home-page__disk-bar">
                        <span
                          style={{
                            width:
                              disk.available > 0
                                ? `${Math.min(100, (disk.required / disk.available) * 100)}%`
                                : "0%",
                          }}
                        />
                      </div>
                    </div>
                  ))}
                  {plan.unpacked_bytes == null && (
                    <small>{t("ui.totalSpaceUnknown")}</small>
                  )}
                  {plan.blocked && (
                    <span className="home-page__space-error">
                      {t("ui.notEnoughSpace")}
                    </span>
                  )}
                </div>
              )}
              {systemError && (
                <ErrorNotice
                  title={t("ui.systemFailed")}
                  error={systemError}
                  onRetry={onRetrySystem}
                />
              )}
              {planError && (
                <ErrorNotice
                  title={t("ui.planFailed")}
                  error={planError}
                  onRetry={loadPlan}
                />
              )}
              {needsProton && (
                <div className="home-page__runtime-line">
                  <Icon name="download" size={15} />
                  <span>Proton</span>
                  <small>{t("ui.recommendedBuild")}</small>
                </div>
              )}
            </div>
          </details>
        )}
        {["needsProton", "protonError"].includes(state) && (
          <>
            <h2>{t("ui.protonTitle")}</h2>
            <div className="home-page__checklist">
              <Status kind="success">{t("ui.gameFound")}</Status>
              <span>{t("ui.runtime")} · Proton</span>
            </div>
          </>
        )}
        {state === "update" && (
          <div className="home-page__update">
            <h2>{t("ui.updateTitle")}</h2>
            <span>
              v{gameState?.installed_version} <Icon name="arrow" size={15} /> v
              {gameState?.latest_version}
            </span>
          </div>
        )}
        {progressState && (
          <ProgressBar
            progress={task?.progress}
            paused={state === "paused"}
            stopping={state === "pausing"}
            onResume={state === "paused" ? resume : null}
            onPause={
              taskActive(task) && task?.progress?.stage !== "extracting"
                ? () => stopTask("game")
                : null
            }
            onCancel={
              state !== "extracting" ? () => confirmCancel("game") : null
            }
          />
        )}
        {state === "protonDownloading" && (
          <ProgressBar
            progress={protonTask?.progress}
            proton
            stopping={protonTask?.status !== "running"}
            onCancel={
              protonTask?.progress?.stage !== "extracting"
                ? () => confirmCancel("proton")
                : null
            }
          />
        )}
        {errorState && (
          <ErrorNotice
            title={t(`ui.${labels[state] || state}`)}
            error={
              state === "versionError"
                ? gameError
                : state === "systemError"
                  ? systemError
                  : state === "taskError"
                    ? tasksError
                    : state === "protonError"
                      ? protonTask?.error
                      : task?.error
            }
          />
        )}
        {!progressState &&
          state !== "protonDownloading" &&
          state !== "running" && (
            <div className="home-page__launch-controls">
              <ActionButton
                onClick={primaryAction}
                disabled={
                  !primaryAction ||
                  (state === "notInstalled" &&
                    (planLoading ||
                      !!planError ||
                      plan?.blocked ||
                      systemLoading ||
                      !!systemError))
                }
                busy={!primaryAction}
              >
                {primaryLabel}
              </ActionButton>
              <ActionMenu items={menuItems} />
            </div>
          )}
        {state === "running" && (
          <Button
            variant="danger"
            icon="stop"
            onClick={() =>
              setConfirmation({
                title: t("home.stopGame"),
                message: t("home.stopConfirm"),
                label: t("home.stopGame"),
                run: async () => {
                  try {
                    await invoke("stop_game");
                  } catch (e) {
                    setLocalError({ title: t("errors.stopFailed"), error: e });
                  }
                },
              })
            }
          >
            {t("home.stopGame")}…
          </Button>
        )}
        {["needsProton", "protonError"].includes(state) && (
          <button
            className="home-page__subaction"
            onClick={() => onOpenSettings("proton")}
          >
            {t("ui.selectProton")}
            <Icon name="arrow" size={14} />
          </button>
        )}
      </section>
      <footer className="home-page__footer">
        <span>
          {stats?.totalPlaytimeSecs > 0 && (
            <>
              {t("ui.stats", {
                time: formatPlaytime(stats.totalPlaytimeSecs, locale),
              })}
              {stats.lastPlayed > 0 && (
                <>
                  {" "}
                  ·{" "}
                  {t("ui.lastPlayed", {
                    date: formatDate(stats.lastPlayed, locale),
                  })}
                </>
              )}
            </>
          )}
        </span>
      </footer>
      {confirmation && (
        <ConfirmDialog
          title={confirmation.title}
          message={confirmation.message}
          confirmLabel={confirmation.label}
          danger
          onCancel={() => setConfirmation(null)}
          onConfirm={() => {
            const run = confirmation.run;
            setConfirmation(null);
            run();
          }}
        />
      )}
    </main>
  );
}
