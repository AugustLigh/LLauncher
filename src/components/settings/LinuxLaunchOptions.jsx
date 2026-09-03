import { useTranslation } from '../../i18n';
import './SettingsModal.css';

// The launch options that only exist on Linux: the renderer and windowing
// choices Proton acts on, the wrappers the launch script can put in front of
// it (gamemode, MangoHud, gamescope) and the Wine synchronisation primitives.
// On Windows the game runs natively and the settings dialog leaves the whole
// section out.
export default function LinuxLaunchOptions({ form, onChange, systemCheck }) {
  const { t } = useTranslation();

  return (
    <>
      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">{t('settings.vulkan.name')}</span>
          <span className="settings-toggle__desc">
            {t('settings.vulkan.desc')}
          </span>
        </div>
        <button
          className={`settings-toggle__switch ${form.use_native_vulkan ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('use_native_vulkan', !form.use_native_vulkan)}
        />
      </div>

      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">{t('settings.wayland.name')}</span>
          <span className="settings-toggle__desc">
            {t('settings.wayland.desc')}
          </span>
        </div>
        <button
          className={`settings-toggle__switch ${form.use_wayland ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('use_wayland', !form.use_wayland)}
        />
      </div>

      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">{t('settings.gamemode.name')}</span>
          <span className="settings-toggle__desc">
            {t('settings.gamemode.desc')}
          </span>
          {systemCheck && !systemCheck.has_gamemode && (
            <span className="settings-toggle__unavailable">{t('settings.unavailable')}</span>
          )}
        </div>
        <button
          className={`settings-toggle__switch ${form.use_gamemode ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('use_gamemode', !form.use_gamemode)}
        />
      </div>

      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">{t('settings.dxvkAsync.name')}</span>
          <span className="settings-toggle__desc">
            {t('settings.dxvkAsync.desc')}
          </span>
        </div>
        <button
          className={`settings-toggle__switch ${form.use_dxvk_async ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('use_dxvk_async', !form.use_dxvk_async)}
        />
      </div>

      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">{t('settings.fsync.name')}</span>
          <span className="settings-toggle__desc">
            {t('settings.fsync.desc')}
          </span>
        </div>
        <button
          className={`settings-toggle__switch ${form.disable_fsync ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('disable_fsync', !form.disable_fsync)}
        />
      </div>

      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">{t('settings.esync.name')}</span>
          <span className="settings-toggle__desc">
            {t('settings.esync.desc')}
          </span>
        </div>
        <button
          className={`settings-toggle__switch ${form.disable_esync ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('disable_esync', !form.disable_esync)}
        />
      </div>

      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">{t('settings.mangohud.name')}</span>
          <span className="settings-toggle__desc">
            {t('settings.mangohud.desc')}
          </span>
          {systemCheck && !systemCheck.has_mangohud && (
            <span className="settings-toggle__unavailable">{t('settings.unavailable')}</span>
          )}
        </div>
        <button
          className={`settings-toggle__switch ${form.use_mangohud ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('use_mangohud', !form.use_mangohud)}
        />
      </div>

      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">{t('settings.gamescope.name')}</span>
          <span className="settings-toggle__desc">
            {t('settings.gamescope.desc')}
          </span>
          {systemCheck && !systemCheck.has_gamescope && (
            <span className="settings-toggle__unavailable">{t('settings.unavailable')}</span>
          )}
        </div>
        <button
          className={`settings-toggle__switch ${form.use_gamescope ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('use_gamescope', !form.use_gamescope)}
        />
      </div>

      {form.use_gamescope && (
        <div className="settings-gamescope">
          <div className="settings-gamescope__row">
            <div className="settings-gamescope__field">
              <span className="settings-modal__label">{t('settings.gamescope.mode')}</span>
              <select
                className="settings-proton__select"
                value={form.gamescope_mode || 'fullscreen'}
                onChange={(e) => onChange('gamescope_mode', e.target.value)}
              >
                <option value="fullscreen">{t('settings.gamescope.modeFullscreen')}</option>
                <option value="borderless">{t('settings.gamescope.modeBorderless')}</option>
                <option value="windowed">{t('settings.gamescope.modeWindowed')}</option>
              </select>
            </div>
            <div className="settings-gamescope__field">
              <span className="settings-modal__label">{t('settings.gamescope.upscaler')}</span>
              <select
                className="settings-proton__select"
                value={form.gamescope_upscaler || 'auto'}
                onChange={(e) => onChange('gamescope_upscaler', e.target.value)}
              >
                <option value="auto">{t('settings.gamescope.upscalerAuto')}</option>
                <option value="fsr">AMD FSR</option>
                <option value="nis">NVIDIA NIS</option>
                <option value="integer">{t('settings.gamescope.upscalerInteger')}</option>
                <option value="stretch">{t('settings.gamescope.upscalerStretch')}</option>
              </select>
            </div>
          </div>

          <div className="settings-gamescope__row">
            <div className="settings-gamescope__field">
              <span className="settings-modal__label">{t('settings.gamescope.renderRes')}</span>
              <input
                value={form.gamescope_render_res || ''}
                onChange={(e) => onChange('gamescope_render_res', e.target.value)}
                placeholder={t('settings.gamescope.resNative')}
                spellCheck={false}
              />
            </div>
            <div className="settings-gamescope__field">
              <span className="settings-modal__label">{t('settings.gamescope.outputRes')}</span>
              <input
                value={form.gamescope_output_res || ''}
                onChange={(e) => onChange('gamescope_output_res', e.target.value)}
                placeholder={t('settings.gamescope.resAuto')}
                spellCheck={false}
              />
            </div>
            <div className="settings-gamescope__field">
              <span className="settings-modal__label">{t('settings.gamescope.fps')}</span>
              <input
                type="number"
                min="0"
                value={form.gamescope_fps_limit || ''}
                onChange={(e) => onChange('gamescope_fps_limit', parseInt(e.target.value, 10) || 0)}
                placeholder={t('settings.gamescope.fpsOff')}
              />
            </div>
          </div>

          <div className="settings-toggle settings-toggle--sub">
            <div className="settings-toggle__info">
              <span className="settings-toggle__name">{t('settings.gamescope.hdr.name')}</span>
              <span className="settings-toggle__desc">{t('settings.gamescope.hdr.desc')}</span>
            </div>
            <button
              className={`settings-toggle__switch ${form.gamescope_hdr ? 'settings-toggle__switch--on' : ''}`}
              onClick={() => onChange('gamescope_hdr', !form.gamescope_hdr)}
            />
          </div>

          <div className="settings-gamescope__field">
            <span className="settings-modal__label">{t('settings.gamescope.extraArgs')}</span>
            <input
              value={form.gamescope_extra_args || ''}
              onChange={(e) => onChange('gamescope_extra_args', e.target.value)}
              placeholder="--adaptive-sync --force-grab-cursor"
              spellCheck={false}
            />
          </div>

          <span className="settings-modal__hint">{t('settings.gamescope.hint')}</span>
        </div>
      )}

      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">{t('settings.prime.name')}</span>
          <span className="settings-toggle__desc">
            {t('settings.prime.desc')}
          </span>
        </div>
        <button
          className={`settings-toggle__switch ${form.use_prime_offload ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('use_prime_offload', !form.use_prime_offload)}
        />
      </div>

      <div className="settings-toggle">
        <div className="settings-toggle__info">
          <span className="settings-toggle__name">
            {t('settings.canonicalHole.name')}
            <span className="settings-toggle__experimental">{t('settings.experimental')}</span>
          </span>
          <span className="settings-toggle__desc">
            {t('settings.canonicalHole.desc')}
          </span>
        </div>
        <button
          className={`settings-toggle__switch ${form.use_canonical_hole ? 'settings-toggle__switch--on' : ''}`}
          onClick={() => onChange('use_canonical_hole', !form.use_canonical_hole)}
        />
      </div>
    </>
  );
}
