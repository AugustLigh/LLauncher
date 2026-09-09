import { useTranslation } from "../../../i18n";
import { Field, Switch } from "../../common/Controls";
import Icon from "../../common/Icon";
import LanguageSelector from "../LanguageSelector";
export default function GeneralSettings({
  form,
  onChange,
  autostart,
  onAutostart,
  autoError,
}) {
  const { t } = useTranslation();
  const action = form.on_launch_action === "nothing" ? "nothing" : "hide";
  return (
    <>
      <Field label={t("settings.language")}>
        {(id) => (
          <LanguageSelector
            id={id}
            value={form.language}
            onChange={(value) => onChange("language", value)}
          />
        )}
      </Field>
      <h3>{t("settings.afterLaunch")}</h3>
      <div className="settings-choices">
        {["hide", "nothing"].map((value) => (
          <button
            key={value}
            className="settings-choice"
            aria-pressed={action === value}
            onClick={() => onChange("on_launch_action", value)}
          >
            <Icon name={value === "hide" ? "minus" : "monitor"} size={30} />
            <span>{t(value === "hide" ? "ui.afterHide" : "ui.afterKeep")}</span>
            {action === value && <Icon name="check" size={16} />}
          </button>
        ))}
      </div>
      <Switch
        label={t("ui.autostart")}
        checked={autostart}
        onChange={onAutostart}
        disabled={autostart == null}
        note={autoError}
      />
      <Switch
        label={t("settings.discord.name")}
        checked={form.use_discord_rpc}
        onChange={(v) => onChange("use_discord_rpc", v)}
      />
    </>
  );
}
