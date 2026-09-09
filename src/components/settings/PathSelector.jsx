import { useId, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "../../i18n";
import { Button } from "../common/Controls";
import "./PathSelector.css";
export default function PathSelector({
  value = "",
  onChange,
  label,
  disabled = false,
}) {
  const { t } = useTranslation();
  const id = useId();
  const [error, setError] = useState(null);
  return (
    <div className="path-field">
      {label && <label htmlFor={id}>{label}</label>}
      <div className="path-selector">
        <input
          id={id}
          aria-label={label || t("ui.chooseFolder")}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          spellCheck={false}
          disabled={disabled}
        />
        <Button
          icon="folder"
          disabled={disabled}
          aria-label={`${t("common.browse")}: ${label || t("ui.chooseFolder")}`}
          onClick={async () => {
            setError(null);
            try {
              const selected = await open({ directory: true });
              if (selected) onChange(selected);
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
