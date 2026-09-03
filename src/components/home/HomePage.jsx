import { useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import SystemWarning from '../common/SystemWarning';
import ActionButton from './ActionButton';
import ProgressBar from './ProgressBar';
import GameStatus from './GameStatus';
import NewsPanel from './NewsPanel';
import SingleEntCard from './SingleEntCard';
import SocialSidebar from './SocialSidebar';
import ProtonPrompt from './ProtonPrompt';
import ConfirmDialog from '../common/ConfirmDialog';
import useDownload from '../../hooks/useDownload';
import useGameRunning from '../../hooks/useGameRunning';
import useGameStats from '../../hooks/useGameStats';
import { useTranslation } from '../../i18n';
import { notify } from '../../utils/notify';
import './HomePage.css';

// `onSync` re-reads settings, the system check and the game state from the
// backend; call it after anything that changes backend state behind the
// frontend's back (import, finished install, Proton download).
export default function HomePage({
  content,
  settings,
  systemCheck,
  gameState,
  gameLoading,
  onSync,
  onOpenSettings,
}) {
  const { t } = useTranslation();
  const { running: gameRunning, markRunning } = useGameRunning();
  const stats = useGameStats();
  const [showProtonPrompt, setShowProtonPrompt] = useState(false);
  const [importError, setImportError] = useState(null);
  // Pre-spawn launch failures (missing exe, unreadable prefix, "already
  // running") come back as a rejected invoke, not as launch://failed, and
  // used to vanish into console.error — the button just did nothing.
  const [launchError, setLaunchError] = useState(null);
  const [confirmStop, setConfirmStop] = useState(false);

  const handleImport = async () => {
    setImportError(null);
    try {
      const dir = await open({ directory: true });
      if (!dir) return;
      await invoke('import_existing_game', { path: dir });
      onSync();
    } catch (e) {
      setImportError(typeof e === 'string' ? e : e.message || t('errors.importFailed'));
    }
  };

  const handleStopGame = async () => {
    setConfirmStop(false);
    try {
      await invoke('stop_game');
    } catch (e) {
      setLaunchError(`${t('errors.stopFailed')}: ${typeof e === 'string' ? e : e.message || e}`);
    }
  };

  const onDownloadComplete = useCallback(async (version) => {
    notify('LLauncher', t('notify.downloadComplete'));
    try {
      await invoke('update_installed_version', { version });
    } catch (e) {
      console.error('Failed to update version:', e);
    }
    // The backend already recorded the version itself; either way, re-read.
    onSync();
  }, [onSync, t]);

  const { downloading, progress, error: dlError, startDownload, startUpdate, pauseDownload, cancelDownload } =
    useDownload(onDownloadComplete);

  useEffect(() => {
    if (dlError) notify('LLauncher', t('notify.downloadError', { message: dlError }));
  }, [dlError, t]);

  // A tray / --play launch refused because the game is out of date: show why
  // and re-read the state so the main button flips to "Update".
  const onSyncRef = useRef(onSync);
  onSyncRef.current = onSync;
  const tRef = useRef(t);
  tRef.current = t;
  useEffect(() => {
    const pending = listen('launch://update-required', (event) => {
      const { installed_version, latest_version } = event.payload || {};
      setLaunchError(tRef.current('home.updateRequired', { installed: installed_version, latest: latest_version }));
      onSyncRef.current();
    });
    return () => {
      pending.then((u) => u());
    };
  }, []);

  const handleAction = async (withMods = false) => {
    if (!gameState) return;
    switch (gameState.status) {
      case 'not_installed':
        startDownload();
        break;
      case 'update_available':
        // Smart update: backend downloads only changed files when safe,
        // otherwise the full packs.
        startUpdate();
        break;
      case 'ready':
        if (systemCheck && !systemCheck.has_proton) {
          setShowProtonPrompt(true);
          return;
        }
        setLaunchError(null);
        try {
          await invoke('launch_game', { withMods });
          markRunning();
          const action = settings?.on_launch_action || 'hide';
          // "close" also just hides: the backend keeps the window alive so
          // the game watcher (playtime, exit handling, Flatpak wineserver
          // clean-up) survives, and the tray is where quitting happens.
          if (action === 'hide' || action === 'close') getCurrentWindow().hide();
        } catch (e) {
          const message = typeof e === 'string' ? e : e.message || t('errors.launchFailed');
          // The update-required case arrives as a translated message through
          // launch://update-required; don't overwrite it with the raw error.
          if (/update required/i.test(message)) {
            onSync();
            break;
          }
          setLaunchError(message);
        }
        break;
    }
  };

  // The backend has switched `proton_dir` to the fresh build; pull the new
  // settings and system check so "Play" no longer trips the prompt.
  const handleProtonDownloadComplete = useCallback(() => {
    setShowProtonPrompt(false);
    onSync();
  }, [onSync]);

  return (
    <div className="home-page">
      <div className="home-page__main">
        {content?.single_ent && (
          <SingleEntCard singleEnt={content.single_ent} />
        )}
        {content?.news_tabs?.length > 0 && (
          <div className="home-page__news">
            <NewsPanel tabs={content.news_tabs} />
          </div>
        )}
        <div className="home-page__warnings">
          {systemCheck && !systemCheck.has_proton && (
            <SystemWarning message={t('home.warning.noProton')} type="warn" />
          )}
          {systemCheck && !systemCheck.has_ntsync && (
            <SystemWarning message={t('home.warning.noNtsync')} type="warn" />
          )}
        </div>
      </div>

      <SocialSidebar sidebars={content?.sidebars} />

      <div className="home-page__bottom">
        <div className="home-page__bottom-left">
          <GameStatus gameState={gameState} stats={stats} />
          <button
            className="home-page__settings-btn"
            onClick={onOpenSettings}
            title={t('home.settingsTooltip')}
          >
            {'⚙'}
          </button>
        </div>

        <div className="home-page__action-area">
          {downloading && progress && (
            <ProgressBar progress={progress} onPause={pauseDownload} onCancel={cancelDownload} />
          )}
          {dlError && (
            <div className="home-page__error">
              <span className="home-page__error-text">{dlError}</span>
            </div>
          )}
          {importError && (
            <div className="home-page__error">
              <span className="home-page__error-text">{importError}</span>
              <button className="home-page__error-dismiss" onClick={() => setImportError(null)} title={t('common.dismiss')}>{'✕'}</button>
            </div>
          )}
          {launchError && (
            <div className="home-page__error">
              <span className="home-page__error-text">{launchError}</span>
              <button className="home-page__error-dismiss" onClick={() => setLaunchError(null)} title={t('common.dismiss')}>{'✕'}</button>
            </div>
          )}
          <ActionButton
            gameState={gameState}
            downloading={downloading}
            extracting={progress?.stage === 'extracting'}
            verifying={progress?.stage === 'verifying'}
            running={gameRunning}
            onAction={() => handleAction(false)}
            disabled={gameLoading}
          />
          {settings?.mods_enabled && !downloading && !gameRunning && gameState?.status === 'ready' && (
            <button className="home-page__mods-btn" onClick={() => handleAction(true)} disabled={gameLoading}>
              {t('home.action.launchMods')}
            </button>
          )}
          {gameRunning && (
            <button className="home-page__stop-btn" onClick={() => setConfirmStop(true)}>
              {t('home.stopGame')}
            </button>
          )}
          {!downloading && !gameRunning && gameState?.status === 'not_installed' && (
            <button className="home-page__import-link" onClick={handleImport}>
              {t('home.importLink')}
            </button>
          )}
        </div>
      </div>

      {confirmStop && (
        <ConfirmDialog
          title={t('home.stopGame')}
          message={t('home.stopConfirm')}
          confirmLabel={t('home.stopGame')}
          danger
          onConfirm={handleStopGame}
          onCancel={() => setConfirmStop(false)}
        />
      )}

      {showProtonPrompt && (
        <ProtonPrompt
          onClose={() => setShowProtonPrompt(false)}
          onConfigureManually={() => {
            setShowProtonPrompt(false);
            onOpenSettings();
          }}
          onDownloadComplete={handleProtonDownloadComplete}
        />
      )}
    </div>
  );
}
