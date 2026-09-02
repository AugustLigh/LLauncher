import { useState, useCallback } from 'react';
import TitleBar from './components/layout/TitleBar';
import MainLayout from './components/layout/MainLayout';
import HomePage from './components/home/HomePage';
import SettingsModal from './components/settings/SettingsModal';
import LaunchFailedDialog from './components/home/LaunchFailedDialog';
import useLauncherContent from './hooks/useLauncherContent';
import useSettings from './hooks/useSettings';
import useSystemCheck from './hooks/useSystemCheck';
import useGameState from './hooks/useGameState';
import useLaunchEvents from './hooks/useLaunchEvents';
import { I18nProvider } from './i18n';

export default function App() {
  const { settings, reload: reloadSettings, saveSettings } = useSettings();
  const { content } = useLauncherContent();
  const { systemCheck, refresh: refreshSystemCheck } = useSystemCheck();
  const { gameState, loading: gameLoading, refresh: refreshGameState } = useGameState();
  const { failure, dismiss: dismissFailure } = useLaunchEvents();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsTab, setSettingsTab] = useState('paths');

  // Bring every backend-derived view back in line after something changed the
  // backend's state: new paths, a finished install/import, a Proton download.
  // Game-state detection picks up an existing install in the game folder, so
  // pointing the launcher at one in Settings is enough — no import step.
  const syncWithBackend = useCallback(() => {
    reloadSettings();
    refreshSystemCheck();
    refreshGameState();
  }, [reloadSettings, refreshSystemCheck, refreshGameState]);

  const handleSaveSettings = useCallback(async (next) => {
    await saveSettings(next);
    refreshSystemCheck();
    refreshGameState();
  }, [saveSettings, refreshSystemCheck, refreshGameState]);

  const openSettings = (tab = 'paths') => {
    setSettingsTab(tab);
    setSettingsOpen(true);
  };

  return (
    <I18nProvider language={settings?.language}>
      <MainLayout background={content?.background}>
        <TitleBar />
        <HomePage
          content={content}
          settings={settings}
          systemCheck={systemCheck}
          gameState={gameState}
          gameLoading={gameLoading}
          onSync={syncWithBackend}
          onOpenSettings={() => openSettings()}
        />
        {settingsOpen && (
          <SettingsModal
            settings={settings}
            initialTab={settingsTab}
            systemCheck={systemCheck}
            onRefreshSystemCheck={refreshSystemCheck}
            onSync={syncWithBackend}
            onSave={handleSaveSettings}
            onClose={() => setSettingsOpen(false)}
          />
        )}
        {failure && (
          <LaunchFailedDialog
            failure={failure}
            onClose={dismissFailure}
            onOpenProtonSettings={() => {
              dismissFailure();
              openSettings('proton');
            }}
          />
        )}
      </MainLayout>
    </I18nProvider>
  );
}
