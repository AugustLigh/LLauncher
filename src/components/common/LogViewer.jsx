import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from '../../i18n';
import useModalDismiss from '../../hooks/useModalDismiss';
import './LogViewer.css';

export default function LogViewer({ initialContent, onClose }) {
  const { t } = useTranslation();
  const [content, setContent] = useState(initialContent ?? null);
  const [loading, setLoading] = useState(initialContent == null);
  useModalDismiss(onClose);

  const loadLog = useCallback(async () => {
    setLoading(true);
    try {
      const text = await invoke('read_launch_log');
      setContent(text);
    } catch (e) {
      console.error('Failed to read log:', e);
      setContent('');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (initialContent == null) loadLog();
  }, [initialContent, loadLog]);

  const trimmed = (content || '').trim();

  return (
    <div className="log-viewer-overlay" onClick={onClose}>
      <div className="log-viewer" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
        <div className="log-viewer__header">
          <span className="log-viewer__title">{t('logViewer.title')}</span>
          <div className="log-viewer__header-actions">
            <button
              className="log-viewer__refresh"
              onClick={loadLog}
              disabled={loading}
              title={t('logViewer.refresh')}
            >
              {t('logViewer.refresh')}
            </button>
            <button className="log-viewer__close" onClick={onClose}>{'✕'}</button>
          </div>
        </div>
        <div className="log-viewer__body">
          {loading ? (
            <div className="log-viewer__empty">{t('common.loading')}</div>
          ) : trimmed ? (
            <pre className="log-viewer__pre">{content}</pre>
          ) : (
            <div className="log-viewer__empty">{t('logViewer.empty')}</div>
          )}
        </div>
      </div>
    </div>
  );
}
