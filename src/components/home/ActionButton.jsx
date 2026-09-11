import Icon from "../common/Icon";
import "./ActionButton.css";
export default function ActionButton({
  children,
  onClick,
  disabled,
  busy,
  icon,
  attract,
}) {
  const label = typeof children === "string" ? children : "";
  const classes = [
    "action-button",
    attract && !disabled && "action-button--attract",
    label.length > 16
      ? "action-button--long"
      : label.length > 10 && "action-button--medium",
  ].filter(Boolean);
  return (
    <button
      className={classes.join(" ")}
      onClick={onClick}
      disabled={disabled}
      aria-busy={busy || undefined}
    >
      {busy ? (
        <Icon name="refresh" className="is-spinning" size={18} />
      ) : (
        icon &&
        !disabled && (
          <Icon
            name={icon}
            className={`action-button__icon${icon === "play" ? " action-button__icon--solid" : ""}`}
            size={22}
          />
        )
      )}
      <span>{children}</span>
    </button>
  );
}
