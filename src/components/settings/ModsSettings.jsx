import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useTranslation } from '../../i18n';
import './ModsSettings.css';

const CATALOG_URL = 'https://gamebanana.com/games/21842';
const VKBASALT_URL = 'https://github.com/DadSchoorse/vkBasalt';

// Two kinds of mods, two cards — because the difference that actually matters
// to a user is not the technology but the price: replacing models forces the
// game onto D3D11 and costs frames, while post-processing rides along on the
// native Vulkan renderer for free. Each row states its own status, so the tab
// can be understood by glancing at it rather than by reading paragraphs.
export default function ModsSettings({ form, onChange, systemCheck }) {
  const { t } = useTranslation();
  const [status, setStatus] = useState(null);
  const [busy, setBusy] = useState(null);
  const [msg, setMsg] = useState(null);

  const isLinux = (systemCheck?.platform || 'linux') !== 'windows';

  const refresh = useCallback(async () => {
    try {
      setStatus(await invoke('get_mods_status'));
    } catch {
      setStatus(null);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const run = async (key, cmd, okText) => {
    setBusy(key);
    setMsg(null);
    try {
      const res = await invoke(cmd);
      setMsg({ ok: true, text: okText(res) });
      await refresh();
    } catch (e) {
      setMsg({ ok: false, text: typeof e === 'string' ? e : e.message || String(e) });
    } finally {
      setBusy(null);
    }
  };

  const loaderReady = !!status?.loader_installed && !!status?.loader_configured;
  const modCount = status?.mod_count || 0;

  if (status?.game_dir_missing) {
    return <span className="settings-modal__hint">{t('settings.mods.noGame')}</span>;
  }

  return (
    <>
      <div className="mods__card">
        <div className="mods__card-head">
          <span className="mods__card-title">{t('settings.mods.skins.title')}</span>
          <span className="mods__tag mods__tag--cost">{t('settings.mods.skins.cost')}</span>
        </div>
        <span className="mods__card-sub">{t('settings.mods.skins.sub')}</span>

        <Row done={loaderReady} label={t('settings.mods.skins.loader')}>
          {loaderReady ? (
            <button
              className="mods__btn mods__btn--quiet"
              onClick={() => run('uninstall', 'uninstall_mod_loader', () => t('settings.mods.uninstalled'))}
              disabled={!!busy}
            >
              {busy === 'uninstall' ? '…' : t('settings.mods.skins.remove')}
            </button>
          ) : (
            <button
              className="mods__btn mods__btn--go"
              onClick={() => run('install', 'install_mod_loader', (r) => t('settings.mods.installed', { version: r.version }))}
              disabled={!!busy}
            >
              {busy === 'install' ? t('settings.mods.skins.installing') : t('settings.mods.skins.install')}
            </button>
          )}
        </Row>

        <Row
          done={modCount > 0}
          label={t('settings.mods.skins.folder')}
          note={modCount > 0 ? t('settings.mods.skins.count', { count: modCount }) : t('settings.mods.skins.empty')}
        >
          <button className="mods__btn" onClick={() => run('open', 'open_mods_folder', () => '')}>
            {t('settings.mods.skins.open')}
          </button>
          <button className="mods__btn" onClick={() => openUrl(CATALOG_URL)}>
            {t('settings.mods.skins.catalog')}
          </button>
        </Row>

        <Row done={!!form.mods_enabled} label={t('settings.mods.skins.button')}>
          <Switch on={!!form.mods_enabled} onToggle={() => onChange('mods_enabled', !form.mods_enabled)} />
        </Row>
      </div>

      <div className="mods__card">
        <div className="mods__card-head">
          <span className="mods__card-title">{t('settings.mods.looks.title')}</span>
          <span className="mods__tag mods__tag--free">{t('settings.mods.looks.free')}</span>
        </div>
        <span className="mods__card-sub">{t('settings.mods.looks.sub')}</span>

        {isLinux && (
          <Row
            done={!!form.use_vkbasalt && !!systemCheck?.has_vkbasalt}
            label={t('settings.mods.looks.vkbasalt')}
            note={systemCheck && !systemCheck.has_vkbasalt ? t('settings.mods.looks.vkbasaltMissing') : null}
          >
            {systemCheck && !systemCheck.has_vkbasalt ? (
              <button className="mods__btn" onClick={() => openUrl(VKBASALT_URL)}>
                {t('settings.mods.looks.howto')}
              </button>
            ) : (
              <Switch on={!!form.use_vkbasalt} onToggle={() => onChange('use_vkbasalt', !form.use_vkbasalt)} />
            )}
          </Row>
        )}

        <Row
          done={!!status?.reshade_installed}
          label={t('settings.mods.looks.reshade')}
          note={status?.reshade_installed ? t('settings.mods.looks.reshadeFound') : t('settings.mods.looks.reshadeHint')}
        />
      </div>

      {msg && msg.text && (
        <span className={`settings-modal__msg ${msg.ok ? 'settings-modal__msg--ok' : 'settings-modal__msg--err'}`}>
          {msg.text}
        </span>
      )}

      <span className="mods__risk">{t('settings.mods.risk')}</span>
    </>
  );
}

function Row({ done, label, note, children }) {
  return (
    <div className="mods__row">
      <span className={`mods__dot ${done ? 'mods__dot--on' : ''}`}>{done ? '✓' : ''}</span>
      <span className="mods__row-label">
        {label}
        {note && <span className="mods__row-note">{note}</span>}
      </span>
      <span className="mods__row-actions">{children}</span>
    </div>
  );
}

function Switch({ on, onToggle }) {
  return (
    <button
      className={`settings-toggle__switch ${on ? 'settings-toggle__switch--on' : ''}`}
      onClick={onToggle}
    />
  );
}
