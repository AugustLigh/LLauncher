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
          {t('home.stats.playtime', { time: formatPlaytime(stats.totalPlaytimeSecs) })}
          {' · '}
          {t('home.stats.lastPlayed', { date: formatDate(stats.lastPlayed) })}
        </div>
      )}
    </div>
  );
}
