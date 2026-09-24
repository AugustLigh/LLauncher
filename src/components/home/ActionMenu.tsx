import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "../../i18n";
import Icon from "../common/Icon";
import "./ActionMenu.css";

export interface ActionMenuItem {
  label: string;
  icon: string;
  disabled?: boolean;
  onSelect: () => void;
}

export interface ActionMenuProps {
  items: ActionMenuItem[];
}

export default function ActionMenu({ items }: ActionMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement | null>(null);
  const trigger = useRef<HTMLButtonElement | null>(null);
  const id = useId();
  useEffect(() => {
    if (!open) return;
    const dismiss = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", dismiss);
    return () => document.removeEventListener("pointerdown", dismiss);
  }, [open]);
  return (
    <div
      className="action-menu"
      ref={ref}
      onKeyDown={(event) => {
        if (event.key === "Escape" && open) {
          event.stopPropagation();
          setOpen(false);
          trigger.current?.focus();
        }
      }}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node)) setOpen(false);
      }}
    >
      <button
        ref={trigger}
        className="action-menu__trigger"
        aria-label={t("ui.gameActions")}
        title={t("ui.gameActions")}
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen((value) => !value)}
      >
        <Icon name="menu" size={19} />
      </button>
      {open && (
        <div
          id={id}
          className="action-menu__items"
          role="group"
          aria-label={t("ui.gameActions")}
        >
          {items.map((item) => (
            <button
              key={item.label}
              disabled={item.disabled}
              onClick={() => {
                setOpen(false);
                item.onSelect();
              }}
            >
              <Icon name={item.icon} size={17} />
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
