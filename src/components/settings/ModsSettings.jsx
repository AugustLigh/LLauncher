import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "../../i18n";
import { Button, Switch, Status } from "../common/Controls";
import ErrorNotice from "../common/ErrorNotice";
import "./ModsSettings.css";
export default function ModsSettings({
  form,
  onChange,
  systemCheck,
  disabled,
}) {
  const { t } = useTranslation();
  const [status, setStatus] = useState(null),
    [busy, setBusy] = useState(null),
    [error, setError] = useState(null),
    [message, setMessage] = useState("");
  // vkBasalt is a Vulkan layer, so it exists on Linux only — not on Windows,
  // and not on macOS, where the game reaches Metal through DXMT.
  const linux = systemCheck?.platform === "linux";
  const refresh = useCallback(async () => {
    setError(null);
    try {
      setStatus(await invoke("get_mods_status"));
    } catch (e) {
      setError(e);
    }
  }, []);
  useEffect(() => {
    refresh();
  }, [refresh, form.installed_version]);
  const run = async (key, command, done) => {
    setBusy(key);
    setError(null);
    setMessage("");
    try {
      const r = await invoke(command);
      if (done) setMessage(done(r));
      await refresh();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(null);
    }
  };
  const link = async (url) => {
    try {
      await openUrl(url);
    } catch (e) {
      setError(e);
    }
  };
  const ready = status?.loader_installed && status?.loader_configured;
  const legacy = ready && !status?.efmi;
  if (!status)
    return error ? (
      <ErrorNotice
        title={t("ui.modsStatusFailed")}
        error={error}
        onRetry={refresh}
      />
    ) : (
      <Status busy>{t("ui.checkingMods")}</Status>
    );
  if (status.game_dir_missing)
    return <Status>{t("settings.mods.noGame")}</Status>;
  return (
    <>
      <div className="mods-row">
        <div>
          <strong>EFMI</strong>
          <small>
            {t(
              legacy
                ? "settings.mods.skins.legacy"
                : ready
                  ? "ui.installed"
                  : "ui.notFound",
            )}
          </small>
        </div>
        <div className="mods-row__actions">
          {(!ready || legacy) && (
            <Button
              variant="primary"
              icon="download"
              disabled={disabled || !!busy}
              onClick={() =>
                run("install", "install_mod_loader", (r) =>
                  t("settings.mods.installed", { version: r.version }),
                )
              }
            >
              {busy === "install"
                ? t("settings.mods.skins.installing")
                : t(
                    legacy
                      ? "settings.mods.skins.upgrade"
                      : "settings.mods.skins.install",
                  )}
            </Button>
          )}
          {status.loader_installed && (
            <Button
              variant="ghost"
              disabled={disabled || !!busy}
              onClick={() =>
                run("remove", "uninstall_mod_loader", () =>
                  t("settings.mods.uninstalled"),
                )
              }
            >
              {t("settings.mods.skins.remove")}
            </Button>
          )}
        </div>
      </div>
      <div className="mods-row">
        <div>
          <strong>{t("settings.mods.skins.folder")}</strong>
          <small>
            {t("settings.mods.skins.count", { count: status.mod_count || 0 })}
          </small>
        </div>
        <div className="mods-row__actions">
          <Button icon="folder" onClick={() => run("open", "open_mods_folder")}>
            {t("settings.mods.skins.open")}
          </Button>
          <Button
            variant="ghost"
            icon="external"
            onClick={() => link("https://gamebanana.com/games/21842")}
          >
            {t("settings.mods.skins.catalog")}
          </Button>
        </div>
      </div>
      <Switch
        label={t("settings.mods.skins.button")}
        checked={form.mods_enabled}
        onChange={(v) => onChange("mods_enabled", v)}
      />
      <h3>{t("settings.mods.looks.title")}</h3>
      {linux &&
        (systemCheck?.has_vkbasalt ? (
          <Switch
            label="vkBasalt"
            checked={form.use_vkbasalt}
            onChange={(v) => onChange("use_vkbasalt", v)}
          />
        ) : (
          <div className="mods-row">
            <div>
              <strong>vkBasalt</strong>
              <small>{t("ui.notFound")}</small>
            </div>
            <Button
              icon="external"
              onClick={() => link("https://github.com/DadSchoorse/vkBasalt")}
            >
              {t("settings.mods.looks.howto")}
            </Button>
          </div>
        ))}
      <div className="mods-row">
        <div>
          <strong>ReShade / RenoDX</strong>
          <small>
            {t(
              status.reshade_installed
                ? "settings.mods.looks.reshadeFound"
                : "settings.mods.looks.reshadeHint",
            )}
          </small>
        </div>
      </div>
      {message && (
        <div className="mods-message">
          <Status kind="success">{message}</Status>
        </div>
      )}
      <ErrorNotice
        title={t("ui.modsStatusFailed")}
        error={error}
        onRetry={refresh}
      />
    </>
  );
}
