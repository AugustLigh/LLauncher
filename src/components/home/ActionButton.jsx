import { useTranslation } from '../../i18n';
import './ActionButton.css';

export default function ActionButton({ gameState, downloading, extracting, verifying, running, onAction, disabled }) {
  const { t } = useTranslation();

  if (running) {
    return (
      <button className="action-button action-button--downloading" disabled>
        {t('home.action.running')}
      </button>
    );
  }

  if (downloading) {
    const label = extracting
      ? t('home.action.extracting')
      : verifying
        ? t('home.action.verifying')
        : t('home.action.downloading');
    return (
      <button className="action-button action-button--downloading" disabled>
        {label}
      </button>
    );
  }

  const getLabel = () => {
    if (!gameState) return t('home.action.loading');
    switch (gameState.status) {
      case 'not_installed': return t('home.action.install');
      case 'update_available': return t('home.action.update');
      case 'ready': return t('home.action.launch');
      default: return t('home.action.launch');
    }
  };

  const isDisabled = disabled || !gameState;

  return (
    <button
      className={`action-button ${isDisabled ? 'action-button--disabled' : ''}`}
      onClick={onAction}
      disabled={isDisabled}
    >
      {getLabel()}
    </button>
  );
}
