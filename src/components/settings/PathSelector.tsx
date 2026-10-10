import { ReactNode, useId, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "../../i18n";
import { Button } from "../common/Controls";
import "./PathSelector.css";

export interface PathSelectorProps {
  value?: string;
  onChange: (value: string) => void;
  label?: ReactNode;
  disabled?: boolean;
}

export default function PathSelector({
  value = "",
  onChange,
  label,
  disabled = false,
}: PathSelectorProps) {
  const { t } = useTranslation();
  const id = useId();
  const [error, setError] = useState<string | null>(null);
  const labelStr = typeof label === "string" ? label : "";
  return (
    <div className="path-field">
      {label && <label htmlFor={id}>{label}</label>}
      <div className="path-selector">
        <input
          id={id}
          aria-label={labelStr || t("ui.chooseFolder")}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          spellCheck={false}
          disabled={disabled}
        />
        <Button
          icon="folder"
          disabled={disabled}
          aria-label={`${t("common.browse")}: ${labelStr || t("ui.chooseFolder")}`}
          onClick={async () => {
            setError(null);
            try {
              const selected = await open({ directory: true });
              if (selected && typeof selected === "string") onChange(selected);
            } catch (e) {
              setError(String(e));
            }
          }}
        >
          {t("common.browse")}
        </Button>
      </div>
      {error && (
        <span className="ui-error" role="alert">
          {error}
        </span>
      )}
    </div>
  );
}
