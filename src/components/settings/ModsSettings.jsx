import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useTranslation } from '../../i18n';
import './ModsSettings.css';

// Where mods for the game are published. Opened in the user's browser, not
// in-app: this is a community site, not part of the launcher.
const CATALOG_URL = 'https://gamebanana.com/games/21842';

// The mods tab, laid out as the three things a user has to do in order:
// install the loader, drop mods in a folder, launch with them. Each step
// shows whether it is done, so the state of the setup is readable at a glance
// instead of hidden behind a toggle that may or may not have taken effect.
export default function ModsSettings({ form, onChange }) {
  const { t } = useTranslation();
  const [status, setStatus] = useState(null);
  const [busy, setBusy] = useState(null);
  const [msg, setMsg] = useState(null);

  const refresh = useCallback(async () => {
    try {
      setStatus(await invoke('get_mods_status'));
    } catch {
      setStatus(null);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const handleInstall = async () => {
    setBusy('install');
    setMsg(null);
    try {
      const res = await invoke('install_mod_loader');
      setMsg({ ok: true, text: t('settings.mods.installed', { version: res.version }) });
      await refresh();
    } catch (e) {
      setMsg({ ok: false, text: typeof e === 'string' ? e : e.message || t('settings.mods.installFailed') });
    } finally {
      setBusy(null);
    }
  };

  const handleUninstall = async () => {
    setBusy('uninstall');
    setMsg(null);
    try {
      await invoke('uninstall_mod_loader');
      setMsg({ ok: true, text: t('settings.mods.uninstalled') });
      await refresh();
    } catch (e) {
      setMsg({ ok: false, text: typeof e === 'string' ? e : e.message || e });
    } finally {
      setBusy(null);
    }
  };

  const handleOpenFolder = async () => {
    setMsg(null);
    try {
      await invoke('open_mods_folder');
      await refresh();
    } catch (e) {
      setMsg({ ok: false, text: typeof e === 'string' ? e : e.message || e });
    }
  };

  const loaderReady = !!status?.loader_installed && !!status?.loader_configured;
  const hasMods = (status?.mod_count || 0) > 0;

  if (status?.game_dir_missing) {
    return (
      <div className="settings-modal__section">
        <span className="settings-modal__hint">{t('settings.mods.noGame')}</span>
      </div>
    );
  }

  return (
    <>
      <div className="mods__intro">
        <p className="mods__intro-text">{t('settings.mods.intro')}</p>
        <p className="mods__intro-note">{t('settings.mods.introNote')}</p>
      </div>

      <ol className="mods__steps">
        <Step n={1} done={loaderReady} title={t('settings.mods.step1.title')} desc={t('settings.mods.step1.desc')}>
          {loaderReady ? (
            <div className="mods__row">
              <span className="mods__ok">{t('settings.mods.step1.ready')}</span>
              <button
                className="settings-modal__btn settings-modal__btn--danger"
                onClick={handleUninstall}
                disabled={!!busy}
              >
                {busy === 'uninstall' ? t('common.loading') : t('settings.mods.step1.remove')}
              </button>
            </div>
          ) : (
            <div className="mods__row">
              <button
                className="settings-modal__btn settings-modal__btn--save"
                onClick={handleInstall}
                disabled={!!busy}
              >
                {busy === 'install' ? t('settings.mods.step1.installing') : t('settings.mods.step1.install')}
              </button>
              <span className="mods__src">{t('settings.mods.step1.source')}</span>
            </div>
          )}
        </Step>

        <Step n={2} done={hasMods} title={t('settings.mods.step2.title')} desc={t('settings.mods.step2.desc')}>
          <div className="mods__row">
            <button className="settings-modal__btn settings-modal__btn--secondary" onClick={handleOpenFolder}>
              {t('settings.mods.step2.open')}
            </button>
            <button className="settings-modal__btn settings-modal__btn--secondary" onClick={() => openUrl(CATALOG_URL)}>
              {t('settings.mods.step2.catalog')}
            </button>
            <span className={hasMods ? 'mods__ok' : 'mods__src'}>
              {hasMods
                ? t('settings.mods.step2.count', { count: status.mod_count })
                : t('settings.mods.step2.empty')}
            </span>
          </div>
        </Step>

        <Step n={3} done={!!form.mods_enabled} title={t('settings.mods.step3.title')} desc={t('settings.mods.step3.desc')}>
          <div className="settings-toggle mods__toggle">
            <div className="settings-toggle__info">
              <span className="settings-toggle__name">{t('settings.mods.enable.name')}</span>
              <span className="settings-toggle__desc">{t('settings.mods.enable.desc')}</span>
            </div>
            <button
              className={`settings-toggle__switch ${form.mods_enabled ? 'settings-toggle__switch--on' : ''}`}
              onClick={() => onChange('mods_enabled', !form.mods_enabled)}
            />
          </div>
        </Step>
      </ol>

      {msg && (
        <span className={`settings-modal__msg ${msg.ok ? 'settings-modal__msg--ok' : 'settings-modal__msg--err'}`}>
          {msg.text}
        </span>
      )}

      <div className="mods__warnings">
        <Warning tone="perf" title={t('settings.mods.warnPerf.title')} text={t('settings.mods.warnPerf.text')} />
        <Warning tone="risk" title={t('settings.mods.warnRisk.title')} text={t('settings.mods.warnRisk.text')} />
      </div>
    </>
  );
}

function Step({ n, done, title, desc, children }) {
  return (
    <li className={`mods__step ${done ? 'mods__step--done' : ''}`}>
      <span className="mods__step-num">{done ? '✓' : n}</span>
      <div className="mods__step-body">
        <span className="mods__step-title">{title}</span>
        <span className="mods__step-desc">{desc}</span>
        {children}
      </div>
    </li>
  );
}

function Warning({ tone, title, text }) {
  return (
    <div className={`mods__warn mods__warn--${tone}`}>
      <span className="mods__warn-title">{title}</span>
      <span className="mods__warn-text">{text}</span>
    </div>
  );
}
