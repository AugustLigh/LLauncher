import { useState, useCallback } from "react";
import TitleBar from "./components/layout/TitleBar";
import MainLayout from "./components/layout/MainLayout";
import HomePage from "./components/home/HomePage";
import SocialSidebar from "./components/home/SocialSidebar";
import SettingsModal from "./components/settings/SettingsModal";
import LaunchFailedDialog from "./components/home/LaunchFailedDialog";
import useLauncherContent from "./hooks/useLauncherContent";
import useSettings from "./hooks/useSettings";
import useSystemCheck from "./hooks/useSystemCheck";
import useGameState from "./hooks/useGameState";
import useLaunchEvents from "./hooks/useLaunchEvents";
import useGameRunning from "./hooks/useGameRunning";
import { TaskProvider } from "./hooks/useTasks";
import { I18nProvider } from "./i18n";
export default function App() {
  const { settings, reload: reloadSettings, saveSettings } = useSettings();
  const content = useLauncherContent(settings?.language);
  const { running: gameRunning, markRunning } = useGameRunning();
  const system = useSystemCheck();
  const game = useGameState();
  const { failure, dismiss: dismissFailure } = useLaunchEvents();
  const [settingsOpen, setSettingsOpen] = useState(false),
    [settingsTab, setSettingsTab] = useState("general");
  const sync = useCallback(
    () => Promise.all([reloadSettings(), system.refresh(), game.refresh()]),
    [reloadSettings, system.refresh, game.refresh],
  );
  const save = useCallback(
    async (next) => {
      await saveSettings(next);
      await Promise.all([system.refresh(), game.refresh()]);
    },
    [saveSettings, system.refresh, game.refresh],
  );
  const openSettings = (tab = "general") => {
    setSettingsTab(tab);
    setSettingsOpen(true);
  };
  return (
    <I18nProvider language={settings?.language}>
      <TaskProvider onComplete={sync}>
        <MainLayout
          background={content.content?.background}
          paused={gameRunning}
        >
          <SocialSidebar sidebars={content.content?.sidebars} />
          <TitleBar
            onOpenSettings={() => openSettings()}
            settingsOpen={settingsOpen}
          />
          <HomePage
            hidden={settingsOpen}
            content={content.content}
            contentLoading={content.loading}
            contentError={content.error}
            onRetryContent={content.refresh}
            settings={settings}
            systemCheck={system.systemCheck}
            systemLoading={system.loading}
            systemError={system.error}
            onRetrySystem={system.refresh}
            gameState={game.gameState}
            gameLoading={game.loading}
            gameError={game.error}
            onRetryGameState={game.refresh}
            gameRunning={gameRunning}
            markRunning={markRunning}
            onSync={sync}
            onSaveSettings={save}
            onOpenSettings={openSettings}
          />
          {settingsOpen && (
            <SettingsModal
              settings={settings}
              initialTab={settingsTab}
              systemCheck={system.systemCheck}
              onRefreshSystemCheck={system.refresh}
              onSync={sync}
              onSave={save}
              onClose={() => setSettingsOpen(false)}
              gameRunning={gameRunning}
            />
          )}
          {failure && (
            <LaunchFailedDialog
              failure={failure}
              onClose={dismissFailure}
              onOpenProtonSettings={() => {
                dismissFailure();
                openSettings("proton");
              }}
            />
          )}
        </MainLayout>
      </TaskProvider>
    </I18nProvider>
  );
}
