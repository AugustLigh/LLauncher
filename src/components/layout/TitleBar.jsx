import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "../../i18n";
import useLauncherUpdate from "../../hooks/useLauncherUpdate";
import Icon from "../common/Icon";
import "./TitleBar.css";
const appWindow = getCurrentWindow();
export default function TitleBar({ onOpenSettings, settingsOpen }) {
  const { t } = useTranslation();
  const update = useLauncherUpdate();
  return (
    <header
      className={`titlebar${settingsOpen ? " titlebar--settings" : ""}`}
      data-tauri-drag-region
    >
      <div className="titlebar__controls">
        {update && (
          <button
            className="titlebar__update"
            onClick={() => openUrl(update.url)}
            title={t("titlebar.updateTooltip")}
          >
            <Icon name="download" size={15} />
            LLauncher {update.version}
          </button>
        )}
        {!settingsOpen && (
          <button
            className="titlebar__settings"
            onClick={onOpenSettings}
            aria-label={t("settings.title")}
            title={t("settings.title")}
          >
            <Icon name="settings" />
          </button>
        )}
        <button
          className="titlebar__btn"
          onClick={() => appWindow.minimize()}
          aria-label={t("titlebar.minimize")}
          title={t("titlebar.minimize")}
        >
          <Icon name="minus" size={17} />
        </button>
        <button
          className="titlebar__btn titlebar__btn--close"
          onClick={() => appWindow.hide()}
          aria-label={t("ui.shutdownHint")}
          title={t("ui.shutdownHint")}
        >
          <Icon name="close" size={17} />
        </button>
      </div>
    </header>
  );
}
