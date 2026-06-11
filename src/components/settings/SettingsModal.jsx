import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import PathSelector from './PathSelector';
import LanguageSelector from './LanguageSelector';
import LogViewer from '../common/LogViewer';
import useProtonDownload from '../../hooks/useProtonDownload';
import { formatSize, formatSpeed, formatPercent } from '../../utils/format';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { useTranslation } from '../../i18n';
import './SettingsModal.css';

export default function SettingsModal({ settings, systemCheck, onRefreshSystemCheck, onSave, onClose }) {
  const { t } = useTranslation();
  const [form, setForm] = useState(null);
  const [activeTab, setActiveTab] = useState('paths');
  const [speedUnit, setSpeedUnit] = useState('MB/s');
  const [releases, setReleases] = useState([]);
  const [installedProtons, setInstalledProtons] = useState([]);
  const [releasesLoading, setReleasesLoading] = useState(false);
  const [autostart, setAutostart] = useState(false);
  const [showLog, setShowLog] = useState(false);
  const [repairing, setRepairing] = useState(false);
  const [uninstalling, setUninstalling] = useState(false);
  const [debugCopied, setDebugCopied] = useState(false);

  const TABS = [
    { id: 'paths', label: t('settings.tab.paths') },
    { id: 'proton', label: t('settings.tab.proton') },
    { id: 'launch', label: t('settings.tab.launch') },
    { id: 'downloads', label: t('settings.tab.downloads') },
    { id: 'game', label: t('settings.tab.game') },
  ];

  const fetchInstalled = useCallback(async () => {
    try {
      const list = await invoke('list_installed_protons');
      setInstalledProtons(list);
    } catch (e) {
      console.error('Failed to list installed protons:', e);
    }
  }, []);

  const onProtonComplete = useCallback((payload) => {
    if (payload?.proton_dir) {
      setForm((prev) => prev ? { ...prev, proton_dir: payload.proton_dir } : prev);
    }
    fetchInstalled();
    if (onRefreshSystemCheck) onRefreshSystemCheck();
  }, [fetchInstalled, onRefreshSystemCheck]);

  const { downloading: protonDownloading, progress: protonProgress, error: protonError, startDownload: startProtonDownload, cancelDownload: cancelProtonDownload } =
    useProtonDownload(onProtonComplete);

  useEffect(() => {
    if (settings) {
      setForm({ ...settings });
      if (settings.download_speed_limit > 0 && settings.download_speed_limit < 1024 * 1024) {
        setSpeedUnit('KB/s');
      }
    }
  }, [settings]);

  const fetchReleases = useCallback(async () => {
    setReleasesLoading(true);
    try {
      const list = await invoke('list_dwproton_releases');
      setReleases(list);
    } catch (e) {
      console.error('Failed to list proton releases:', e);
    } finally {
      setReleasesLoading(false);
    }
  }, []);

  useEffect(() => {
    if (activeTab === 'launch') {
      isEnabled().then(setAutostart).catch(console.error);
    }
  }, [activeTab]);

  useEffect(() => {
    if (activeTab === 'proton') {
      fetchReleases();
      fetchInstalled();
    }
  }, [activeTab, fetchReleases, fetchInstalled]);

  if (!form) return null;

  const handleChange = (key, value) => {
    setForm((prev) => ({ ...prev, [key]: value }));
  };

  const handleAutostartToggle = async () => {
    try {
      if (autostart) {
        await disable();
      } else {
        await enable();
      }
      const status = await isEnabled();
      setAutostart(status);
    } catch (e) {
      console.error('Failed to toggle autostart:', e);
    }
  };

  const handleSave = () => {
    onSave(form);
    onClose();
  };

  const handleProtonDownload = (release) => {
    startProtonDownload(release || undefined);
  };

  const handleSetActiveProton = async (path) => {
    try {
      await invoke('set_active_proton', { path });
      setForm((prev) => prev ? { ...prev, proton_dir: path } : prev);
      if (onRefreshSystemCheck) onRefreshSystemCheck();
    } catch (e) {
      console.error('Failed to set active proton:', e);
    }
  };

  const handleRepair = async () => {
    if (repairing) return;
    if (!confirm(t('settings.repair.confirm'))) return;
    setRepairing(true);
    try {
      await invoke('repair_game');
      onClose();
    } catch (e) {
      console.error('Failed to repair:', e);
    } finally {
      setRepairing(false);
    }
  };

  const handleUninstall = async () => {
    if (uninstalling) return;
    if (!confirm(t('settings.uninstall.confirm'))) return;
    setUninstalling(true);
    try {
      await invoke('uninstall_game');
      // Reload so every view picks up the now-empty installation state.
      window.location.reload();
    } catch (e) {
      console.error('Failed to uninstall:', e);
      alert(typeof e === 'string' ? e : e.message || 'Uninstall failed');
    } finally {
      setUninstalling(false);
    }
  };

  const handleCopyDebugInfo = async () => {
    try {
      const info = await invoke('get_debug_info');
      await navigator.clipboard.writeText(info);
      setDebugCopied(true);
      setTimeout(() => setDebugCopied(false), 2000);
    } catch (e) {
      console.error('Failed to copy debug info:', e);
    }
  };

  const findInstalled = (tagName) => {
    return installedProtons.find((p) => p.name === tagName || p.name.startsWith(tagName + '-'));
  };

  const isInstalled = (tagName) => {
    return !!findInstalled(tagName);
  };

  const getInstalledPath = (tagName) => {
    return findInstalled(tagName)?.path || null;
  };

  const isActive = (tagName) => {
    if (!form?.proton_dir) return false;
    const installed = findInstalled(tagName);
    return installed ? form.proton_dir.startsWith(installed.path) : false;
  };

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-modal" onClick={(e) => e.stopPropagation()}>
        <div className="settings-modal__header">
          <span className="settings-modal__title">{t('settings.title')}</span>
          <button className="settings-modal__close" onClick={onClose}>
            {'✕'}
          </button>
        </div>

        <div className="settings-modal__tabs">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              className={`settings-modal__tab ${activeTab === tab.id ? 'settings-modal__tab--active' : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className="settings-modal__body">
          {activeTab === 'paths' && (
            <>
              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.gameDir')}</span>
                <PathSelector
                  value={form.game_dir}
                  onChange={(v) => handleChange('game_dir', v)}
                />
              </div>

              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.downloadDir')}</span>
                <PathSelector
                  value={form.download_dir}
                  onChange={(v) => handleChange('download_dir', v)}
                />
                <span className="settings-modal__hint">{t('settings.downloadDirHint')}</span>
              </div>

              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.language')}</span>
                <LanguageSelector
                  value={form.language}
                  onChange={(v) => handleChange('language', v)}
                />
              </div>
            </>
          )}

          {activeTab === 'proton' && (
            <>
              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.activeProton')}</span>
                <PathSelector
                  value={form.proton_dir}
                  onChange={(v) => handleChange('proton_dir', v)}
                />
                <span className="settings-modal__hint">
                  {t('settings.statusPrefix')} {systemCheck?.has_proton ? t('common.ready') : t('common.notFound')}
                </span>
              </div>

              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.prefixDir')}</span>
                <PathSelector
                  value={form.proton_prefix_dir}
                  onChange={(v) => handleChange('proton_prefix_dir', v)}
                />
                <span className="settings-modal__hint">{t('settings.prefixDirHint')}</span>
              </div>

              <div className="settings-modal__section">
                <div className="settings-proton__header">
                  <span className="settings-modal__label">{t('settings.availableVersions')}</span>
                  <button
                    className="settings-proton__refresh-btn"
                    onClick={() => { fetchReleases(); fetchInstalled(); }}
                    disabled={releasesLoading}
                  >
                    {releasesLoading ? t('common.loading') : t('common.refresh')}
                  </button>
                </div>

                {releasesLoading && releases.length === 0 ? (
                  <span className="settings-modal__hint">{t('settings.loadingReleases')}</span>
                ) : releases.length > 0 ? (
                  <div className="settings-proton__list">
                    {releases.map((r) => {
                      const installed = isInstalled(r.tag_name);
                      const active = isActive(r.tag_name);
                      return (
                        <div key={r.tag_name} className={`settings-proton__item ${active ? 'settings-proton__item--active' : ''}`}>
                          <div className="settings-proton__item-info">
                            <span className="settings-proton__item-name">
                              {r.tag_name}
                              {active && <span className="settings-proton__badge settings-proton__badge--active">{t('settings.badgeActive')}</span>}
                              {installed && !active && <span className="settings-proton__badge settings-proton__badge--installed">{t('settings.badgeInstalled')}</span>}
                            </span>
                            <span className="settings-proton__item-meta">
                              {r.published_at || 'unknown'} &middot; {formatSize(r.size)}
                            </span>
                          </div>
                          <div className="settings-proton__item-actions">
                            {installed ? (
                              !active && (
                                <button
                                  className="settings-proton__use-btn"
                                  onClick={() => handleSetActiveProton(getInstalledPath(r.tag_name))}
                                >
                                  {t('settings.use')}
                                </button>
                              )
                            ) : (
                              <button
                                className="settings-proton__dl-btn"
                                onClick={() => handleProtonDownload(r)}
                                disabled={protonDownloading}
                              >
                                {t('settings.download')}
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <span className="settings-modal__hint">{t('settings.noReleases')}</span>
                )}
              </div>

              {protonDownloading && protonProgress && (
                <div className="settings-proton__progress">
                  <div className="settings-proton__progress-info">
                    <span>{protonProgress.stage === 'extracting' ? t('protonPrompt.extracting') : t('protonPrompt.downloading')}</span>
                    <span>
                      {formatPercent(protonProgress.bytes_downloaded, protonProgress.bytes_total)}
                      {protonProgress.speed_bps > 0 && ` • ${formatSpeed(protonProgress.speed_bps)}`}
                    </span>
                  </div>
                  <div className="settings-proton__progress-bar">
                    <div
                      className="settings-proton__progress-fill"
                      style={{
                        width: protonProgress.bytes_total > 0
                          ? `${(protonProgress.bytes_downloaded / protonProgress.bytes_total) * 100}%`
                          : '0%',
                      }}
                    />
                  </div>
                  <div className="settings-proton__progress-detail">
                    {formatSize(protonProgress.bytes_downloaded)} / {formatSize(protonProgress.bytes_total)}
                  </div>
                  <button
                    className="settings-modal__btn settings-modal__btn--cancel"
                    onClick={cancelProtonDownload}
                  >
                    {t('common.cancel')}
                  </button>
                </div>
              )}

              {protonError && (
                <div className="settings-proton__error">{protonError}</div>
              )}
            </>
          )}

          {activeTab === 'launch' && (
            <>
              <div className="settings-toggle">
                <div className="settings-toggle__info">
                  <span className="settings-toggle__name">{t('settings.autostart.name')}</span>
                  <span className="settings-toggle__desc">
                    {t('settings.autostart.desc')}
                  </span>
                </div>
                <button
                  className={`settings-toggle__switch ${autostart ? 'settings-toggle__switch--on' : ''}`}
                  onClick={handleAutostartToggle}
                />
              </div>

              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.afterLaunch')}</span>
                <select
                  className="settings-proton__select"
                  value={form.on_launch_action || 'hide'}
                  onChange={(e) => handleChange('on_launch_action', e.target.value)}
                >
                  <option value="hide">{t('settings.afterLaunchHide')}</option>
                  <option value="close">{t('settings.afterLaunchClose')}</option>
                  <option value="nothing">{t('settings.afterLaunchKeep')}</option>
                </select>
              </div>

              <div className="settings-toggle">
                <div className="settings-toggle__info">
                  <span className="settings-toggle__name">{t('settings.vulkan.name')}</span>
                  <span className="settings-toggle__desc">
                    {t('settings.vulkan.desc')}
                  </span>
                </div>
                <button
                  className={`settings-toggle__switch ${form.use_native_vulkan ? 'settings-toggle__switch--on' : ''}`}
                  onClick={() => handleChange('use_native_vulkan', !form.use_native_vulkan)}
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
                  onClick={() => handleChange('use_wayland', !form.use_wayland)}
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
                  onClick={() => handleChange('use_gamemode', !form.use_gamemode)}
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
                  onClick={() => handleChange('use_dxvk_async', !form.use_dxvk_async)}
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
                  onClick={() => handleChange('disable_fsync', !form.disable_fsync)}
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
                  onClick={() => handleChange('disable_esync', !form.disable_esync)}
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
                  onClick={() => handleChange('use_mangohud', !form.use_mangohud)}
                />
              </div>

              <div className="settings-toggle">
                <div className="settings-toggle__info">
                  <span className="settings-toggle__name">{t('settings.prime.name')}</span>
                  <span className="settings-toggle__desc">
                    {t('settings.prime.desc')}
                  </span>
                </div>
                <button
                  className={`settings-toggle__switch ${form.use_prime_offload ? 'settings-toggle__switch--on' : ''}`}
                  onClick={() => handleChange('use_prime_offload', !form.use_prime_offload)}
                />
              </div>

              <div className="settings-toggle">
                <div className="settings-toggle__info">
                  <span className="settings-toggle__name">{t('settings.discord.name')}</span>
                  <span className="settings-toggle__desc">
                    {t('settings.discord.desc')}
                  </span>
                </div>
                <button
                  className={`settings-toggle__switch ${form.use_discord_rpc ? 'settings-toggle__switch--on' : ''}`}
                  onClick={() => handleChange('use_discord_rpc', !form.use_discord_rpc)}
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
                  onClick={() => handleChange('use_canonical_hole', !form.use_canonical_hole)}
                />
              </div>

              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.launchArgs')}</span>
                <input
                  value={form.custom_launch_args}
                  onChange={(e) => handleChange('custom_launch_args', e.target.value)}
                  placeholder={t('settings.launchArgsPlaceholder')}
                  spellCheck={false}
                />
              </div>

              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.envVars')}</span>
                <textarea
                  className="settings-textarea"
                  value={form.custom_env_vars}
                  onChange={(e) => handleChange('custom_env_vars', e.target.value)}
                  placeholder={"# KEY=VALUE\nDXVK_HUD=fps\nMESA_SHADER_CACHE=1"}
                  spellCheck={false}
                />
                <span className="settings-modal__hint">
                  {t('settings.envVarsHint')}
                </span>
              </div>
            </>
          )}

          {activeTab === 'downloads' && (
            <>
              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.maxConcurrent')}</span>
                <select
                  className="settings-proton__select"
                  value={form.download_max_concurrent || 4}
                  onChange={(e) => handleChange('download_max_concurrent', parseInt(e.target.value, 10))}
                >
                  {[1, 2, 3, 4, 5, 6, 7, 8].map((n) => (
                    <option key={n} value={n}>{n}</option>
                  ))}
                </select>
              </div>

              <div className="settings-modal__section">
                <span className="settings-modal__label">{t('settings.speedLimit')}</span>
                <div className="settings-speed-limit">
                  <input
                    type="number"
                    className="settings-speed-limit__input"
                    min="0"
                    value={
                      form.download_speed_limit === 0
                        ? ''
                        : speedUnit === 'MB/s'
                          ? Math.round(form.download_speed_limit / (1024 * 1024))
                          : Math.round(form.download_speed_limit / 1024)
                    }
                    onChange={(e) => {
                      const val = parseInt(e.target.value, 10) || 0;
                      const bytes = speedUnit === 'MB/s' ? val * 1024 * 1024 : val * 1024;
                      handleChange('download_speed_limit', bytes);
                    }}
                    placeholder="0"
                  />
                  <select
                    className="settings-speed-limit__unit"
                    value={speedUnit}
                    onChange={(e) => {
                      const newUnit = e.target.value;
                      const oldUnit = speedUnit;
                      setSpeedUnit(newUnit);
                      if (form.download_speed_limit > 0) {
                        let val;
                        if (oldUnit === 'MB/s' && newUnit === 'KB/s') {
                          val = Math.round(form.download_speed_limit / 1024) * 1024;
                        } else if (oldUnit === 'KB/s' && newUnit === 'MB/s') {
                          val = Math.round(form.download_speed_limit / (1024 * 1024)) * 1024 * 1024;
                        } else {
                          val = form.download_speed_limit;
                        }
                        handleChange('download_speed_limit', val);
                      }
                    }}
                  >
                    <option value="MB/s">MB/s</option>
                    <option value="KB/s">KB/s</option>
                  </select>
                </div>
                <span className="settings-modal__hint">{t('settings.speedLimitHint')}</span>
              </div>
            </>
          )}

          {activeTab === 'game' && (
            <>
              <div className="settings-action-row">
                <div className="settings-action-row__info">
                  <span className="settings-action-row__name">{t('settings.repair.name')}</span>
                  <span className="settings-action-row__desc">{t('settings.repair.desc')}</span>
                </div>
                <button
                  className="settings-modal__btn settings-modal__btn--secondary"
                  onClick={handleRepair}
                  disabled={repairing}
                >
                  {repairing ? t('common.loading') : t('settings.repair.button')}
                </button>
              </div>

              <div className="settings-action-row">
                <div className="settings-action-row__info">
                  <span className="settings-action-row__name">{t('settings.viewLog.name')}</span>
                  <span className="settings-action-row__desc">{t('settings.viewLog.desc')}</span>
                </div>
                <button
                  className="settings-modal__btn settings-modal__btn--secondary"
                  onClick={() => setShowLog(true)}
                >
                  {t('settings.viewLog.button')}
                </button>
              </div>

              <div className="settings-action-row">
                <div className="settings-action-row__info">
                  <span className="settings-action-row__name">{t('settings.debugInfo.name')}</span>
                  <span className="settings-action-row__desc">{t('settings.debugInfo.desc')}</span>
                </div>
                <button
                  className="settings-modal__btn settings-modal__btn--secondary"
                  onClick={handleCopyDebugInfo}
                >
                  {debugCopied ? t('settings.debugInfo.copied') : t('settings.debugInfo.button')}
                </button>
              </div>

              <div className="settings-action-row">
                <div className="settings-action-row__info">
                  <span className="settings-action-row__name">{t('settings.uninstall.name')}</span>
                  <span className="settings-action-row__desc">{t('settings.uninstall.desc')}</span>
                </div>
                <button
                  className="settings-modal__btn settings-modal__btn--danger"
                  onClick={handleUninstall}
                  disabled={uninstalling}
                >
                  {uninstalling ? t('common.loading') : t('settings.uninstall.button')}
                </button>
              </div>
            </>
          )}
        </div>

        <div className="settings-modal__footer">
          <button className="settings-modal__btn settings-modal__btn--cancel" onClick={onClose}>
            {t('common.cancel')}
          </button>
          <button className="settings-modal__btn settings-modal__btn--save" onClick={handleSave}>
            {t('common.save')}
          </button>
        </div>
      </div>

      {showLog && <LogViewer onClose={() => setShowLog(false)} />}
    </div>
  );
}
