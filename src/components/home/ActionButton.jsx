import Icon from "../common/Icon";
import "./ActionButton.css";
export default function ActionButton({ children, onClick, disabled, busy }) {
  return (
    <button
      className="action-button"
      onClick={onClick}
      disabled={disabled}
      aria-busy={busy || undefined}
    >
      {busy && <Icon name="refresh" className="is-spinning" size={18} />}
      <span>{children}</span>
    </button>
  );
}
