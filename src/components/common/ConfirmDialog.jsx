import useModalDismiss from '../../hooks/useModalDismiss';
import { useTranslation } from '../../i18n';
import './ConfirmDialog.css';

// The app's own confirmation dialog, replacing the native confirm()/alert()
// which render as unstyled WebKit chrome in a decorations-off window and are
// not keyboard/gamepad friendly. `danger` tints the confirm button red.
// When `onCancel` is omitted the dialog is a plain alert (single OK button).
export default function ConfirmDialog({
  title,
  message,
  confirmLabel,
  cancelLabel,
  danger = false,
  onConfirm,
  onCancel,
}) {
  const { t } = useTranslation();
  const dismiss = onCancel || onConfirm;
  useModalDismiss(dismiss);

  return (
    <div className="confirm-overlay" onClick={dismiss}>
      <div
        className="confirm-dialog"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
      >
        {title && <div className="confirm-dialog__title">{title}</div>}
        <div className="confirm-dialog__message">{message}</div>
        <div className="confirm-dialog__actions">
          {onCancel && (
            <button
              className="confirm-dialog__btn confirm-dialog__btn--secondary"
              onClick={onCancel}
            >
              {cancelLabel || t('common.cancel')}
            </button>
          )}
          <button
            className={`confirm-dialog__btn ${danger ? 'confirm-dialog__btn--danger' : 'confirm-dialog__btn--primary'}`}
            onClick={onConfirm}
            autoFocus
          >
            {confirmLabel || t('common.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
}
