import { formatSize, formatSpeed, formatEta } from "../../utils/format";
import { progressDetails } from "../../utils/progress";
import { useTranslation } from "../../i18n";
import { Button } from "../common/Controls";
import "./ProgressBar.css";
export default function ProgressBar({
  progress,
  paused = false,
  proton = false,
  onPause,
  onResume,
  onCancel,
  onScreenOff,
  stopping = false,
}) {
  const { t, locale } = useTranslation();
  const p = progress || { stage: "fetching" };
  const { percent, done, total, byFiles } = progressDetails(p);
  const extracting = p.stage === "extracting",
    fetching = p.stage === "fetching",
    preparing = extracting || p.stage === "verifying";
  const determinate =
    !fetching && (extracting ? Number.isFinite(p.percent) : total > 0);
  const stage = fetching
    ? "fetching"
    : extracting
      ? "extracting"
      : p.stage === "verifying"
        ? "verifying"
        : proton
          ? "protonDownloading"
          : "downloading";
  const eta =
    !paused && !preparing && !fetching
      ? formatEta(total - done, p.speed_bps, locale)
      : "";
  return (
    <div className={`progress-bar ${paused ? "progress-bar--paused" : ""}`}>
      <div className="progress-bar__heading">
        {t(paused ? "ui.paused" : `ui.${stage}`)}
        {determinate && <span>{percent}%</span>}
      </div>
      {!proton && (
        <div className="progress-bar__steps" aria-hidden="true">
          <span className={!preparing ? "active" : ""}>
            01 {t("ui.downloadStage")}
          </span>
          <span className={preparing ? "active" : ""}>
            02 {t("ui.prepareStage")}
          </span>
          <span>03 {t("ui.readyStage")}</span>
        </div>
      )}
      <div
        className={`progress-bar__track ${determinate ? "" : "progress-bar__track--indeterminate"}`}
        role="progressbar"
        aria-label={t(`ui.${stage}`)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={determinate ? percent : undefined}
      >
        <div
          className="progress-bar__fill"
          style={determinate ? { width: `${percent}%` } : undefined}
        />
      </div>
      <div className="progress-bar__info">
        <span>
          {byFiles
            ? t("progress.filesChecked", { current: done, total })
            : determinate && !extracting
              ? `${formatSize(done)} / ${formatSize(total)}`
              : ""}
        </span>
        <span>
          {!paused && !preparing && !fetching && p.speed_bps > 0
            ? formatSpeed(p.speed_bps)
            : ""}
        </span>
      </div>
      {paused && <small>{t("ui.retained")}</small>}
      {eta && (
        <div className="progress-bar__eta">
          {t("ui.remaining", { time: eta })}
        </div>
      )}
      {(onPause || onResume || onCancel || onScreenOff) && (
        <div className="progress-bar__actions">
          {onPause && (
            <Button icon="pause" onClick={onPause} disabled={stopping}>
              {t(stopping ? "ui.pausing" : "progress.pause")}
            </Button>
          )}
          {onResume && (
            <Button variant="primary" icon="play" onClick={onResume}>
              {t("ui.resume")}
            </Button>
          )}
          {onCancel && (
            <Button variant="ghost" onClick={onCancel} disabled={stopping}>
              {t("common.cancel")}…
            </Button>
          )}
          {onScreenOff && (
            <Button
              variant="ghost"
              icon="monitor"
              onClick={onScreenOff}
              title={t("progress.screenOffHint")}
            >
              {t("progress.screenOff")}
            </Button>
          )}
        </div>
      )}
      {p.file_name && (
        <details className="progress-bar__details">
          <summary>{t("ui.details")}</summary>
          <span className="selectable">{p.file_name}</span>
        </details>
      )}
    </div>
  );
}
