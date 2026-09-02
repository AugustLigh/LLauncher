import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import useProtonDownload from '../../hooks/useProtonDownload';
import useModalDismiss from '../../hooks/useModalDismiss';
import { formatSize, formatSpeed, formatPercent } from '../../utils/format';
import { useTranslation } from '../../i18n';
import './ProtonPrompt.css';

export default function ProtonPrompt({ onClose, onConfigureManually, onDownloadComplete }) {
  const { t } = useTranslation();
  const { downloading, progress, error, startDownload, cancelDownload } =
    useProtonDownload(onDownloadComplete);
  const [recommendedTag, setRecommendedTag] = useState('');

  useEffect(() => {
    invoke('recommended_proton_tag').then(setRecommendedTag).catch(() => {});
  }, []);

  // No release argument: the backend picks the recommended build. Never pass
  // the click event through — the IPC layer cannot serialise it.
  const handleDownload = () => startDownload();

  // Closing the prompt mid-download used to leave a multi-hundred-MB
  // transfer running with no UI and no way to stop it. Cancel it instead.
  const handleClose = () => {
    if (downloading) cancelDownload();
    onClose();
  };
  useModalDismiss(handleClose);

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
                  {progress ? formatPercent(progress.bytes_downloaded, progress.bytes_total) : ''}
                  {progress?.speed_bps > 0 && ` • ${formatSpeed(progress.speed_bps)}`}
                </span>
              </div>
              <div className="proton-prompt__progress-bar">
                <div
                  className="proton-prompt__progress-fill"
                  style={{
                    width: progress?.bytes_total > 0
                      ? `${(progress.bytes_downloaded / progress.bytes_total) * 100}%`
                      : '0%',
                  }}
                />
              </div>
              <div className="proton-prompt__progress-detail">
                {progress
                  ? `${formatSize(progress.bytes_downloaded)} / ${formatSize(progress.bytes_total)}`
                  : ''}
              </div>
              <button
                className="proton-prompt__btn proton-prompt__btn--secondary"
                onClick={cancelDownload}
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
