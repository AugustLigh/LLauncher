import { formatSize, formatSpeed, formatEta } from '../../utils/format';
import { useTranslation } from '../../i18n';
import './ProgressBar.css';

export default function ProgressBar({ progress, onCancel }) {
  const { t } = useTranslation();
  if (!progress) return null;

  const extracting = progress.stage === 'extracting';
  const verifying = progress.stage === 'verifying';

  const percent = extracting
    ? progress.percent ?? 0
    : progress.bytes_total > 0
      ? Math.round((progress.bytes_downloaded / progress.bytes_total) * 100)
      : 0;

  const done = extracting ? progress.bytes_processed : progress.bytes_downloaded;
  const total = progress.bytes_total;
  const eta = formatEta(total - done, progress.speed_bps);

  const stageClass = extracting
    ? 'progress-bar--extracting'
    : verifying
      ? 'progress-bar--verifying'
      : '';

  return (
    <div className={`progress-bar ${stageClass}`}>
      <div className="progress-bar__track">
        <div
          className="progress-bar__fill"
          style={{ width: `${percent}%` }}
        />
      </div>
      <div className="progress-bar__info">
        <span>
          {formatSize(done)} / {formatSize(total)}
          {' — '}{percent}%
        </span>
        <span>
          {formatSpeed(progress.speed_bps)}
          {eta ? ` — ${eta}` : ''}
        </span>
      </div>
      <div className="progress-bar__file">
        {extracting ? (
          <span className="progress-bar__stage">{t('progress.extractLabel')}</span>
        ) : (
          <>
            <span className="progress-bar__file-name">
              {verifying
                ? t('progress.verifyLabel', {
                    current: progress.file_index + 1,
                    total: progress.total_files,
                    name: progress.file_name,
                  })
                : t('progress.fileLabel', {
                    current: progress.file_index + 1,
                    total: progress.total_files,
                    name: progress.file_name,
                  })}
            </span>
            <button className="progress-bar__cancel" onClick={onCancel}>
              {t('common.cancel')}
            </button>
          </>
        )}
      </div>
    </div>
  );
}
