import { useId, useState } from "react";
import NewsItem from "./NewsItem";
import { useTranslation } from "../../i18n";
import { Button } from "../common/Controls";
import "./NewsPanel.css";
export default function NewsPanel({ tabs = [], loading, error, onRetry }) {
  const { t } = useTranslation();
  const [active, setActive] = useState(0),
    [expanded, setExpanded] = useState(false);
  const id = useId();
  const index = active < tabs.length ? active : 0;
  const entries = tabs[index]?.announcements || [];
  return (
    <section className="news-panel" aria-label={t("ui.gameNews")}>
      <div className="news-panel__heading">
        <h2>{t("ui.gameNews")}</h2>
        {entries.length > 3 && (
          <button
            onClick={() => setExpanded(!expanded)}
            aria-expanded={expanded}
          >
            {t(expanded ? "ui.lessNews" : "ui.allNews")}
          </button>
        )}
      </div>
      {tabs.length > 0 && (
        <div
          className="news-panel__tabs"
          role="tablist"
          aria-label={t("ui.gameNews")}
        >
          {tabs.map((tab, i) => (
            <button
              id={`${id}-tab-${i}`}
              key={i}
              role="tab"
              aria-selected={index === i}
              aria-controls={`${id}-panel`}
              tabIndex={index === i ? 0 : -1}
              className={`news-panel__tab ${index === i ? "news-panel__tab--active" : ""}`}
              onClick={() => setActive(i)}
              onKeyDown={(e) => {
                if (
                  ["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)
                ) {
                  e.preventDefault();
                  const next =
                    e.key === "Home"
                      ? 0
                      : e.key === "End"
                        ? tabs.length - 1
                        : (index +
                            (e.key === "ArrowRight" ? 1 : -1) +
                            tabs.length) %
                          tabs.length;
                  setActive(next);
                  document.getElementById(`${id}-tab-${next}`)?.focus();
                }
              }}
            >
              {tab.tabName?.trim() || tab.tab_name?.trim() || t("ui.gameNews")}
            </button>
          ))}
        </div>
      )}
      <div
        id={`${id}-panel`}
        className={`news-panel__content ${expanded ? "news-panel__content--expanded" : ""}`}
        role={tabs.length > 0 ? "tabpanel" : undefined}
        aria-labelledby={tabs.length > 0 ? `${id}-tab-${index}` : undefined}
      >
        {error && (
          <div className="news-panel__empty">
            <span>{t("ui.newsFailed")}</span>
            <Button icon="refresh" variant="ghost" onClick={onRetry}>
              {t("common.retry")}
            </Button>
          </div>
        )}
        {!error && !entries.length && (
          <span className="news-panel__empty">
            {t(loading ? "common.loading" : "ui.newsEmpty")}
          </span>
        )}
        {(expanded ? entries : entries.slice(0, 3)).map((item, i) => (
          <NewsItem key={`${item.jump_url || "news"}-${i}`} item={item} />
        ))}
      </div>
    </section>
  );
}
