import { useTranslation } from "../../../i18n";
import { Field } from "../../common/Controls";
import LinuxLaunchOptions from "../LinuxLaunchOptions";
import MacosLaunchOptions from "../MacosLaunchOptions";
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
  // Until the backend answers, assume Linux (the historical behaviour). Both
  // Unix platforms have a runtime to pick and a prefix; only what it is
  // called and which setting holds it differ.
  const platform = systemCheck?.platform || "linux";
  const linux = platform === "linux",
    mac = platform === "macos";
  return (
    <>
      {(linux || mac) && (
        <RuntimeSettings
          form={form}
          onChange={onChange}
          systemCheck={systemCheck}
          initialOpen={initialRuntime}
          busy={busy}
          activeProton={activeProton}
          field={mac ? "macos_wine_dir" : "proton_dir"}
          mac={mac}
        />
      )}
      {linux && (
        <LinuxLaunchOptions
          form={form}
          onChange={onChange}
          systemCheck={systemCheck}
        />
      )}
      {mac && (
        <MacosLaunchOptions
          form={form}
          onChange={onChange}
          systemCheck={systemCheck}
        />
      )}
      {platform === "windows" && (
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
