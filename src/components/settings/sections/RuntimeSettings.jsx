import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../../../i18n";
import { Button, Status } from "../../common/Controls";
import ErrorNotice from "../../common/ErrorNotice";
import PathSelector from "../PathSelector";
import ProgressBar from "../../home/ProgressBar";
import useTasks, { taskActive } from "../../../hooks/useTasks";
import { formatSize } from "../../../utils/format";
export default function RuntimeSettings({
  form,
  onChange,
  systemCheck,
  initialOpen,
  busy,
  activeProton,
  // Which setting holds the active build (`proton_dir` on Linux,
  // `macos_wine_dir` on macOS) and whether to call it Wine rather than Proton.
  field = "proton_dir",
  mac = false,
}) {
  const { t } = useTranslation();
  const runtimeName = mac ? "Wine" : "Proton";
  const [open, setOpen] = useState(initialOpen),
    [releases, setReleases] = useState([]),
    [installed, setInstalled] = useState([]),
    [recommended, setRecommended] = useState(""),
    [loading, setLoading] = useState(false),
    [error, setError] = useState(null);
  const { tasks, start } = useTasks();
  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [r, i, tag] = await Promise.all([
        invoke("list_dwproton_releases"),
        invoke("list_installed_protons"),
        invoke("recommended_proton_tag"),
      ]);
      setReleases(r || []);
      setInstalled(i || []);
      setRecommended(tag);
    } catch (e) {
      setError(e);
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    if (open) load();
  }, [open, load, tasks.proton?.status]);
  const pending = form[field] !== activeProton;
  const task = tasks.proton;
  const selected = installed.find((p) => p.path === form[field]);
  const recommendedRelease = releases.find((r) => r.tag_name === recommended);
  const row = (release) => {
    const existing = installed.find(
      // DWProton unpacks to `<tag>-x86_64`, the macOS Wine builds are
      // installed under `wine-staging-<tag>`.
      (p) =>
        p.name === release.tag_name ||
        p.name.startsWith(release.tag_name + "-") ||
        p.name.endsWith("-" + release.tag_name),
    );
    return (
      <div className="runtime-row" key={release.tag_name}>
        <div>
          <strong>{release.tag_name}</strong>
          <small>
            {release.tag_name === recommended
              ? t("ui.recommended")
              : release.published_at}
            {release.size > 0 && ` · ${formatSize(release.size)}`}
          </small>
        </div>
        {existing ? (
          <Button
            disabled={busy || form[field] === existing.path}
            onClick={() => onChange(field, existing.path)}
          >
            {t(
              form[field] === existing.path
                ? "ui.selected"
                : "settings.use",
            )}
          </Button>
        ) : (
          <Button
            disabled={busy}
            icon="download"
            onClick={() => start("proton", { release })}
          >
            {t("settings.download")}
          </Button>
        )}
      </div>
    );
  };
  return (
    <section className="runtime">
      <div className="runtime-summary">
        <div>
          <Status
            kind={
              pending
                ? "neutral"
                : systemCheck?.has_proton
                  ? "success"
                  : "warning"
            }
          >
            {t(
              pending
                ? "ui.selected"
                : systemCheck?.has_proton
                  ? "ui.runtimeReady"
                  : "ui.notFound",
            )}
          </Status>
          <small title={form[field]}>
            {selected?.name ||
              form[field]?.split(/[\\/]/).filter(Boolean).pop() ||
              runtimeName}
          </small>
        </div>
        <Button onClick={() => setOpen(!open)} aria-expanded={!!open}>
          {t(open ? "ui.lessNews" : "ui.manage")}
        </Button>
      </div>
      {taskActive(task) && <ProgressBar progress={task.progress} proton />}
      {task?.error && (
        <ErrorNotice
          title={t("errors.protonDownloadFailed")}
          error={task.error}
        />
      )}
      {open && (
        <div className="runtime-body">
          {loading && !releases.length && <small>{t("common.loading")}</small>}
          {error && (
            <ErrorNotice
              title={t("ui.protonReleasesFailed")}
              error={error}
              onRetry={load}
            />
          )}
          {installed.map((p) => (
            <div className="runtime-row" key={p.path}>
              <div>
                <strong>{p.name}</strong>
                <small>
                  {t(
                    form[field] === p.path ? "ui.selected" : "ui.installed",
                  )}
                  {mac && p.wine_patch && ` · ${p.wine_patch}`}
                  {mac && p.dxmt && ` · DXMT ${p.dxmt}`}
                  {mac && !p.wine_patch && ` · ${t("ui.wineUnpatched")}`}
                </small>
              </div>
              <Button
                disabled={busy || form[field] === p.path}
                onClick={() => onChange(field, p.path)}
              >
                {t("settings.use")}
              </Button>
            </div>
          ))}
          {recommendedRelease && row(recommendedRelease)}
          <details className="ui-details">
            <summary>{t("ui.otherVersions")}</summary>
            {releases.filter((r) => r.tag_name !== recommended).map(row)}
            <Button
              variant="ghost"
              icon="refresh"
              onClick={load}
              disabled={loading}
            >
              {t("common.refresh")}
            </Button>
          </details>
          <details className="ui-details">
            <summary>{t("ui.manualPaths")}</summary>
            <PathSelector
              label={t(mac ? "settings.wineDir" : "settings.activeProton")}
              value={form[field]}
              onChange={(v) => onChange(field, v)}
              disabled={busy}
            />
            <PathSelector
              label={t("settings.prefixDir")}
              value={form.proton_prefix_dir}
              onChange={(v) => onChange("proton_prefix_dir", v)}
              disabled={busy}
            />
          </details>
        </div>
      )}
    </section>
  );
}
