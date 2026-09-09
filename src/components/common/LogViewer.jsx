import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from '../../i18n';
import { copyText } from '../../utils/clipboard';
import useModalDismiss from '../../hooks/useModalDismiss';
import './LogViewer.css';

export default function LogViewer({ initialContent, onClose }) {
  const { t } = useTranslation();
  const [content, setContent] = useState(initialContent ?? null);
  const [loading, setLoading] = useState(initialContent == null);
  const [error, setError] = useState(null);
  const [copyStatus, setCopyStatus] = useState(null);
  useModalDismiss(onClose);

  const loadLog = useCallback(async () => {
    setLoading(true);
    setError(null);
    setCopyStatus(null);
    try {
      const text = await invoke('read_launch_log');
      setContent(text);
    } catch (e) {
      console.error('Failed to read log:', e);
      setError(typeof e === 'string' ? e : e?.message || String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (initialContent == null) loadLog();
  }, [initialContent, loadLog]);

  const handleCopy = async () => {
    const copied = await copyText(content || '');
    setCopyStatus(copied ? 'copied' : 'copyFailed');
  };

  const trimmed = (content || '').trim();

  return (
    <div className="log-viewer-overlay" onClick={onClose}>
      <div className="log-viewer" role="dialog" aria-modal="true" aria-labelledby="log-viewer-title" onClick={(e) => e.stopPropagation()}>
        <div className="log-viewer__header">
          <span className="log-viewer__title" id="log-viewer-title">{t('logViewer.title')}</span>
          <div className="log-viewer__header-actions">
            <button className="log-viewer__refresh" onClick={handleCopy} disabled={loading || !trimmed}>
              {t('logViewer.copy')}
            </button>
            <button
              className="log-viewer__refresh"
              onClick={loadLog}
              disabled={loading}
              title={t('logViewer.refresh')}
            >
              {t('logViewer.refresh')}
            </button>
            <button className="log-viewer__close" onClick={onClose} aria-label={t('common.close')}>{'✕'}</button>
          </div>
        </div>
        <div className="log-viewer__body">
          {error && (
            <div className="log-viewer__error selectable" role="alert">
              {t('logViewer.readFailed')}: {error}
            </div>
          )}
          {copyStatus && (
            <div className={copyStatus === 'copyFailed' ? 'log-viewer__error' : 'log-viewer__status'} role="status">
              {t(`logViewer.${copyStatus}`)}
            </div>
          )}
          {loading ? (
            <div className="log-viewer__empty">{t('common.loading')}</div>
          ) : trimmed ? (
            <pre className="log-viewer__pre">{content}</pre>
          ) : !error ? (
            <div className="log-viewer__empty">{t('logViewer.empty')}</div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
