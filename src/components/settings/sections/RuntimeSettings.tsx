import { commands, InstalledProton, ProtonReleaseInfo, SystemCheck } from '../../../bindings';
import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "../../../i18n";
import { useTaskStore, taskActive } from "../../../stores/taskStore";
import { Button, Status } from "../../common/Controls";
import ErrorNotice from "../../common/ErrorNotice";
import PathSelector from "../PathSelector";
import ProgressBar from "../../home/ProgressBar";
import { formatSize } from "../../../utils/format";

export interface RuntimeSettingsProps {
  form: Record<string, any>;
  onChange: (key: string, value: any) => void;
  systemCheck?: SystemCheck | null;
  initialOpen?: boolean;
  busy?: boolean;
  activeProton?: string;
  field?: string;
  mac?: boolean;
}

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
}: RuntimeSettingsProps) {
  const { t } = useTranslation();
  const runtimeName = mac ? "Wine" : "Proton";
  const [open, setOpen] = useState(Boolean(initialOpen)),
    [releases, setReleases] = useState<ProtonReleaseInfo[]>([]),
    [installed, setInstalled] = useState<InstalledProton[]>([]),
    [recommended, setRecommended] = useState(""),
    [loading, setLoading] = useState(false),
    [error, setError] = useState<any>(null);
  const { tasks, start } = useTaskStore();
  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [r, i, tag] = await Promise.all([
        commands.listDwprotonReleases(),
        commands.listInstalledProtons(),
        commands.recommendedProtonTag(),
      ]);
      if (r.status === "error") throw new Error(r.error);
      if (i.status === "error") throw new Error(i.error);
      setReleases(r.data);
      setInstalled(i.data);
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
  const row = (release: ProtonReleaseInfo) => {
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
            {release.size && release.size > 0 && ` · ${formatSize(release.size)}`}
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
                  : "ui.runtimeNeeded",
              { runtime: runtimeName },
            )}
          </Status>
          <span className="runtime-summary__name">
            {selected?.name ||
              (form[field]
                ? form[field].split(/[/\\]/).filter(Boolean).pop()
                : null) ||
              t("ui.noRuntimeSelected", { runtime: runtimeName })}
          </span>
          {pending && (
            <small className="runtime-summary__note">
              {t("ui.runtimePending", { runtime: runtimeName })}
            </small>
          )}
        </div>
        <Button
          variant="secondary"
          onClick={() => {
            setOpen((v: boolean) => !v);
            if (!open) load();
          }}
        >
          {t(open ? "ui.hidePicker" : "ui.chooseRuntime", {
            runtime: runtimeName,
          })}
        </Button>
      </div>
      {taskActive(task) && (
        <ProgressBar progress={task?.progress} proton paused={false} />
      )}
      {task?.status === "completed" && (
        <Status kind="success">
          {t("settings.runtimeDownloaded", { runtime: runtimeName })}
        </Status>
      )}
      {open && (
        <div className="runtime-picker">
          <div className="runtime-picker__heading">
            <h4>{t("ui.availableBuilds")}</h4>
            <Button
              variant="ghost"
              icon="refresh"
              onClick={load}
              disabled={loading}
            >
              {t("common.refresh")}
            </Button>
          </div>
          {loading && !releases.length && (
            <Status busy>{t("common.loading")}</Status>
          )}
          {error && (
            <ErrorNotice
              title={t("ui.runtimeListFailed", { runtime: runtimeName })}
              error={error}
              onRetry={load}
            />
          )}
          {recommendedRelease && row(recommendedRelease)}
          {releases
            .filter((r) => r.tag_name !== recommended)
            .map((r) => row(r))}
          {installed
            .filter(
              (p) =>
                !releases.some(
                  (r) =>
                    p.name === r.tag_name ||
                    p.name.startsWith(r.tag_name + "-"),
                ),
            )
            .map((p) => (
              <div className="runtime-row" key={p.path}>
                <div>
                  <strong>{p.name}</strong>
                  <small className="selectable">{p.path}</small>
                </div>
                <Button
                  disabled={busy || form[field] === p.path}
                  onClick={() => onChange(field, p.path)}
                >
                  {t(
                    form[field] === p.path
                      ? "ui.selected"
                      : "settings.use",
                  )}
                </Button>
              </div>
            ))}
          <details className="ui-details">
            <summary>{t("ui.customRuntime", { runtime: runtimeName })}</summary>
            <div className="ui-details__body">
              <PathSelector
                label={t("ui.runtimeDirectory", { runtime: runtimeName })}
                value={form[field]}
                onChange={(v) => onChange(field, v)}
                disabled={busy}
              />
            </div>
          </details>
        </div>
      )}
    </section>
  );
}
