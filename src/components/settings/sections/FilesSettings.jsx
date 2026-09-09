import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../../../i18n";
import { Button, Field, Status } from "../../common/Controls";
import PathSelector from "../PathSelector";
import ProgressBar from "../../home/ProgressBar";
import ErrorNotice from "../../common/ErrorNotice";
import useTasks, { taskActive } from "../../../hooks/useTasks";
export default function FilesSettings({
  form,
  onChange,
  confirm,
  onSync,
  busy,
  pathsChanged,
}) {
  const { t } = useTranslation();
  const { tasks, start, stop } = useTasks();
  const [error, setError] = useState(null),
    [working, setWorking] = useState(false);
  const task = tasks.game;
  const action = async (command) => {
    setWorking(true);
    setError(null);
    try {
      await invoke(command);
      await onSync();
    } catch (e) {
      setError(e);
    } finally {
      setWorking(false);
    }
  };
  const check = async () => {
    setError(null);
    try {
      const latest = await invoke("get_game_version");
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
            value={form.download_speed_limit > 0 ? "limited" : "unlimited"}
            onChange={(e) =>
              onChange(
                "download_speed_limit",
                e.target.value === "unlimited" ? 0 : 10 * 1024 * 1024,
              )
            }
          >
            <option value="unlimited">{t("ui.unlimited")}</option>
            <option value="limited">{t("ui.limit")}</option>
          </select>
          {form.download_speed_limit > 0 && (
            <>
              <input
                type="number"
                min="0.01"
                step="0.01"
                aria-label={t("settings.speedLimit")}
                value={+(form.download_speed_limit / 1024 / 1024).toFixed(2)}
                onChange={(e) =>
                  onChange(
                    "download_speed_limit",
                    Math.max(
                      1,
                      Math.round(
                        (Number(e.target.value) || 0.01) * 1024 * 1024,
                      ),
                    ),
                  )
                }
              />
              <small>MB/s</small>
            </>
          )}
        </div>
      </div>
      <div className="setting-row">
        <span>{t("settings.integrity.name")}</span>
        <Button
          icon="refresh"
          onClick={check}
          disabled={busy || pathsChanged || working || !form.installed_version}
        >
          {t("settings.integrity.button")}…
        </Button>
      </div>
      {(taskActive(task) || task?.status === "paused") && (
        <div className="settings-transfer">
          <ProgressBar
            progress={task.progress}
            paused={task.status === "paused"}
            stopping={["pausing", "cancelling"].includes(task.status)}
            onResume={
              task.status === "paused" && !pathsChanged
                ? () => start(task.kind)
                : null
            }
            onPause={
              taskActive(task) && task.progress?.stage !== "extracting"
                ? () => stop("game").catch(setError)
                : null
            }
          />
        </div>
      )}
      {task?.kind === "integrity" && task.status === "completed" && (
        <Status kind="success">
          {t(
            task.result?.repaired > 0
              ? "settings.integrity.resultRepaired"
              : "settings.integrity.resultOk",
            {
              checked: task.result?.checked || 0,
              repaired: task.result?.repaired || 0,
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
                  run: () => action("uninstall_game"),
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
