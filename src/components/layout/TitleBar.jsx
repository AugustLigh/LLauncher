import { getCurrentWindow } from '@tauri-apps/api/window';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useTranslation } from '../../i18n';
import useLauncherUpdate from '../../hooks/useLauncherUpdate';
import './TitleBar.css';

const appWindow = getCurrentWindow();

export default function TitleBar() {
  const { t } = useTranslation();
  const update = useLauncherUpdate();
  return (
    <div className="titlebar" data-tauri-drag-region>
      <span className="titlebar__title">LLauncher</span>
      <div className="titlebar__controls">
        {update && (
          <button
            className="titlebar__update"
            onClick={() => openUrl(update.url)}
            title={t('titlebar.updateTooltip')}
          >
            {t('titlebar.update', { version: update.version })}
          </button>
        )}
        <button
          className="titlebar__btn"
          onClick={() => appWindow.minimize()}
          title={t('titlebar.minimize')}
        >
          <svg viewBox="0 0 16 16">
            <rect x="3" y="7.5" width="10" height="1" />
          </svg>
        </button>
        <button
          className="titlebar__btn titlebar__btn--close"
          onClick={() => appWindow.hide()}
          title={t('titlebar.close')}
        >
          <svg viewBox="0 0 16 16">
            <path d="M4.5 3.5l7 7m0-7l-7 7" stroke="currentColor" strokeWidth="1.2" fill="none" />
          </svg>
        </button>
      </div>
    </div>
  );
}
