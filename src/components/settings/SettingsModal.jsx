import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import { useTranslation } from "../../i18n";
import useTasks, { taskActive } from "../../hooks/useTasks";
import useModalDismiss from "../../hooks/useModalDismiss";
import { Button, Status } from "../common/Controls";
import Icon from "../common/Icon";
import ErrorNotice from "../common/ErrorNotice";
import ConfirmDialog from "../common/ConfirmDialog";
import LogViewer from "../common/LogViewer";
import GeneralSettings from "./sections/GeneralSettings";
import FilesSettings from "./sections/FilesSettings";
import LaunchSettings from "./sections/LaunchSettings";
import DiagnosticsSettings from "./sections/DiagnosticsSettings";
import ModsSettings from "./ModsSettings";
import "./SettingsModal.css";
export default function SettingsModal({
  settings,
  initialTab,
  systemCheck,
  onRefreshSystemCheck,
  onSync,
  onSave,
  onClose,
  gameRunning,
}) {
  const { t } = useTranslation(),
    { busy: transferring, tasks } = useTasks();
  const busy = transferring || gameRunning;
  const tabMap = {
    paths: "files",
    downloads: "files",
    game: "files",
    proton: "launch",
  };
  const [tab, setTab] = useState(tabMap[initialTab] || initialTab || "general"),
    [form, setForm] = useState(settings),
    [baseline, setBaseline] = useState(settings),
    [saving, setSaving] = useState(false),
    [error, setError] = useState(null),
    [loadError, setLoadError] = useState(null),
    [confirmation, setConfirmation] = useState(null),
    [showLog, setShowLog] = useState(false),
    [autostart, setAutostart] = useState(null),
    [baseAuto, setBaseAuto] = useState(null),
    [autoError, setAutoError] = useState(null);
  const gamePathChanged =
    !!form &&
    !!settings &&
    (form.game_dir !== settings.game_dir ||
      form.download_dir !== settings.download_dir);
  const prefixPathChanged =
    !!form &&
    !!settings &&
    (form.proton_dir !== settings.proton_dir ||
      form.proton_prefix_dir !== settings.proton_prefix_dir);
  const edited = useRef(false),
    formRef = useRef(form),
    baseRef = useRef(baseline);
  formRef.current = form;
  baseRef.current = baseline;
  const dirty =
    !!form &&
    !!baseline &&
    (JSON.stringify(form) !== JSON.stringify(baseline) ||
      autostart !== baseAuto);
  const load = async () => {
    setLoadError(null);
    try {
      const fresh = await invoke("get_settings");
      if (!edited.current) {
        setForm(fresh);
        setBaseline(fresh);
      }
    } catch (e) {
      setLoadError(e);
    }
  };
  useEffect(() => {
    let active = true;
    invoke("get_settings")
      .then((fresh) => {
        if (active && !edited.current) {
          setForm(fresh);
          setBaseline(fresh);
        }
      })
      .catch((e) => {
        if (active) setLoadError(e);
      });
    isEnabled()
      .then((value) => {
        if (active) {
          setAutostart(value);
          setBaseAuto(value);
        }
      })
      .catch((e) => {
        if (active) setAutoError(String(e));
      });
    return () => {
      active = false;
    };
  }, []);
  // Preserve edits while taking in fields changed by completed backend commands.
  useEffect(() => {
    if (!settings || !formRef.current || !baseRef.current) return;
    const next = { ...formRef.current };
    for (const key of Object.keys(settings))
      if (next[key] === baseRef.current[key]) next[key] = settings[key];
    setForm(next);
    setBaseline(settings);
  }, [settings]);
  const onChange = (key, value) => {
    edited.current = true;
    setForm((prev) => ({ ...prev, [key]: value }));
  };
  const requestClose = () => {
    if (saving) return;
    if (dirty)
      setConfirmation({
        title: t("settings.unsavedTitle"),
        message: t("settings.unsavedBody"),
        label: t("settings.unsavedDiscard"),
        danger: true,
        run: onClose,
      });
    else onClose();
  };
  useModalDismiss(requestClose, !confirmation && !showLog);
  const handleSave = async () => {
    if (saving || !dirty) return;
    setSaving(true);
    setError(null);
    let autoChanged = false;
    try {
      const fresh = await invoke("get_settings");
      const next = { ...fresh };
      for (const key of Object.keys(form))
        if (form[key] !== baseline[key]) next[key] = form[key];
      if (autostart !== baseAuto && autostart != null) {
        await (autostart ? enable() : disable());
        autoChanged = true;
      }
      await onSave(next);
      const saved = await invoke("get_settings");
      setForm(saved);
      setBaseline(saved);
      setBaseAuto(autostart);
      edited.current = false;
    } catch (e) {
      if (autoChanged) {
        try {
          await (baseAuto ? enable() : disable());
        } catch (rollback) {
          setAutoError(String(rollback));
          const actual = await isEnabled().catch(() => null);
          setAutostart(actual);
          setBaseAuto(actual);
        }
      }
      setError(e);
    } finally {
      setSaving(false);
    }
  };
  const tabs = [
    ["general", "settings"],
    ["files", "folder"],
    ["launch", "play"],
    ["mods", "mods"],
    ["diagnostics", "file"],
  ];
  return (
    <section className="settings-page" aria-label={t("settings.title")}>
      <nav className="settings-nav" aria-label={t("settings.title")}>
        <h2>{t("settings.title")}</h2>
        {tabs.map(([id, icon]) => (
          <button
            key={id}
            onClick={() => setTab(id)}
            aria-current={tab === id ? "page" : undefined}
          >
            <Icon name={icon} size={17} />
            {t(`ui.${id}`)}
          </button>
        ))}
        <button className="settings-nav__back" onClick={requestClose}>
          <Icon name="back" size={17} />
          {t("ui.back")}
        </button>
      </nav>
      <div className="settings-main">
        <div className="settings-content" key={tab}>
          <h1>{t(`ui.${tab}`)}</h1>
          <ErrorNotice
            title={t("ui.settingsLoadFailed")}
            error={loadError}
            onRetry={load}
          />
          {form && (
            <fieldset disabled={saving} className="settings-fields">
              {(gamePathChanged || prefixPathChanged) && tab !== "general" && (
                <Status kind="warning">{t("ui.savePathsFirst")}</Status>
              )}
              {tab === "general" && (
                <GeneralSettings
                  form={form}
                  onChange={onChange}
                  autostart={autostart}
                  onAutostart={setAutostart}
                  autoError={autoError}
                />
              )}
              {tab === "files" && (
                <FilesSettings
                  pathsChanged={gamePathChanged}
                  form={form}
                  onChange={onChange}
                  confirm={setConfirmation}
                  onSync={onSync}
                  busy={busy}
                />
              )}
              {tab === "launch" && (
                <LaunchSettings
                  activeProton={settings?.proton_dir}
                  form={form}
                  onChange={onChange}
                  systemCheck={systemCheck}
                  initialRuntime={initialTab === "proton"}
                  busy={busy}
                  onSync={onSync}
                />
              )}
              {tab === "mods" && (
                <ModsSettings
                  key={settings?.game_dir}
                  form={form}
                  onChange={onChange}
                  systemCheck={systemCheck}
                  disabled={busy || gamePathChanged}
                />
              )}
              {tab === "diagnostics" && (
                <DiagnosticsSettings
                  systemCheck={systemCheck}
                  onRefresh={onRefreshSystemCheck}
                  onShowLog={() => setShowLog(true)}
                  confirm={setConfirmation}
                  busy={busy || gamePathChanged || prefixPathChanged}
                />
              )}
            </fieldset>
          )}
        </div>
        <footer className="settings-footer">
          <div>
            <span role="status">
              {t(
                saving ? "settings.saving" : dirty ? "ui.unsaved" : "ui.saved",
              )}
            </span>
            {transferring && (
              <button className="settings-footer__task" onClick={requestClose}>
                <Icon name="download" size={14} />
                {t(
                  taskActive(tasks.proton)
                    ? "ui.protonDownloading"
                    : "ui.downloadTask",
                )}
              </button>
            )}
            {error && (
              <ErrorNotice title={t("errors.saveFailed")} error={error} />
            )}
          </div>
          <Button
            variant="primary"
            onClick={handleSave}
            disabled={saving || !dirty || !form}
          >
            {t(saving ? "settings.saving" : "common.save")}
          </Button>
        </footer>
      </div>
      {showLog && <LogViewer onClose={() => setShowLog(false)} />}
      {confirmation && (
        <ConfirmDialog
          title={confirmation.title}
          message={confirmation.message}
          confirmLabel={confirmation.label}
          danger={confirmation.danger}
          onCancel={() => setConfirmation(null)}
          onConfirm={() => {
            const run = confirmation.run;
            setConfirmation(null);
            run();
          }}
        />
      )}
    </section>
  );
}
