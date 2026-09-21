import { useState, useCallback, useEffect } from "react";
import TitleBar from "./components/layout/TitleBar";
import MainLayout from "./components/layout/MainLayout";
import HomePage from "./components/home/HomePage";
import SocialSidebar from "./components/home/SocialSidebar";
import SettingsModal from "./components/settings/SettingsModal";
import LaunchFailedDialog from "./components/home/LaunchFailedDialog";
import { I18nProvider } from "./i18n";

import { useSettingsStore } from "./stores/settingsStore";
import { useSystemStore } from "./stores/systemStore";
import { useGameStore } from "./stores/gameStore";
import { useTaskStore } from "./stores/taskStore";

export default function App() {
  const settings = useSettingsStore((s) => s.settings);
  const reloadSettings = useSettingsStore((s) => s.reload);
  
  const content = useGameStore((s) => s.content);
  const refreshContent = useGameStore((s) => s.refreshContent);
  const launchFailure = useGameStore((s) => s.launchFailure);
  const dismissFailure = useGameStore((s) => s.dismissFailure);
  const running = useGameStore((s) => s.running);
  const initGameListeners = useGameStore((s) => s.initListeners);
  const refreshGameState = useGameStore((s) => s.refreshGameState);

  const refreshSystem = useSystemStore((s) => s.refresh);
  const initTaskListeners = useTaskStore((s) => s.initListeners);

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsTab, setSettingsTab] = useState("general");

  const sync = useCallback(() => {
    return Promise.all([reloadSettings(), refreshSystem(), refreshGameState()]);
  }, [reloadSettings, refreshSystem, refreshGameState]);

  useEffect(() => {
    reloadSettings();
    refreshSystem();
    refreshGameState();
  }, [reloadSettings, refreshSystem, refreshGameState]);

  useEffect(() => {
    refreshContent(settings?.language);
  }, [settings?.language, refreshContent]);

  useEffect(() => {
    let unlistenGame: () => void;
    let unlistenTasks: () => void;
    
    initGameListeners().then(u => unlistenGame = u);
    initTaskListeners(sync).then(u => unlistenTasks = u);
    
    return () => {
      if (unlistenGame) unlistenGame();
      if (unlistenTasks) unlistenTasks();
    };
  }, [initGameListeners, initTaskListeners, sync]);

  const openSettings = (tab = "general") => {
    setSettingsTab(tab);
    setSettingsOpen(true);
  };

  return (
    <I18nProvider language={settings?.language}>
      <MainLayout background={content?.background} paused={running}>
        <SocialSidebar sidebars={content?.sidebars} />
        <TitleBar onOpenSettings={() => openSettings()} settingsOpen={settingsOpen} />
        <HomePage
          hidden={settingsOpen}
          onOpenSettings={openSettings}
        />
        {settingsOpen && (
          <SettingsModal
            initialTab={settingsTab}
            onClose={() => setSettingsOpen(false)}
          />
        )}
        {launchFailure && (
          <LaunchFailedDialog
            failure={launchFailure}
            onClose={dismissFailure}
            onOpenProtonSettings={() => {
              dismissFailure();
              openSettings("proton");
            }}
          />
        )}
      </MainLayout>
    </I18nProvider>
  );
}
