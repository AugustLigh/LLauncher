import { useId } from "react";
import Icon from "./Icon";
export function Button({
  children,
  icon,
  variant = "secondary",
  className = "",
  ...props
}) {
  return (
    <button
      className={`ui-button ui-button--${variant} ${className}`}
      {...props}
    >
      {icon && <Icon name={icon} />}
      {children}
    </button>
  );
}
export function Switch({ label, checked, onChange, disabled = false, note }) {
  const id = useId();
  return (
    <div className="setting-row">
      <div className="setting-row__label">
        <span id={id}>{label}</span>
        {note && <small>{note}</small>}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={!!checked}
        aria-labelledby={id}
        disabled={disabled}
        className="ui-switch"
        onClick={() => onChange(!checked)}
      >
        <span />
      </button>
    </div>
  );
}
export function Field({ label, children, hint }) {
  const id = useId();
  return (
    <div className="setting-field">
      <label htmlFor={id}>{label}</label>
      {typeof children === "function" ? children(id) : children}
      {hint && <small>{hint}</small>}
    </div>
  );
}
export function Status({ children, kind = "neutral", busy = false }) {
  return (
    <span className={`ui-status ui-status--${kind}`} role="status">
      <Icon
        name={
          busy
            ? "refresh"
            : kind === "success"
              ? "check"
              : kind === "error"
                ? "alert"
                : "clock"
        }
        className={busy ? "is-spinning" : ""}
        size={16}
      />
      {children}
    </span>
  );
}
