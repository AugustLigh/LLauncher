import { useTranslation } from "../../../i18n";
import { Field } from "../../common/Controls";
import LinuxLaunchOptions from "../LinuxLaunchOptions";
import WindowsLaunchOptions from "../WindowsLaunchOptions";
import RuntimeSettings from "./RuntimeSettings";
export default function LaunchSettings({
  form,
  onChange,
  systemCheck,
  initialRuntime,
  busy,
  activeProton,
}) {
  const { t } = useTranslation();
  const linux = systemCheck?.platform !== "windows";
  return (
    <>
      {linux && (
        <RuntimeSettings
          form={form}
          onChange={onChange}
          systemCheck={systemCheck}
          initialOpen={initialRuntime}
          busy={busy}
          activeProton={activeProton}
        />
      )}
      {linux ? (
        <LinuxLaunchOptions
          form={form}
          onChange={onChange}
          systemCheck={systemCheck}
        />
      ) : (
        <WindowsLaunchOptions form={form} onChange={onChange} />
      )}
      <details className="ui-details">
        <summary>
          {t("settings.launchArgs")} / {t("settings.envVars")}
        </summary>
        <div className="ui-details__body">
          <Field label={t("settings.launchArgs")}>
            {(id) => (
              <input
                id={id}
                value={form.custom_launch_args || ""}
                onChange={(e) => onChange("custom_launch_args", e.target.value)}
                spellCheck={false}
              />
            )}
          </Field>
          <Field label={t("settings.envVars")}>
            {(id) => (
              <textarea
                id={id}
                value={form.custom_env_vars || ""}
                onChange={(e) => onChange("custom_env_vars", e.target.value)}
                placeholder="KEY=VALUE"
                spellCheck={false}
              />
            )}
          </Field>
        </div>
      </details>
    </>
  );
}
