import { useTranslation } from '../../i18n';
import { formatPlaytime, formatDate } from '../../utils/format';
import './GameStatus.css';

export default function GameStatus({ gameState, stats }) {
  const { t } = useTranslation();

  if (!gameState) {
    return (
      <div className="game-info">
        <div className="game-info__subtitle">{t('home.gameSubtitle')}</div>
        <div className="game-info__title">{t('common.loading')}</div>
      </div>
    );
  }

  const getBadge = () => {
    switch (gameState.status) {
      case 'ready':
        return { cls: 'game-info__badge--ready', text: t('home.badge.ready') };
      case 'update_available':
        return { cls: 'game-info__badge--update', text: t('home.badge.update') };
      case 'not_installed':
        return { cls: 'game-info__badge--not-installed', text: t('home.badge.notInstalled') };
      default:
        return { cls: '', text: '' };
    }
  };

  const renderVersion = () => {
    switch (gameState.status) {
      case 'ready':
        return <span className="game-info__version">v{gameState.version}</span>;
      case 'update_available':
        return (
          <span className="game-info__version">
            v{gameState.installed_version}
            <span className="game-info__version-arrow">{'→'}</span>
            v{gameState.latest_version}
          </span>
        );
      case 'not_installed':
        return <span className="game-info__version">v{gameState.latest_version}</span>;
      default:
        return null;
    }
  };

  const badge = getBadge();

  return (
    <div className="game-info">
      <div className="game-info__subtitle">{t('home.gameSubtitle')}</div>
      <div className="game-info__title">{t('home.gameTitle')}</div>
      <div className="game-info__meta">
        <div className={`game-info__badge ${badge.cls}`}>
          <span className="game-info__badge-dot" />
          {badge.text}
        </div>
        {renderVersion()}
      </div>
      {stats?.lastPlayed > 0 && (
        <div className="game-info__stats">
          <div>
            {t('home.stats.playtime', { time: formatPlaytime(stats.totalPlaytimeSecs) })}
            {' · '}
            {t('home.stats.lastPlayed', { date: formatDate(stats.lastPlayed) })}
          </div>
          {stats.weekSecs > 0 && (
            <div className="game-info__stats-week">
              <span>
                {t('home.stats.week', { time: formatPlaytime(stats.weekSecs) })}
                {stats.avgSessionSecs > 0 && (
                  <>
                    {' · '}
                    {t('home.stats.avgSession', { time: formatPlaytime(stats.avgSessionSecs) })}
                  </>
                )}
              </span>
              <div className="game-info__spark" title={t('home.stats.sparkTooltip')}>
                {stats.days.map((secs, i) => {
                  const max = Math.max(...stats.days, 1);
                  return (
                    <div
                      key={i}
                      className={`game-info__spark-bar ${secs > 0 ? 'game-info__spark-bar--active' : ''}`}
                      style={{ height: `${4 + Math.round((secs / max) * 12)}px` }}
                    />
                  );
                })}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
