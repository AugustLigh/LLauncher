import { commands } from '../../bindings';
import { useState, useEffect } from 'react';
import { useTaskStore } from '../../stores/taskStore';
import useModalDismiss from '../../hooks/useModalDismiss';
import { formatSize, formatSpeed, formatPercent } from '../../utils/format';
import { useTranslation } from '../../i18n';
import './ProtonPrompt.css';

export interface ProtonPromptProps {
  onClose: () => void;
  onConfigureManually?: () => void;
  onDownloadComplete?: () => void;
}

export default function ProtonPrompt({ onClose, onConfigureManually }: ProtonPromptProps) {
  const { t } = useTranslation();
  
  const start = useTaskStore(s => s.start);
  const stop = useTaskStore(s => s.stop);
  const task = useTaskStore(s => s.tasks.proton);
  const downloading = task && task.status === 'running';
  const progress: any = task?.progress;
  const error = task?.error;
  
  const [recommendedTag, setRecommendedTag] = useState('');

  useEffect(() => {
    commands.recommendedProtonTag().then(tag => {
      if (tag) setRecommendedTag(tag);
    }).catch(() => {});
  }, []);

  const handleDownload = () => start("proton");

  const handleClose = () => {
    if (downloading) stop("proton", true);
    onClose();
  };
  useModalDismiss(handleClose);

  const bytesDownloaded = progress ? Number(progress.bytes_downloaded) || 0 : 0;
  const bytesTotal = progress ? Number(progress.bytes_total) || 0 : 0;
  const speedBps = progress ? Number(progress.speed_bps) || 0 : 0;

  return (
    <div className="proton-prompt-overlay" onClick={handleClose}>
      <div className="proton-prompt" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
        <div className="proton-prompt__header">
          <span className="proton-prompt__title">{t('protonPrompt.title')}</span>
          <button className="proton-prompt__close" onClick={handleClose}>
            {'✕'}
          </button>
        </div>

        <div className="proton-prompt__body">
          {!downloading && !error && (
            <>
              <p className="proton-prompt__text">{t('protonPrompt.body')}</p>
              {recommendedTag && (
                <p className="proton-prompt__text proton-prompt__text--muted">
                  {t('protonPrompt.recommended', { tag: recommendedTag })}
                </p>
              )}
              <div className="proton-prompt__actions">
                <button
                  className="proton-prompt__btn proton-prompt__btn--primary"
                  onClick={handleDownload}
                >
                  {t('protonPrompt.download')}
                </button>
                <button
                  className="proton-prompt__btn proton-prompt__btn--secondary"
                  onClick={onConfigureManually}
                >
                  {t('protonPrompt.configure')}
                </button>
              </div>
            </>
          )}

          {downloading && (
            <div className="proton-prompt__progress">
              <div className="proton-prompt__progress-info">
                <span>
                  {progress?.stage === 'extracting'
                    ? t('protonPrompt.extracting')
                    : t('protonPrompt.downloading')}
                </span>
                <span>
                  {progress ? formatPercent(bytesDownloaded, bytesTotal) : ''}
                  {speedBps > 0 && ` • ${formatSpeed(speedBps)}`}
                </span>
              </div>
              <div className="proton-prompt__progress-bar">
                <div
                  className="proton-prompt__progress-fill"
                  style={{
                    width: bytesTotal > 0
                      ? `${(bytesDownloaded / bytesTotal) * 100}%`
                      : '0%',
                  }}
                />
              </div>
              <div className="proton-prompt__progress-detail">
                {progress
                  ? `${formatSize(bytesDownloaded)} / ${formatSize(bytesTotal)}`
                  : ''}
              </div>
              <button
                className="proton-prompt__btn proton-prompt__btn--secondary"
                onClick={() => stop("proton", true)}
              >
                {t('common.cancel')}
              </button>
            </div>
          )}

          {error && (
            <div className="proton-prompt__error">
              <p className="proton-prompt__error-text">{error}</p>
              <div className="proton-prompt__actions">
                <button
                  className="proton-prompt__btn proton-prompt__btn--primary"
                  onClick={handleDownload}
                >
                  {t('common.retry')}
                </button>
                <button
                  className="proton-prompt__btn proton-prompt__btn--secondary"
                  onClick={onClose}
                >
                  {t('common.close')}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
