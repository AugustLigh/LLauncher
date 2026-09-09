import { useTranslation } from "../../i18n";
import { Field, Switch } from "../common/Controls";
export default function LinuxLaunchOptions({ form, onChange, systemCheck }) {
  const { t } = useTranslation();
  const toggle = (key, label, missing = false, inverse = false) => (
    <Switch
      key={key}
      label={label}
      checked={inverse ? !form[key] : !!form[key]}
      onChange={(v) => onChange(key, inverse ? !v : v)}
      disabled={missing && !form[key]}
      note={missing ? t("settings.unavailable") : null}
    />
  );
  const field = (key, label, placeholder = "", type = "text") => (
    <Field label={label}>
      {(id) => (
        <input
          id={id}
          type={type}
          min={type === "number" ? 0 : undefined}
          value={form[key] ?? ""}
          placeholder={placeholder}
          onChange={(e) =>
            onChange(
              key,
              type === "number"
                ? Math.max(0, Number(e.target.value) || 0)
                : e.target.value,
            )
          }
          spellCheck={false}
        />
      )}
    </Field>
  );
  const select = (key, label, options, fallback) => (
    <Field label={label}>
      {(id) => (
        <select
          id={id}
          value={form[key] || fallback}
          onChange={(e) => onChange(key, e.target.value)}
        >
          {options.map(([value, text]) => (
            <option key={value} value={value}>
              {text}
            </option>
          ))}
        </select>
      )}
    </Field>
  );
  return (
    <>
      {toggle("use_sdl_input", t("ui.controller"))}
      {toggle(
        "use_mangohud",
        t("ui.fps"),
        systemCheck && !systemCheck.has_mangohud,
      )}
      <details className="ui-details">
        <summary>{t("ui.advancedLaunch")}</summary>
        <div className="ui-details__body">
          <Field label={t("ui.renderer")}>
            {(id) => (
              <select
                id={id}
                value={form.use_native_vulkan ? "vulkan" : "dx11"}
                onChange={(e) =>
                  onChange("use_native_vulkan", e.target.value === "vulkan")
                }
              >
                <option value="vulkan">Vulkan</option>
                <option value="dx11">DirectX 11</option>
              </select>
            )}
          </Field>
          {toggle("use_wayland", "Wayland")}
          {toggle(
            "use_gamemode",
            "GameMode",
            systemCheck && !systemCheck.has_gamemode,
          )}
          {toggle("use_dxvk_async", "DXVK Async")}
          {toggle("disable_fsync", t("ui.useFsync"), false, true)}
          {toggle("disable_esync", t("ui.useEsync"), false, true)}
          {toggle("use_prime_offload", t("settings.prime.name"))}
          {toggle("use_canonical_hole", t("settings.canonicalHole.name"))}
          {toggle(
            "use_gamescope",
            "Gamescope",
            systemCheck && !systemCheck.has_gamescope,
          )}
          {form.use_gamescope && (
            <div className="settings-gamescope">
              <div className="settings-field-grid">
                {select(
                  "gamescope_mode",
                  t("settings.gamescope.mode"),
                  [
                    ["fullscreen", t("settings.gamescope.modeFullscreen")],
                    ["borderless", t("settings.gamescope.modeBorderless")],
                    ["windowed", t("settings.gamescope.modeWindowed")],
                  ],
                  "fullscreen",
                )}
                {select(
                  "gamescope_upscaler",
                  t("settings.gamescope.upscaler"),
                  [
                    ["auto", t("settings.gamescope.upscalerAuto")],
                    ["fsr", "AMD FSR"],
                    ["nis", "NVIDIA NIS"],
                    ["integer", t("settings.gamescope.upscalerInteger")],
                    ["stretch", t("settings.gamescope.upscalerStretch")],
                  ],
                  "auto",
                )}
              </div>
              <div className="settings-field-grid">
                {field(
                  "gamescope_render_res",
                  t("settings.gamescope.renderRes"),
                  t("settings.gamescope.resNative"),
                )}
                {field(
                  "gamescope_output_res",
                  t("settings.gamescope.outputRes"),
                  t("settings.gamescope.resAuto"),
                )}
                {field("gamescope_fps_limit", t("ui.fpsLimit"), "0", "number")}
              </div>
              {toggle("gamescope_hdr", t("settings.gamescope.hdr.name"))}
              {field("gamescope_extra_args", t("settings.gamescope.extraArgs"))}
            </div>
          )}
        </div>
      </details>
    </>
  );
}
