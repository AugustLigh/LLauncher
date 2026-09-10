import { useTranslation } from "../../i18n";
import { Switch } from "../common/Controls";
// The launch options that only exist on Windows. The game runs natively, so
// there is no compatibility layer to configure — what the launcher can do is
// pick the renderer the game starts on, register the game with Windows' own
// per-app graphics settings, and shape the host for the session (power plan,
// process priority, elevation).
export default function WindowsLaunchOptions({ form, onChange }) {
  const { t } = useTranslation();
  const toggle = (key, name, desc) => (
    <Switch
      key={key}
      label={name}
      note={desc}
      checked={!!form[key]}
      onChange={(v) => onChange(key, v)}
    />
  );
  return (
    <>
      {toggle(
        "windows_use_vulkan",
        `${t("settings.windows.vulkan.name")} · ${t("settings.experimental")}`,
        t("settings.windows.vulkan.desc"),
      )}
      {toggle(
        "windows_prefer_dgpu",
        t("settings.windows.dgpu.name"),
        t("settings.windows.dgpu.desc"),
      )}
      {toggle(
        "windows_windowed_optimizations",
        t("settings.windows.windowedOptimizations.name"),
        t("settings.windows.windowedOptimizations.desc"),
      )}
      {toggle(
        "windows_high_perf_power",
        t("settings.windows.powerPlan.name"),
        t("settings.windows.powerPlan.desc"),
      )}
      {toggle(
        "windows_high_priority",
        t("settings.windows.priority.name"),
        t("settings.windows.priority.desc"),
      )}
      {toggle(
        "windows_run_as_admin",
        t("settings.runAsAdmin.name"),
        t("settings.runAsAdmin.desc"),
      )}
    </>
  );
}
