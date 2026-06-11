import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import SystemWarning from '../common/SystemWarning';
import ActionButton from './ActionButton';
import ProgressBar from './ProgressBar';
import GameStatus from './GameStatus';
import SingleEntCard from './SingleEntCard';
import SocialSidebar from './SocialSidebar';
import ProtonPrompt from './ProtonPrompt';
import useGameState from '../../hooks/useGameState';
import useDownload from '../../hooks/useDownload';
import useGameRunning from '../../hooks/useGameRunning';
import { useTranslation } from '../../i18n';
import './HomePage.css';

export default function HomePage({ content, settings, systemCheck, onOpenSettings }) {
  const { t } = useTranslation();
  const { gameState, loading: gameLoading, refresh } = useGameState();
  const { running: gameRunning, markRunning } = useGameRunning();
  const [showProtonPrompt, setShowProtonPrompt] = useState(false);

  const onDownloadComplete = useCallback(async (version) => {
    try {
      await invoke('update_installed_version', { version });
      refresh();
    } catch (e) {
      console.error('Failed to update version:', e);
    }
  }, [refresh]);

  const { downloading, progress, error: dlError, startDownload, cancelDownload } =
    useDownload(onDownloadComplete);

  const handleAction = async () => {
    if (!gameState) return;
    switch (gameState.status) {
      case 'not_installed':
      case 'update_available':
        startDownload();
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
          <GameStatus gameState={gameState} />
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
            <ProgressBar progress={progress} onCancel={cancelDownload} />
          )}
          {dlError && <div className="home-page__error">{dlError}</div>}
          <ActionButton
            gameState={gameState}
            downloading={downloading}
            extracting={progress?.stage === 'extracting'}
            verifying={progress?.stage === 'verifying'}
            running={gameRunning}
            onAction={handleAction}
            disabled={gameLoading}
          />
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
