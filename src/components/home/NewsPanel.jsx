import { useState } from 'react';
import NewsItem from './NewsItem';
import { useTranslation } from '../../i18n';
import './NewsPanel.css';

export default function NewsPanel({ tabs }) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState(0);

  if (!tabs || tabs.length === 0) {
    return (
      <div className="news-panel">
        <div className="news-panel__content">
          <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--font-size-sm)' }}>
            {t('news.empty')}
          </span>
        </div>
      </div>
    );
  }

  const currentTab = tabs[activeTab];

  return (
    <div className="news-panel">
      <div className="news-panel__tabs">
        {tabs.map((tab, i) => (
          <button
            key={i}
            className={`news-panel__tab ${i === activeTab ? 'news-panel__tab--active' : ''}`}
            onClick={() => setActiveTab(i)}
          >
            {tab.tab_name}
          </button>
        ))}
      </div>
      <div className="news-panel__content">
        {currentTab?.announcements?.map((item, i) => (
          <NewsItem key={i} item={item} index={i} />
        ))}
      </div>
    </div>
  );
}
