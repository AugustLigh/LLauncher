import { commands } from '../../../bindings';
import { useState } from "react";
import { useTranslation } from "../../../i18n";
import { useTaskStore, taskActive } from "../../../stores/taskStore";
import { Button, Field, Status, Switch } from "../../common/Controls";
import PathSelector from "../PathSelector";
import ProgressBar from "../../home/ProgressBar";
import ErrorNotice from "../../common/ErrorNotice";

export interface FilesSettingsProps {
  form: Record<string, any>;
  onChange: (key: string, value: any) => void;
  confirm: (dialog: { title: string; message: string; label: string; danger?: boolean; run: () => void }) => void;
  onSync: () => Promise<void>;
  busy?: boolean;
  pathsChanged?: boolean;
}

export default function FilesSettings({
  form,
  onChange,
  confirm,
  onSync,
  busy,
  pathsChanged,
}: FilesSettingsProps) {
  const { t } = useTranslation();
  const { tasks, start, stop } = useTaskStore();
  const [error, setError] = useState<any>(null),
    [working, setWorking] = useState(false);
  const task = tasks.game;
  const uninstall = async () => {
    setWorking(true);
    setError(null);
    try {
      const res = await commands.uninstallGame();
      if (res.status === 'error') throw new Error(res.error);
      await onSync();
    } catch (e: any) {
      setError(e);
    } finally {
      setWorking(false);
    }
  };
  const check = async () => {
    setError(null);
    try {
      const latestRes = await commands.getGameVersion();
      if (latestRes.status === "error") throw new Error(latestRes.error);
      const latest = latestRes.data;
      confirm({
        title: t("settings.integrity.name"),
        message: t("settings.integrity.confirm", {
          installed: form.installed_version || "—",
          latest: latest?.version || "—",
        }),
        label: t("settings.integrity.button"),
        run: () => start("integrity"),
      });
    } catch (e) {
      setError(e);
    }
  };
  return (
    <>
      <PathSelector
        label={t("settings.gameDir")}
        value={form.game_dir}
        onChange={(v) => onChange("game_dir", v)}
        disabled={busy}
      />
      <h3>{t("ui.downloadSettings")}</h3>
      <div className="setting-row">
        <span>{t("settings.speedLimit")}</span>
        <div className="settings-speed">
          <select
            aria-label={t("settings.speedLimit")}
            value={
              form.download_speed_limit > 0
                ? form.download_speed_limit >= 1048576
                  ? `${Math.round(form.download_speed_limit / 1048576)}M`
                  : `${Math.round(form.download_speed_limit / 1024)}K`
                : "none"
            }
            onChange={(e) => {
              const v = e.target.value;
              onChange(
                "download_speed_limit",
                v === "none"
                  ? 0
                  : v.endsWith("M")
                    ? parseInt(v, 10) * 1048576
                    : parseInt(v, 10) * 1024,
              );
            }}
          >
            <option value="none">{t("settings.unlimited")}</option>
            <option value="1M">1 MB/s</option>
            <option value="5M">5 MB/s</option>
            <option value="10M">10 MB/s</option>
            <option value="25M">25 MB/s</option>
            <option value="50M">50 MB/s</option>
          </select>
        </div>
      </div>
      <Switch
        label={t("settings.keepPacks.name")}
        note={t("settings.keepPacks.desc")}
        checked={form.keep_download_packs}
        onChange={(v) => onChange("keep_download_packs", v)}
      />
      <Switch
        label={t("settings.verifyAfter.name")}
        note={t("settings.verifyAfter.desc")}
        checked={form.verify_after_download}
        onChange={(v) => onChange("verify_after_download", v)}
      />
      <div className="setting-row">
        <div className="setting-row__label">
          <span>{t("settings.integrity.name")}</span>
          <small>{t("settings.integrity.desc")}</small>
        </div>
        <Button
          disabled={
            busy || pathsChanged || working || !form.installed_version
          }
          onClick={check}
        >
          {t("settings.integrity.button")}
        </Button>
      </div>
      {taskActive(task) && task?.kind === "integrity" && (
        <ProgressBar
          progress={task.progress}
          paused={task.status === "paused"}
          stopping={task.status === "pausing"}
          onResume={() => start("integrity")}
          onPause={() => stop("game")}
          onCancel={() => stop("game", true)}
        />
      )}
      {task?.status === "completed" && task.kind === "integrity" && (
        <Status
          kind={
            Number(task.result?.corrupted) > 0 || Number(task.result?.missing) > 0
              ? "warning"
              : "success"
          }
        >
          {t(
            Number(task.result?.corrupted) > 0 || Number(task.result?.missing) > 0
              ? "settings.integrity.issuesFound"
              : "settings.integrity.clean",
            {
              missing: Number(task.result?.missing) || 0,
              corrupted: Number(task.result?.corrupted) || 0,
              checked: Number(task.result?.files_checked) || 0,
            },
          )}
        </Status>
      )}
      {taskActive(task) && task?.kind === "repair" && (
        <ProgressBar
          progress={task.progress}
          paused={task.status === "paused"}
          stopping={task.status === "pausing"}
          onResume={() => start("repair")}
          onPause={() => stop("game")}
          onCancel={() => stop("game", true)}
        />
      )}
      {task?.status === "completed" && task.kind === "repair" && (
        <Status kind="success">
          {t(
            Number(task.result?.repaired) > 0
              ? "settings.repair.done"
              : "settings.repair.clean",
            {
              repaired: Number(task.result?.repaired) || 0,
            },
          )}
        </Status>
      )}
      <ErrorNotice
        title={t("errors.integrityFailed")}
        error={error || task?.error}
      />
      <details className="ui-details">
        <summary>{t("ui.advanced")}</summary>
        <div className="ui-details__body">
          <PathSelector
            label={t("settings.downloadDir")}
            value={form.download_dir}
            onChange={(v) => onChange("download_dir", v)}
            disabled={busy}
          />
          <Field label={t("settings.maxConcurrent")}>
            {(id) => (
              <select
                id={id}
                value={form.download_max_concurrent || 4}
                onChange={(e) =>
                  onChange("download_max_concurrent", Number(e.target.value))
                }
              >
                {Array.from({ length: 8 }, (_, i) => (
                  <option key={i} value={i + 1}>
                    {i + 1}
                  </option>
                ))}
              </select>
            )}
          </Field>
          <div className="setting-row">
            <span>{t("settings.repair.name")}</span>
            <Button
              disabled={busy || pathsChanged || working}
              onClick={() =>
                confirm({
                  title: t("settings.repair.name"),
                  message: t("settings.repair.confirm"),
                  label: t("settings.repair.button"),
                  run: () => start("repair"),
                })
              }
            >
              {t("settings.repair.button")}…
            </Button>
          </div>
          <div className="setting-row">
            <span>{t("settings.uninstall.name")}</span>
            <Button
              variant="danger"
              disabled={
                busy || pathsChanged || working || !form.installed_version
              }
              onClick={() =>
                confirm({
                  title: t("settings.uninstall.name"),
                  message: t("settings.uninstall.confirm"),
                  label: t("settings.uninstall.button"),
                  danger: true,
                  run: uninstall,
                })
              }
            >
              {t("settings.uninstall.button")}…
            </Button>
          </div>
        </div>
      </details>
    </>
  );
}
