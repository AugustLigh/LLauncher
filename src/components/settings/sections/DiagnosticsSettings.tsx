import { commands, PrefixInfo, SystemCheck } from '../../../bindings';
import { useState, useEffect } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "../../../i18n";
import { Button, Status } from "../../common/Controls";
import ErrorNotice from "../../common/ErrorNotice";
import { copyText } from "../../../utils/clipboard";
import { formatSize } from "../../../utils/format";

export interface DiagnosticsSettingsProps {
  systemCheck?: SystemCheck | null;
  onRefresh?: () => void;
  onShowLog?: () => void;
  confirm: (dialog: { title: string; message: string; label: string; danger?: boolean; run: () => void }) => void;
  busy?: boolean;
}

export default function DiagnosticsSettings({
  systemCheck,
  onRefresh,
  onShowLog,
  confirm,
  busy,
}: DiagnosticsSettingsProps) {
  const { t } = useTranslation();
  const [error, setError] = useState<any>(null),
    [message, setMessage] = useState(""),
    [working, setWorking] = useState(false),
    [prefix, setPrefix] = useState<PrefixInfo | null>(null);
  // "linux" here means "has a Wine prefix": macOS does too.
  const platform = systemCheck?.platform || "linux";
  const linux = platform !== "windows";
  const checks: [string, keyof SystemCheck | null][] =
    platform === "macos"
      ? [
          ["Wine", "has_proton"],
          ["Rosetta 2", "has_rosetta"],
          ["Endfield modules", "wine_patch_version"],
          ["DXMT", "dxmt_version"],
        ]
      : linux
        ? [
            ["Proton", "has_proton"],
            ["ntsync", "has_ntsync"],
            ["GameMode", "has_gamemode"],
            ["MangoHud", "has_mangohud"],
            ["Gamescope", "has_gamescope"],
          ]
        : [["Windows", null]];
  useEffect(() => {
    if (linux) {
      commands
        .getPrefixInfo()
        .then((res) => {
          if (res.status === "ok") {
            setPrefix(res.data);
          } else {
            setError(res.error);
          }
        })
        .catch(setError);
    }
  }, [linux]);
  const run = async (action: () => Promise<any>, done?: (result: any) => string) => {
    setWorking(true);
    setError(null);
    setMessage("");
    try {
      const res = await action();
      if (res && typeof res === "object" && "status" in res) {
        if (res.status === "error") throw new Error(String(res.error));
        if (done) setMessage(done(res.data));
      } else {
        if (done) setMessage(done(res));
      }
      if (linux) {
        const pRes = await commands.getPrefixInfo();
        if (pRes.status === "ok") {
          setPrefix(pRes.data);
        } else {
          setError(pRes.error);
        }
      }
    } catch (e: any) {
      setError(e.message || String(e));
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
        run(() => commands.backupPrefix(dest), () =>
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
      if (archive && typeof archive === "string")
        confirm({
          title: t("settings.prefixTools.restore"),
          message: t("settings.prefixTools.restoreConfirm"),
          label: t("settings.prefixTools.restore"),
          danger: true,
          run: () =>
            run(() => commands.restorePrefix(archive), () =>
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
        {checks.map(([label, key]) => (
          <div key={label}>
            <span>{label}</span>
            <Status kind={!key || (key && systemCheck?.[key]) ? "success" : "neutral"}>
              {t(!key || (key && systemCheck?.[key]) ? "ui.installed" : "ui.notFound")}
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
              const res = await commands.getDebugInfo();
              if (res.status === "error") throw new Error(res.error);
              if (!(await copyText(res.data)))
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
                onClick={() => run(() => commands.openPrefixFolder())}
              >
                {t("settings.prefixTools.open")}
              </Button>
              <Button
                disabled={busy || working}
                onClick={() => run(() => commands.runPrefixTool("winecfg"))}
              >
                {t("settings.prefixTools.winecfg")}
              </Button>
              <Button
                disabled={busy || working}
                onClick={() =>
                  run(() => commands.clearShaderCache(), (r) =>
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
                      run(() => commands.resetPrefix(), () =>
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
