import { useTranslation } from "../../i18n";
import { Switch, Status } from "../common/Controls";
// The launch options that only exist on macOS. There is no wrapper ecosystem
// here (no gamemode, MangoHud, gamescope): what the Mac has instead is a
// choice of renderer path — Direct3D 11 through DXMT, or the game's own
// Vulkan renderer over MoltenVK — and two knobs that belong to Rosetta and
// Metal rather than to Wine. Above them, what the active Wine contains: the
// Endfield module set is what gets the game past its anti-cheat, so a build
// without it is called out rather than left to fail at launch.
export default function MacosLaunchOptions({ form, onChange, systemCheck }) {
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
      {systemCheck?.proton_path && (
        <div className="setting-row">
          <div className="setting-row__label">
            <span>{t("settings.macos.wineInUse")}</span>
            <small className="selectable">{systemCheck.proton_path}</small>
            <small>
              {systemCheck.wine_patch_version
                ? t("settings.macos.modulesPresent", {
                    version: systemCheck.wine_patch_version,
                  })
                : t("settings.macos.modulesAbsent")}
            </small>
            <small>
              {systemCheck.dxmt_version
                ? t("settings.macos.dxmtPresent", {
                    version: systemCheck.dxmt_version,
                  })
                : t("settings.macos.dxmtAbsent")}
            </small>
          </div>
          <Status
            kind={
              systemCheck.wine_patch_version && systemCheck.dxmt_version
                ? "success"
                : "warning"
            }
          >
            {t(
              systemCheck.wine_patch_version
                ? "ui.runtimeReady"
                : "ui.wineUnpatched",
            )}
          </Status>
        </div>
      )}
      {toggle(
        "macos_native_vulkan",
        t("settings.macos.vulkan.name"),
        t("settings.macos.vulkan.desc"),
      )}
      {toggle(
        "macos_advertise_avx",
        t("settings.macos.avx.name"),
        t("settings.macos.avx.desc"),
      )}
      {toggle(
        "macos_metal_hud",
        t("settings.macos.metalHud.name"),
        t("settings.macos.metalHud.desc"),
      )}
    </>
  );
}
