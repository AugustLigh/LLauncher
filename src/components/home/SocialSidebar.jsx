import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "../../i18n";
import SocialIcon from "./SocialIcons";
import launcherIcon from "../../assets/launcher-icon.png";
import "./SocialSidebar.css";

export default function SocialSidebar({ sidebars = [] }) {
  const { t } = useTranslation();
  return (
    <aside className="social-sidebar">
      <img
        className="social-sidebar__app-icon"
        src={launcherIcon}
        alt="LLauncher"
        title="LLauncher"
        draggable={false}
      />
      <nav className="social-sidebar__links" aria-label={t("ui.community")}>
        {sidebars
          .filter((item) => item.jump_url)
          .map((item, i) => {
            const label =
              item.sidebar_labels?.[0]?.content ||
              item.media ||
              t("ui.community");
            return (
              <a
                key={`${item.jump_url}-${i}`}
                href={item.jump_url}
                aria-label={label}
                title={label}
                onClick={(event) => {
                  event.preventDefault();
                  openUrl(item.jump_url).catch(console.error);
                }}
              >
                <span aria-hidden="true">
                  <SocialIcon media={item.media} />
                </span>
              </a>
            );
          })}
      </nav>
    </aside>
  );
}
