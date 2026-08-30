import { useState } from 'react';
import LogViewer from '../common/LogViewer';
import { useTranslation } from '../../i18n';
import './LaunchFailedDialog.css';

// Known-failure signature ids from the backend (game::diagnose) mapped to
// their translated advice strings.
const HINT_TEXT_KEYS = {
  'dwproton11-ntoskrnl': 'launchFailed.hintDwproton11',
};

export default function LaunchFailedDialog({ failure, onClose, onOpenProtonSettings }) {
  const { t } = useTranslation();
  const [showFullLog, setShowFullLog] = useState(false);

  if (!failure) return null;

  const exitText = failure.exit_code != null
    ? t('launchFailed.exitCode', { code: failure.exit_code })
    : t('launchFailed.exitCodeUnknown');

  const tail = (failure.log_tail || '').trim();
  const hintKey = HINT_TEXT_KEYS[failure.hint];

  return (
    <>
      <div className="launch-failed-overlay" onClick={onClose}>
        <div className="launch-failed" onClick={(e) => e.stopPropagation()}>
          <div className="launch-failed__header">
            <span className="launch-failed__title">{t('launchFailed.title')}</span>
            <button className="launch-failed__close" onClick={onClose}>{'✕'}</button>
          </div>
          <div className="launch-failed__body">
            <div className="launch-failed__exit">{exitText}</div>
            {hintKey && (
              <div className="launch-failed__hint">
                <div className="launch-failed__hint-text">{t(hintKey)}</div>
                {onOpenProtonSettings && (
                  <button
                    className="launch-failed__btn launch-failed__btn--primary"
                    onClick={onOpenProtonSettings}
                  >
                    {t('launchFailed.openProtonSettings')}
                  </button>
                )}
              </div>
            )}
            {tail ? (
              <pre className="launch-failed__tail">{tail}</pre>
            ) : (
              <div className="launch-failed__empty">{t('launchFailed.noLog')}</div>
            )}
            <div className="launch-failed__actions">
              <button
                className="launch-failed__btn launch-failed__btn--secondary"
                onClick={() => setShowFullLog(true)}
              >
                {t('launchFailed.viewLog')}
              </button>
              <button
                className="launch-failed__btn launch-failed__btn--primary"
                onClick={onClose}
              >
                {t('common.close')}
              </button>
            </div>
          </div>
        </div>
      </div>
      {showFullLog && <LogViewer onClose={() => setShowFullLog(false)} />}
    </>
  );
}
