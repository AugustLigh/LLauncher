// @ts-nocheck
import React, { useId, ReactNode } from "react";
import Icon from "./Icon";

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  icon?: string;
  variant?: "primary" | "secondary" | "danger" | "ghost";
}

export function Button({
  children,
  icon,
  variant = "secondary",
  className = "",
  ...props
}: ButtonProps) {
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

export interface SwitchProps {
  label: ReactNode;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  note?: ReactNode;
}

export function Switch({ label, checked, onChange, disabled = false, note }: SwitchProps) {
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

export interface FieldProps {
  label: ReactNode;
  children: ReactNode | ((id: string) => ReactNode);
  hint?: ReactNode;
}

export function Field({ label, children, hint }: FieldProps) {
  const id = useId();
  return (
    <div className="setting-field">
      <label htmlFor={id}>{label}</label>
      {typeof children === "function" ? children(id) : children}
      {hint && <small>{hint}</small>}
    </div>
  );
}

export interface StatusProps {
  children: ReactNode;
  kind?: "neutral" | "success" | "warning" | "error";
  busy?: boolean;
}

export function Status({ children, kind = "neutral", busy = false }: StatusProps) {
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
