import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
import useGameState from '../../hooks/useGameState';
import useDownload from '../../hooks/useDownload';
import useGameRunning from '../../hooks/useGameRunning';
import useGameStats from '../../hooks/useGameStats';
import { useTranslation } from '../../i18n';
import { notify } from '../../utils/notify';
import './HomePage.css';

export default function HomePage({ content, settings, systemCheck, onOpenSettings }) {
  const { t } = useTranslation();
  const { gameState, loading: gameLoading, refresh } = useGameState();
  const { running: gameRunning, markRunning } = useGameRunning();
  const stats = useGameStats();
  const [showProtonPrompt, setShowProtonPrompt] = useState(false);
  const [importError, setImportError] = useState(null);

  const handleImport = async () => {
    setImportError(null);
    try {
      const dir = await open({ directory: true });
      if (!dir) return;
      await invoke('import_existing_game', { path: dir });
      refresh();
    } catch (e) {
      setImportError(typeof e === 'string' ? e : e.message || 'Import failed');
    }
  };

  const handleStopGame = async () => {
    if (!confirm(t('home.stopConfirm'))) return;
    try {
      await invoke('stop_game');
    } catch (e) {
      console.error('Failed to stop game:', e);
    }
  };

  const onDownloadComplete = useCallback(async (version) => {
    notify('LLauncher', t('notify.downloadComplete'));
    try {
      await invoke('update_installed_version', { version });
      refresh();
    } catch (e) {
      console.error('Failed to update version:', e);
    }
  }, [refresh, t]);

  const { downloading, progress, error: dlError, startDownload, startUpdate, pauseDownload, cancelDownload } =
    useDownload(onDownloadComplete);

  useEffect(() => {
    if (dlError) notify('LLauncher', t('notify.downloadError', { message: dlError }));
  }, [dlError, t]);

  const handleAction = async () => {
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
        try {
          await invoke('launch_game');
          markRunning();
          const action = settings?.on_launch_action || 'hide';
          if (action === 'hide') getCurrentWindow().hide();
          else if (action === 'close') getCurrentWindow().close();
        } catch (e) {
          console.error('Failed to launch game:', e);
        }
        break;
    }
  };

  const handleProtonDownloadComplete = useCallback(() => {
    setShowProtonPrompt(false);
  }, []);

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
          {dlError && <div className="home-page__error">{dlError}</div>}
          {importError && <div className="home-page__error">{importError}</div>}
          <ActionButton
            gameState={gameState}
            downloading={downloading}
            extracting={progress?.stage === 'extracting'}
            verifying={progress?.stage === 'verifying'}
            running={gameRunning}
            onAction={handleAction}
            disabled={gameLoading}
          />
          {gameRunning && (
            <button className="home-page__stop-btn" onClick={handleStopGame}>
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
