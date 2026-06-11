import { useState } from 'react';
import TitleBar from './components/layout/TitleBar';
import MainLayout from './components/layout/MainLayout';
import HomePage from './components/home/HomePage';
import SettingsModal from './components/settings/SettingsModal';
import LaunchFailedDialog from './components/home/LaunchFailedDialog';
import useLauncherContent from './hooks/useLauncherContent';
import useSettings from './hooks/useSettings';
import useSystemCheck from './hooks/useSystemCheck';
import useLaunchEvents from './hooks/useLaunchEvents';
import { I18nProvider } from './i18n';

export default function App() {
  const { settings, saveSettings } = useSettings();
  const { content } = useLauncherContent();
  const { systemCheck, refresh: refreshSystemCheck } = useSystemCheck();
  const { failure, dismiss: dismissFailure } = useLaunchEvents();
  const [settingsOpen, setSettingsOpen] = useState(false);

  return (
    <I18nProvider language={settings?.language}>
      <MainLayout background={content?.background}>
        <TitleBar />
        <HomePage
          content={content}
          settings={settings}
          systemCheck={systemCheck}
          onOpenSettings={() => setSettingsOpen(true)}
        />
        {settingsOpen && (
          <SettingsModal
            settings={settings}
            systemCheck={systemCheck}
            onRefreshSystemCheck={refreshSystemCheck}
            onSave={saveSettings}
            onClose={() => setSettingsOpen(false)}
          />
        )}
        {failure && (
          <LaunchFailedDialog failure={failure} onClose={dismissFailure} />
        )}
      </MainLayout>
    </I18nProvider>
  );
}
