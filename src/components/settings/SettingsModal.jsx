import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import PathSelector from './PathSelector';
import LanguageSelector from './LanguageSelector';
import LogViewer from '../common/LogViewer';
import useProtonDownload from '../../hooks/useProtonDownload';
import useIntegrityCheck from '../../hooks/useIntegrityCheck';
import { formatSize, formatSpeed, formatPercent } from '../../utils/format';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { useTranslation } from '../../i18n';
import './SettingsModal.css';

export default function SettingsModal({ settings, initialTab, systemCheck, onRefreshSystemCheck, onSave, onClose }) {
  const { t } = useTranslation();
  const [form, setForm] = useState(null);
  const [activeTab, setActiveTab] = useState(initialTab || 'paths');
  const [speedUnit, setSpeedUnit] = useState('MB/s');
  const [releases, setReleases] = useState([]);
  const [installedProtons, setInstalledProtons] = useState([]);
  const [releasesLoading, setReleasesLoading] = useState(false);
  const [autostart, setAutostart] = useState(false);
  const [showLog, setShowLog] = useState(false);
  const [repairing, setRepairing] = useState(false);
  const [uninstalling, setUninstalling] = useState(false);
  const [debugCopied, setDebugCopied] = useState(false);
  const [latestVersion, setLatestVersion] = useState('');
  const [recommendedTag, setRecommendedTag] = useState('');
  const [prefixInfo, setPrefixInfo] = useState(null);
  const [prefixBusy, setPrefixBusy] = useState(null);
  const [prefixMsg, setPrefixMsg] = useState(null);

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

  const {
    checking: integrityChecking,
    progress: integrityProgress,
    result: integrityResult,
    error: integrityError,
    start: startIntegrity,
    cancel: cancelIntegrity,
  } = useIntegrityCheck();

  const handleIntegrity = () => {
    if (integrityChecking) return;
    const installed = form?.installed_version || '—';
    const latest = latestVersion || '—';
    if (!confirm(t('settings.integrity.confirm', { installed, latest }))) return;
    startIntegrity();
  };

  const integrityPct = integrityProgress
    ? integrityProgress.stage === 'downloading'
      ? integrityProgress.bytes_total > 0
        ? (integrityProgress.bytes_done / integrityProgress.bytes_total) * 100
        : 0
      : integrityProgress.total_files > 0
        ? (integrityProgress.files_done / integrityProgress.total_files) * 100
        : 0
    : 0;

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
      invoke('recommended_proton_tag').then(setRecommendedTag).catch(() => {});
      invoke('get_prefix_info').then(setPrefixInfo).catch(() => {});
    }
  }, [activeTab, fetchReleases, fetchInstalled]);

  // The integrity check can only compare against the latest version's manifest
  // (the API exposes no older one), so surface the latest version up front.
  useEffect(() => {
    if (activeTab === 'game') {
      invoke('get_game_version')
        .then((r) => setLatestVersion(r?.version || ''))
        .catch(() => {});
    }
  }, [activeTab]);

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

  const runPrefixAction = async (kind, action, doneMsg) => {
    if (prefixBusy) return;
    setPrefixBusy(kind);
    setPrefixMsg(null);
    try {
      const result = await action();
      if (doneMsg) setPrefixMsg({ ok: true, text: doneMsg(result) });
      invoke('get_prefix_info').then(setPrefixInfo).catch(() => {});
    } catch (e) {
      setPrefixMsg({ ok: false, text: typeof e === 'string' ? e : e.message || String(e) });
    } finally {
      setPrefixBusy(null);
    }
  };

  const handleClearShaderCache = () =>
    runPrefixAction('cache', () => invoke('clear_shader_cache'), (r) =>
      t('settings.prefixTools.cacheDone', { files: r.files_removed, size: formatSize(r.bytes_freed) })
    );

  const handleBackupPrefix = async () => {
    if (prefixBusy) return;
    const stamp = new Date().toISOString().slice(0, 10);
    const dest = await saveDialog({
      defaultPath: `endfield-prefix-${stamp}.tar.gz`,
      filters: [{ name: 'Prefix backup', extensions: ['tar.gz', 'gz'] }],
    });
    if (!dest) return;
    await runPrefixAction('backup', () => invoke('backup_prefix', { dest }), () =>
      t('settings.prefixTools.backupDone')
    );
  };

  const handleRestorePrefix = async () => {
    if (prefixBusy) return;
    const archive = await openDialog({
      filters: [{ name: 'Prefix backup', extensions: ['gz'] }],
    });
    if (!archive) return;
    if (!confirm(t('settings.prefixTools.restoreConfirm'))) return;
    await runPrefixAction('restore', () => invoke('restore_prefix', { archive }), () =>
      t('settings.prefixTools.restoreDone')
    );
  };

  const handleResetPrefix = async () => {
    if (prefixBusy) return;
    if (!confirm(t('settings.prefixTools.resetConfirm'))) return;
    await runPrefixAction('reset', () => invoke('reset_prefix'), () =>
      t('settings.prefixTools.resetDone')
    );
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

        <div className="settings-modal__body" key={activeTab}>
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
                <span className="settings-modal__label">{t('settings.prefixTools.title')}</span>
                <span className="settings-modal__hint">
                  {prefixInfo?.exists
                    ? prefixInfo.path
                    : t('settings.prefixTools.noPrefix')}
                </span>
                <div className="settings-prefix-tools">
                  <button
                    className="settings-modal__btn settings-modal__btn--secondary"
                    onClick={() => runPrefixAction('open', () => invoke('open_prefix_folder'))}
                    disabled={!!prefixBusy || !prefixInfo?.exists}
                  >
                    {t('settings.prefixTools.open')}
                  </button>
                  <button
                    className="settings-modal__btn settings-modal__btn--secondary"
                    onClick={() => runPrefixAction('winecfg', () => invoke('run_prefix_tool', { tool: 'winecfg' }))}
                    disabled={!!prefixBusy}
                  >
                    {t('settings.prefixTools.winecfg')}
                  </button>
                  <button
                    className="settings-modal__btn settings-modal__btn--secondary"
                    onClick={handleClearShaderCache}
                    disabled={!!prefixBusy}
                  >
                    {prefixBusy === 'cache' ? t('common.loading') : t('settings.prefixTools.clearCache')}
                  </button>
                  <button
                    className="settings-modal__btn settings-modal__btn--secondary"
                    onClick={handleBackupPrefix}
                    disabled={!!prefixBusy || !prefixInfo?.exists}
                  >
                    {prefixBusy === 'backup' ? t('common.loading') : t('settings.prefixTools.backup')}
                  </button>
                  <button
                    className="settings-modal__btn settings-modal__btn--secondary"
                    onClick={handleRestorePrefix}
                    disabled={!!prefixBusy}
                  >
                    {prefixBusy === 'restore' ? t('common.loading') : t('settings.prefixTools.restore')}
                  </button>
                  <button
                    className="settings-modal__btn settings-modal__btn--danger"
                    onClick={handleResetPrefix}
                    disabled={!!prefixBusy || !prefixInfo?.exists}
                  >
                    {prefixBusy === 'reset' ? t('common.loading') : t('settings.prefixTools.reset')}
                  </button>
                </div>
                {prefixMsg && (
                  <span className={prefixMsg.ok ? 'settings-modal__hint' : 'settings-proton__error'}>
                    {prefixMsg.text}
                  </span>
                )}
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
                      const recommended = recommendedTag && r.tag_name === recommendedTag;
                      const majorMatch = r.tag_name.match(/dwproton-(\d+)/);
                      const risky = majorMatch ? parseInt(majorMatch[1], 10) >= 11 : false;
                      return (
                        <div key={r.tag_name} className={`settings-proton__item ${active ? 'settings-proton__item--active' : ''}`}>
                          <div className="settings-proton__item-info">
                            <span className="settings-proton__item-name">
                              {r.tag_name}
                              {recommended && <span className="settings-proton__badge settings-proton__badge--recommended">{t('settings.badgeRecommended')}</span>}
                              {active && <span className="settings-proton__badge settings-proton__badge--active">{t('settings.badgeActive')}</span>}
                              {installed && !active && <span className="settings-proton__badge settings-proton__badge--installed">{t('settings.badgeInstalled')}</span>}
                            </span>
                            <span className="settings-proton__item-meta">
                              {r.published_at || 'unknown'} &middot; {formatSize(r.size)}
                            </span>
                            {risky && <span className="settings-proton__warn">{t('settings.protonRiskyWarn')}</span>}
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
                  onClick={() => handleChange('use_gamescope', !form.use_gamescope)}
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
                        onChange={(e) => handleChange('gamescope_mode', e.target.value)}
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
                        onChange={(e) => handleChange('gamescope_upscaler', e.target.value)}
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
                        onChange={(e) => handleChange('gamescope_render_res', e.target.value)}
                        placeholder={t('settings.gamescope.resNative')}
                        spellCheck={false}
                      />
                    </div>
                    <div className="settings-gamescope__field">
                      <span className="settings-modal__label">{t('settings.gamescope.outputRes')}</span>
                      <input
                        value={form.gamescope_output_res || ''}
                        onChange={(e) => handleChange('gamescope_output_res', e.target.value)}
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
                        onChange={(e) => handleChange('gamescope_fps_limit', parseInt(e.target.value, 10) || 0)}
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
                      onClick={() => handleChange('gamescope_hdr', !form.gamescope_hdr)}
                    />
                  </div>

                  <div className="settings-gamescope__field">
                    <span className="settings-modal__label">{t('settings.gamescope.extraArgs')}</span>
                    <input
                      value={form.gamescope_extra_args || ''}
                      onChange={(e) => handleChange('gamescope_extra_args', e.target.value)}
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
                  <span className="settings-action-row__name">{t('settings.integrity.name')}</span>
                  <span className="settings-action-row__desc">{t('settings.integrity.desc')}</span>
                </div>
                <button
                  className="settings-modal__btn settings-modal__btn--secondary"
                  onClick={handleIntegrity}
                  disabled={integrityChecking}
                >
                  {integrityChecking ? t('common.loading') : t('settings.integrity.button')}
                </button>
              </div>

              {!integrityChecking && latestVersion && form.installed_version && form.installed_version !== latestVersion && (
                <div className="settings-integrity-warning">
                  {t('settings.integrity.behindNote', { installed: form.installed_version, latest: latestVersion })}
                </div>
              )}
              {!integrityChecking && latestVersion && (
                <span className="settings-modal__hint">
                  {t('settings.integrity.compareNote', { latest: latestVersion })}
                </span>
              )}

              {integrityChecking && integrityProgress && (
                <div className="settings-proton__progress">
                  <div className="settings-proton__progress-info">
                    <span>{t(`settings.integrity.${integrityProgress.stage}`)}</span>
                    <span>
                      {integrityProgress.stage === 'downloading'
                        ? `${formatPercent(integrityProgress.bytes_done, integrityProgress.bytes_total)}${integrityProgress.speed_bps > 0 ? ` • ${formatSpeed(integrityProgress.speed_bps)}` : ''}`
                        : integrityProgress.total_files > 0
                          ? `${integrityProgress.files_done} / ${integrityProgress.total_files}`
                          : ''}
                    </span>
                  </div>
                  <div className="settings-proton__progress-bar">
                    <div
                      className="settings-proton__progress-fill"
                      style={{ width: `${integrityPct}%` }}
                    />
                  </div>
                  {integrityProgress.stage === 'downloading' && integrityProgress.bytes_total > 0 && (
                    <div className="settings-proton__progress-detail">
                      {formatSize(integrityProgress.bytes_done)} / {formatSize(integrityProgress.bytes_total)}
                    </div>
                  )}
                  <button
                    className="settings-modal__btn settings-modal__btn--cancel"
                    onClick={cancelIntegrity}
                  >
                    {t('common.cancel')}
                  </button>
                </div>
              )}

              {!integrityChecking && integrityResult && (
                <div className="settings-modal__hint">
                  {integrityResult.repaired > 0
                    ? t('settings.integrity.resultRepaired', { checked: integrityResult.checked, repaired: integrityResult.repaired })
                    : t('settings.integrity.resultOk', { checked: integrityResult.checked })}
                </div>
              )}

              {integrityError && (
                <div className="settings-proton__error">{integrityError}</div>
              )}

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
