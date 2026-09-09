import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "../../../i18n";
import { Button, Status } from "../../common/Controls";
import ErrorNotice from "../../common/ErrorNotice";
import { copyText } from "../../../utils/clipboard";
import { formatSize } from "../../../utils/format";
export default function DiagnosticsSettings({
  systemCheck,
  onRefresh,
  onShowLog,
  confirm,
  busy,
}) {
  const { t } = useTranslation();
  const [error, setError] = useState(null),
    [message, setMessage] = useState(""),
    [working, setWorking] = useState(false),
    [prefix, setPrefix] = useState(null);
  const linux = systemCheck?.platform !== "windows";
  useEffect(() => {
    if (linux) invoke("get_prefix_info").then(setPrefix).catch(setError);
  }, [linux]);
  const run = async (command, args = {}, done) => {
    setWorking(true);
    setError(null);
    setMessage("");
    try {
      const result = await invoke(command, args);
      if (done) setMessage(done(result));
      if (linux) setPrefix(await invoke("get_prefix_info"));
    } catch (e) {
      setError(e);
    } finally {
      setWorking(false);
    }
  };
  const backup = async () => {
    try {
      const dest = await save({
        defaultPath: `endfield-prefix-${new Date().toISOString().slice(0, 10)}.tar.gz`,
        filters: [{ name: "Prefix backup", extensions: ["tar.gz", "gz"] }],
      });
      if (dest)
        run("backup_prefix", { dest }, () =>
          t("settings.prefixTools.backupDone"),
        );
    } catch (e) {
      setError(e);
    }
  };
  const restore = async () => {
    try {
      const archive = await open({
        filters: [{ name: "Prefix backup", extensions: ["gz"] }],
      });
      if (archive)
        confirm({
          title: t("settings.prefixTools.restore"),
          message: t("settings.prefixTools.restoreConfirm"),
          label: t("settings.prefixTools.restore"),
          danger: true,
          run: () =>
            run("restore_prefix", { archive }, () =>
              t("settings.prefixTools.restoreDone"),
            ),
        });
    } catch (e) {
      setError(e);
    }
  };
  return (
    <>
      <div className="settings-section-heading">
        <h3>{t("ui.systemStatus")}</h3>
        <Button variant="ghost" icon="refresh" onClick={onRefresh}>
          {t("ui.refreshCheck")}
        </Button>
      </div>
      <div className="diagnostic-checks">
        {(linux
          ? [
              ["Proton", "has_proton"],
              ["ntsync", "has_ntsync"],
              ["GameMode", "has_gamemode"],
              ["MangoHud", "has_mangohud"],
              ["Gamescope", "has_gamescope"],
            ]
          : [["Windows", null]]
        ).map(([label, key]) => (
          <div key={label}>
            <span>{label}</span>
            <Status kind={!key || systemCheck?.[key] ? "success" : "neutral"}>
              {t(!key || systemCheck?.[key] ? "ui.installed" : "ui.notFound")}
            </Status>
          </div>
        ))}
      </div>
      <div className="setting-row">
        <span>{t("logViewer.title")}</span>
        <Button icon="file" onClick={onShowLog}>
          {t("settings.viewLog.button")}
        </Button>
      </div>
      <div className="setting-row">
        <span>{t("settings.debugInfo.name")}</span>
        <Button
          icon="copy"
          onClick={async () => {
            setError(null);
            try {
              const info = await invoke("get_debug_info");
              if (!(await copyText(info)))
                throw new Error(t("settings.debugInfo.fallback"));
              setMessage(t("ui.copied"));
            } catch (e) {
              setError(e);
            }
          }}
        >
          {t("ui.copy")}
        </Button>
      </div>
      {message && <Status kind="success">{message}</Status>}
      <ErrorNotice title={t("ui.diagnostics")} error={error} />
      {linux && (
        <details className="ui-details">
          <summary>{t("ui.prefixTools")}</summary>
          <div className="ui-details__body">
            <small className="selectable">
              {prefix?.path || t("settings.prefixTools.noPrefix")}
            </small>
            <div className="settings-tool-grid">
              <Button
                disabled={working || !prefix?.exists}
                onClick={() => run("open_prefix_folder")}
              >
                {t("settings.prefixTools.open")}
              </Button>
              <Button
                disabled={busy || working}
                onClick={() => run("run_prefix_tool", { tool: "winecfg" })}
              >
                {t("settings.prefixTools.winecfg")}
              </Button>
              <Button
                disabled={busy || working}
                onClick={() =>
                  run("clear_shader_cache", {}, (r) =>
                    t("settings.prefixTools.cacheDone", {
                      files: r.files_removed,
                      size: formatSize(r.bytes_freed),
                    }),
                  )
                }
              >
                {t("settings.prefixTools.clearCache")}
              </Button>
              <Button
                disabled={busy || working || !prefix?.exists}
                onClick={backup}
              >
                {t("settings.prefixTools.backup")}
              </Button>
              <Button disabled={busy || working} onClick={restore}>
                {t("settings.prefixTools.restore")}…
              </Button>
              <Button
                variant="danger"
                disabled={busy || working || !prefix?.exists}
                onClick={() =>
                  confirm({
                    title: t("settings.prefixTools.reset"),
                    message: t("settings.prefixTools.resetConfirm"),
                    label: t("settings.prefixTools.reset"),
                    danger: true,
                    run: () =>
                      run("reset_prefix", {}, () =>
                        t("settings.prefixTools.resetDone"),
                      ),
                  })
                }
              >
                {t("settings.prefixTools.reset")}…
              </Button>
            </div>
          </div>
        </details>
      )}
    </>
  );
}
