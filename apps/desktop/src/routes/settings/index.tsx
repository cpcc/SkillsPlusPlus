import { useState } from "react";
import { useAppInfo } from "../../hooks/use-app-info";
import { useTheme, type ThemePreference } from "../../hooks/use-theme";
import { useUpdateCheck } from "../../hooks/use-update-check";
import { useMirrorConfig, useSetMirrorConfig, useMirrorHealth } from "../../hooks/use-mirror";
import {
  useExportSnapshot, useImportSnapshot,
  useSyncConfig, useSetSyncConfig, DEFAULT_SYNC_CONFIG,
  useSyncStatus, useSyncNow, useTestWebdavConnection,
} from "../../hooks/use-sync";
import { useToast } from "../../components/ui/toast";
import { ipc } from "../../lib/ipc";
import type { SyncConfig } from "@skills-pp/shared";
import {
  Info, Database, Monitor, SunMoon, RefreshCw, Download, Globe, Plus, Trash2,
  Activity, Upload, FileJson, Cloud, CloudUpload, Wifi, CheckCircle2, AlertCircle, CloudOff,
} from "lucide-react";

const THEME_OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
  { value: "system", label: "跟随系统" },
];

export default function SettingsPage() {
  const { data, isLoading, error } = useAppInfo();
  const { preference, setPreference } = useTheme();
  const updateQuery = useUpdateCheck();
  const { data: mirrorConfig } = useMirrorConfig();
  const setMirrorConfigMutation = useSetMirrorConfig();
  const { data: mirrorHealth, refetch: refetchHealth, isFetching: checkingHealth } = useMirrorHealth();
  const exportMutation = useExportSnapshot();
  const importMutation = useImportSnapshot();
  const toast = useToast();

  // Phase 2: WebDAV sync
  const { data: syncConfig } = useSyncConfig();
  const setSyncConfigMutation = useSetSyncConfig();
  const { data: syncStatus, refetch: refetchSyncStatus } = useSyncStatus();
  const syncNowMutation = useSyncNow();
  const testConnectionMutation = useTestWebdavConnection();

  const handleCheckUpdate = async () => {
    try {
      const info = await updateQuery.refetch();
      const data = info.data;
      if (data?.hasUpdate) {
        toast(`发现新版本 v${data.latestVersion}`, "点击下方按钮前往 GitHub 下载");
      } else if (data) {
        toast("已是最新版本", `v${data.currentVersion}`);
      }
    } catch (e) {
      toast("检查更新失败", String(e), "error");
    }
  };

  const handleDownload = (url: string, version: string) => {
    ipc.openReleaseUrl(url);
    toast(`正在打开 GitHub Release 页面`, `v${version}`);
  };

  // 镜像配置操作
  const handleToggleMirrorEnabled = (enabled: boolean) => {
    if (!mirrorConfig) return;
    setMirrorConfigMutation.mutate({ ...mirrorConfig, enabled }, {
      onSuccess: () => toast("已保存镜像配置"),
      onError: (e) => toast("保存失败", String(e), "error"),
    });
  };

  const handleAddMirror = () => {
    if (!mirrorConfig) return;
    const newMirror = window.prompt("输入镜像前缀（如 https://gh-proxy.com）：");
    if (!newMirror?.trim()) return;
    setMirrorConfigMutation.mutate(
      { ...mirrorConfig, githubMirrors: [...mirrorConfig.githubMirrors, newMirror.trim()] },
      {
        onSuccess: () => toast("已添加镜像"),
        onError: (e) => toast("添加失败", String(e), "error"),
      }
    );
  };

  const handleRemoveMirror = (idx: number) => {
    if (!mirrorConfig) return;
    const next = mirrorConfig.githubMirrors.filter((_, i) => i !== idx);
    setMirrorConfigMutation.mutate({ ...mirrorConfig, githubMirrors: next }, {
      onSuccess: () => toast("已移除镜像"),
      onError: (e) => toast("移除失败", String(e), "error"),
    });
  };

  const handleCheckHealth = () => {
    refetchHealth();
  };

  // 同步操作
  const handleExport = () => {
    exportMutation.mutate(undefined, {
      onSuccess: () => toast("导出成功", "配置快照已下载到本地"),
      onError: (e) => toast("导出失败", String(e), "error"),
    });
  };

  const handleImportFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const text = String(reader.result);
      importMutation.mutate(text, {
        onSuccess: (result) => {
          const parts: string[] = [];
          if (result.importedSkills) parts.push(`${result.importedSkills} 个安装记录`);
          if (result.importedDirectories) parts.push(`${result.importedDirectories} 个目录`);
          if (result.updatedSources) parts.push(`${result.updatedSources} 个来源开关`);
          if (result.updatedSettings) parts.push(`${result.updatedSettings} 个设置`);
          if (result.mergedLockfileEntries) parts.push(`${result.mergedLockfileEntries} 个锁文件条目`);
          if (result.skippedSkills) parts.push(`${result.skippedSkills} 个已存在跳过`);
          const summary = parts.length > 0 ? parts.join("、") : "无变更";
          toast("导入成功", summary);
        },
        onError: (e) => toast("导入失败", String(e), "error"),
      });
    };
    reader.readAsText(file);
    // 重置 input 以便重复选同一文件
    e.target.value = "";
  };

  return (
    <div className="mx-auto max-w-[680px]">
      <h1 className="text-xl font-semibold tracking-tight text-[var(--color-text-primary)]">
        设置
      </h1>
      <p className="mt-1 text-[13px] text-[var(--color-text-secondary)]">
        来源站配置、缓存管理与日志
      </p>

      {/* Appearance */}
      <div className="mt-8">
        <h3 className="mb-3 text-[12px] font-medium uppercase tracking-[0.08em] text-[var(--color-text-tertiary)]">
          外观
        </h3>
        <div className="rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] divide-y divide-[var(--color-border-subtle)]">
          <InfoRow icon={SunMoon} label="主题">
            <div className="inline-flex rounded-[var(--radius-md)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-0.5">
              {THEME_OPTIONS.map((opt) => {
                const active = preference === opt.value;
                return (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => setPreference(opt.value)}
                    className={[
                      "px-3 py-1.5 text-[12px] font-medium rounded-[var(--radius-sm)] transition-colors",
                      active
                        ? "bg-[var(--color-accent-subtle)] text-[var(--color-accent-text)]"
                        : "text-[var(--color-text-secondary)] hover:bg-[var(--color-surface-hover)]",
                    ].join(" ")}
                  >
                    {opt.label}
                  </button>
                );
              })}
            </div>
          </InfoRow>
        </div>
      </div>

      {/* Mirror Settings */}
      <div className="mt-8">
        <h3 className="mb-3 text-[12px] font-medium uppercase tracking-[0.08em] text-[var(--color-text-tertiary)]">
          网络镜像
        </h3>
        <div className="rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] divide-y divide-[var(--color-border-subtle)]">
          <InfoRow icon={Globe} label="启用镜像">
            <button
              type="button"
              onClick={() => mirrorConfig && handleToggleMirrorEnabled(!mirrorConfig.enabled)}
              className={[
                "w-12 h-6 rounded-full relative transition-colors",
                mirrorConfig?.enabled
                  ? "bg-[var(--color-accent-subtle)]"
                  : "bg-[var(--color-border-subtle)]",
              ].join(" ")}
            >
              <span
                className={[
                  "absolute top-1 w-4 h-4 rounded-full transition-transform shadow-sm",
                  mirrorConfig?.enabled ? "left-7 translate-x-0" : "left-1",
                  mirrorConfig?.enabled
                    ? "bg-[var(--color-accent-text)]"
                    : "bg-[var(--color-text-tertiary)]",
                ].join(" ")}
              />
            </button>
          </InfoRow>

          {mirrorConfig && mirrorConfig.enabled && (
            <>
              <div className="px-5 py-3.5 space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-[12px] font-medium text-[var(--color-text-tertiary)]">镜像列表</span>
                  <div className="flex gap-2">
                    <button
                      type="button"
                      onClick={handleCheckHealth}
                      disabled={checkingHealth}
                      className="inline-flex items-center gap-1 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-2.5 py-1 text-[12px] font-medium text-[var(--color-text-primary)] transition-colors hover:bg-[var(--color-surface-hover)] disabled:opacity-50"
                    >
                      <Activity className={`h-3 w-3 ${checkingHealth ? "animate-pulse" : ""}`} />
                      测试连通性
                    </button>
                    <button
                      type="button"
                      onClick={handleAddMirror}
                      className="inline-flex items-center gap-1 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-2.5 py-1 text-[12px] font-medium text-[var(--color-text-primary)] transition-colors hover:bg-[var(--color-surface-hover)]"
                    >
                      <Plus className="h-3 w-3" />
                      添加
                    </button>
                  </div>
                </div>
                <div className="space-y-1.5">
                  {mirrorConfig.githubMirrors.map((prefix, idx) => (
                    <div
                      key={idx}
                      className="flex items-center gap-2 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-2.5"
                    >
                      <span className="min-w-0 flex-1 truncate text-[13px] text-[var(--color-text-primary)] font-mono">
                        {prefix === "" ? "<直连>" : prefix}
                      </span>
                      {mirrorHealth && mirrorHealth.length > idx && (
                        <span
                          className={[
                            "text-[11px] px-1.5 py-0.5 rounded",
                            mirrorHealth[idx].reachable
                              ? "bg-[var(--color-success-subtle)] text-[var(--color-success)]"
                              : "bg-[var(--color-danger-subtle)] text-[var(--color-danger)]",
                          ].join(" ")}
                        >
                          {mirrorHealth[idx].reachable
                            ? `${mirrorHealth[idx].latencyMs ?? "-"}ms`
                            : "不可达"}
                        </span>
                      )}
                      {idx > 0 && (
                        <button
                          type="button"
                          onClick={() => handleRemoveMirror(idx)}
                          className="shrink-0 rounded-[var(--radius-sm)] p-1 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-danger)]"
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Sync - Phase 1 + Phase 2 */}
      <div className="mt-8">
        <h3 className="mb-3 text-[12px] font-medium uppercase tracking-[0.08em] text-[var(--color-text-tertiary)]">
          跨设备同步
        </h3>
        <div className="rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] divide-y divide-[var(--color-border-subtle)]">
          {/* Phase 1: 本地导出/导入 */}
          <div className="px-5 py-4">
            <div className="flex items-center gap-2 mb-2">
              <FileJson className="h-3.5 w-3.5 text-[var(--color-text-tertiary)]" />
              <span className="text-[12px] font-medium text-[var(--color-text-secondary)]">本地文件</span>
            </div>
            <p className="text-[12px] text-[var(--color-text-tertiary)] mb-3">
              导出当前安装记录、目录配置和设置到 JSON 文件，在另一台设备导入即可同步。
            </p>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={handleExport}
                disabled={exportMutation.isPending}
                className="inline-flex items-center gap-1.5 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-3 py-1.5 text-[12px] font-medium text-[var(--color-text-primary)] transition-colors hover:bg-[var(--color-surface-hover)] disabled:opacity-50"
              >
                {exportMutation.isPending
                  ? <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                  : <Download className="h-3.5 w-3.5" />}
                导出配置
              </button>
              <label
                className="inline-flex items-center gap-1.5 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-3 py-1.5 text-[12px] font-medium text-[var(--color-text-primary)] transition-colors hover:bg-[var(--color-surface-hover)] cursor-pointer"
              >
                {importMutation.isPending
                  ? <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                  : <Upload className="h-3.5 w-3.5" />}
                导入配置
                <input
                  type="file"
                  accept=".json,application/json"
                  onChange={handleImportFile}
                  className="hidden"
                  disabled={importMutation.isPending}
                />
              </label>
            </div>
          </div>

          {/* Phase 2: WebDAV 云同步 */}
          <WebDavSyncSection
            config={syncConfig}
            status={syncStatus}
            onSaveConfig={(cfg) => {
              setSyncConfigMutation.mutate(cfg, {
                onSuccess: () => toast("已保存同步配置"),
                onError: (e) => toast("保存失败", String(e), "error"),
              });
            }}
            onTestConnection={(cfg) => {
              testConnectionMutation.mutate(cfg, {
                onSuccess: () => toast("连接成功", "WebDAV 服务器可达"),
                onError: (e) => toast("连接失败", String(e), "error"),
              });
            }}
            onSyncNow={() => {
              syncNowMutation.mutate(undefined, {
                onSuccess: (result) => {
                  const parts: string[] = [];
                  if (result.pulledSkills) parts.push(`拉取 ${result.pulledSkills} 条`);
                  if (result.pushedSkills) parts.push(`推送 ${result.pushedSkills} 条`);
                  if (result.updatedSettings) parts.push(`${result.updatedSettings} 个设置`);
                  if (result.mergedLockfileEntries) parts.push(`${result.mergedLockfileEntries} 个锁条目`);
                  if (result.conflicts.length) parts.push(`${result.conflicts.length} 个冲突`);
                  const summary = parts.length > 0 ? parts.join("、") : "无变更";
                  if (result.conflicts.length > 0) {
                    toast("同步完成（有冲突）", `${summary}。远端已删除的 skill 仍保留在本地。`, "error");
                  } else {
                    toast("同步成功", summary);
                  }
                  refetchSyncStatus();
                },
                onError: (e) => toast("同步失败", String(e), "error"),
              });
            }}
            isSyncing={syncNowMutation.isPending}
            isTesting={testConnectionMutation.isPending}
            isSaving={setSyncConfigMutation.isPending}
          />
        </div>
      </div>

      {/* App Info */}
      <div className="mt-8">
        <h3 className="mb-3 text-[12px] font-medium uppercase tracking-[0.08em] text-[var(--color-text-tertiary)]">
          应用信息
        </h3>
        <div className="rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] divide-y divide-[var(--color-border-subtle)]">
          {isLoading && (
            <div className="px-5 py-4">
              <div className="animate-pulse space-y-3">
                <div className="h-4 w-32 rounded bg-[var(--color-border-subtle)]" />
                <div className="h-4 w-24 rounded bg-[var(--color-border-subtle)]" />
                <div className="h-4 w-48 rounded bg-[var(--color-border-subtle)]" />
              </div>
            </div>
          )}
          {error && (
            <div className="flex items-center gap-3 px-5 py-4">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-[var(--color-danger-subtle)]">
                <Info className="h-4 w-4 text-[var(--color-danger)]" />
              </div>
              <p className="text-[13px] text-[var(--color-danger)]">
                加载失败：{String(error)}
              </p>
            </div>
          )}
          {data && (
            <>
              <InfoRow icon={Info} label="版本">
                <span className="text-[13px] text-[var(--color-text-secondary)]">
                  {data.version}
                </span>
              </InfoRow>
              <InfoRow icon={RefreshCw} label="更新">
                <div className="flex items-center gap-3">
                  {updateQuery.data?.hasUpdate ? (
                    <>
                      <span className="text-[13px] text-[var(--color-accent-text)]">
                        发现新版本 v{updateQuery.data.latestVersion}
                      </span>
                      <button
                        type="button"
                        onClick={() =>
                          handleDownload(
                            updateQuery.data!.releaseUrl,
                            updateQuery.data!.latestVersion,
                          )
                        }
                        className="inline-flex items-center gap-1 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-2.5 py-1 text-[12px] font-medium text-[var(--color-text-primary)] transition-colors hover:bg-[var(--color-surface-hover)]"
                      >
                        <Download className="h-3 w-3" />
                        前往下载
                      </button>
                    </>
                  ) : (
                    <span className="text-[13px] text-[var(--color-text-secondary)]">
                      {updateQuery.isLoading
                        ? "正在检查…"
                        : updateQuery.error
                          ? "检查失败"
                          : "已是最新版"}
                    </span>
                  )}
                  <button
                    type="button"
                    onClick={handleCheckUpdate}
                    disabled={updateQuery.isFetching}
                    className="ml-auto inline-flex items-center gap-1 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-2.5 py-1 text-[12px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] disabled:opacity-50"
                  >
                    <RefreshCw
                      className={`h-3 w-3 ${updateQuery.isFetching ? "animate-spin" : ""}`}
                    />
                    检查更新
                  </button>
                </div>
              </InfoRow>
              <InfoRow icon={Monitor} label="平台">
                <span className="text-[13px] text-[var(--color-text-secondary)]">
                  {data.platform}
                </span>
              </InfoRow>
              <InfoRow icon={Database} label="数据库">
                <span className="truncate font-mono text-[11px] text-[var(--color-text-tertiary)]">
                  {data.dbPath}
                </span>
              </InfoRow>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

// ─── WebDAV 同步区块 ──────────────────────────────────────────────────────────

function WebDavSyncSection({
  config,
  status,
  onSaveConfig,
  onTestConnection,
  onSyncNow,
  isSyncing,
  isTesting,
  isSaving,
}: {
  config?: SyncConfig;
  status?: import("@skills-pp/shared").SyncStatus;
  onSaveConfig: (config: SyncConfig) => void;
  onTestConnection: (config: SyncConfig) => void;
  onSyncNow: () => void;
  isSyncing: boolean;
  isTesting: boolean;
  isSaving: boolean;
}) {
  const cfg = config ?? DEFAULT_SYNC_CONFIG;
  const [form, setForm] = useState<SyncConfig>(cfg);
  const [formDirty, setFormDirty] = useState(false);

  // 当后端配置加载完成后，同步到本地 form
  if (config && !formDirty) {
    if (JSON.stringify(form) !== JSON.stringify(config)) {
      setForm(config);
    }
  }

  const updateField = <K extends keyof SyncConfig>(key: K, value: SyncConfig[K]) => {
    setForm((prev) => ({ ...prev, [key]: value }));
    setFormDirty(true);
  };

  const handleSave = () => {
    onSaveConfig(form);
    setFormDirty(false);
  };

  const handleTest = () => {
    onTestConnection(form);
  };

  const handleSyncNow = () => {
    // 如果配置有变更，先保存再同步
    if (formDirty) {
      onSaveConfig(form);
      setFormDirty(false);
    }
    onSyncNow();
  };

  const formatSyncTime = (ts: string | null) => {
    if (!ts) return "从未同步";
    try {
      const d = new Date(ts);
      return d.toLocaleString("zh-CN", {
        month: "2-digit", day: "2-digit",
        hour: "2-digit", minute: "2-digit",
      });
    } catch {
      return ts;
    }
  };

  const syncResultIcon = (result: string | null) => {
    if (!result) return <CloudOff className="h-3.5 w-3.5 text-[var(--color-text-tertiary)]" />;
    if (result === "success") return <CheckCircle2 className="h-3.5 w-3.5 text-[var(--color-success)]" />;
    if (result === "conflict") return <AlertCircle className="h-3.5 w-3.5 text-[var(--color-warning)]" />;
    return <AlertCircle className="h-3.5 w-3.5 text-[var(--color-danger)]" />;
  };

  return (
    <div className="px-5 py-4">
      <div className="flex items-center gap-2 mb-3">
        <Cloud className="h-3.5 w-3.5 text-[var(--color-text-tertiary)]" />
        <span className="text-[12px] font-medium text-[var(--color-text-secondary)]">WebDAV 云同步</span>
      </div>

      {/* 配置表单 */}
      <div className="space-y-3">
        <FormField label="WebDAV URL">
          <input
            type="text"
            value={form.webdavUrl}
            onChange={(e) => updateField("webdavUrl", e.target.value)}
            placeholder="https://dav.example.com"
            className="w-full rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-3 py-1.5 text-[13px] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-subtle)]"
          />
        </FormField>

        <div className="grid grid-cols-2 gap-3">
          <FormField label="用户名">
            <input
              type="text"
              value={form.webdavUsername}
              onChange={(e) => updateField("webdavUsername", e.target.value)}
              placeholder="username"
              className="w-full rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-3 py-1.5 text-[13px] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-subtle)]"
            />
          </FormField>
          <FormField label="密码">
            <input
              type="password"
              value={form.webdavPassword}
              onChange={(e) => updateField("webdavPassword", e.target.value)}
              placeholder="••••••••"
              className="w-full rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-3 py-1.5 text-[13px] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-subtle)]"
            />
          </FormField>
        </div>

        <FormField label="远端路径">
          <input
            type="text"
            value={form.webdavRemotePath}
            onChange={(e) => updateField("webdavRemotePath", e.target.value)}
            placeholder="/skillspp"
            className="w-full rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-3 py-1.5 text-[13px] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-subtle)]"
          />
        </FormField>

        {/* 操作按钮 */}
        <div className="flex items-center gap-2 pt-1">
          <button
            type="button"
            onClick={handleSave}
            disabled={isSaving || !formDirty}
            className="inline-flex items-center gap-1.5 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-3 py-1.5 text-[12px] font-medium text-[var(--color-text-primary)] transition-colors hover:bg-[var(--color-surface-hover)] disabled:opacity-50"
          >
            {isSaving ? <RefreshCw className="h-3.5 w-3.5 animate-spin" /> : null}
            保存配置
          </button>
          <button
            type="button"
            onClick={handleTest}
            disabled={isTesting || !form.webdavUrl}
            className="inline-flex items-center gap-1.5 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-3 py-1.5 text-[12px] font-medium text-[var(--color-text-primary)] transition-colors hover:bg-[var(--color-surface-hover)] disabled:opacity-50"
          >
            {isTesting ? <RefreshCw className="h-3.5 w-3.5 animate-spin" /> : <Wifi className="h-3.5 w-3.5" />}
            测试连接
          </button>
          <button
            type="button"
            onClick={handleSyncNow}
            disabled={isSyncing || !form.webdavUrl}
            className="inline-flex items-center gap-1.5 rounded-[var(--radius-sm)] bg-[var(--color-accent-subtle)] px-3 py-1.5 text-[12px] font-medium text-[var(--color-accent-text)] transition-colors hover:opacity-90 disabled:opacity-50"
          >
            {isSyncing ? <RefreshCw className="h-3.5 w-3.5 animate-spin" /> : <CloudUpload className="h-3.5 w-3.5" />}
            立即同步
          </button>
        </div>

        {/* 同步状态 */}
        <div className="flex items-center gap-2 pt-2 border-t border-[var(--color-border-subtle)]">
          {syncResultIcon(status?.lastSyncResult ?? null)}
          <span className="text-[12px] text-[var(--color-text-tertiary)]">
            上次同步：{formatSyncTime(status?.lastSyncAt ?? null)}
          </span>
          {status?.lastSyncDevice && (
            <span className="text-[11px] text-[var(--color-text-tertiary)]">
              · {status.lastSyncDevice}
            </span>
          )}
        </div>

        {/* 自动同步 */}
        <div className="flex items-center gap-3 pt-2">
          <button
            type="button"
            onClick={() => updateField("autoSync", !form.autoSync)}
            className={[
              "w-12 h-6 rounded-full relative transition-colors shrink-0",
              form.autoSync
                ? "bg-[var(--color-accent-subtle)]"
                : "bg-[var(--color-border-subtle)]",
            ].join(" ")}
          >
            <span
              className={[
                "absolute top-1 w-4 h-4 rounded-full transition-transform shadow-sm",
                form.autoSync ? "left-7 translate-x-0" : "left-1",
                form.autoSync
                  ? "bg-[var(--color-accent-text)]"
                  : "bg-[var(--color-text-tertiary)]",
              ].join(" ")}
            />
          </button>
          <span className="text-[12px] text-[var(--color-text-secondary)]">启动时自动同步</span>
          {form.autoSync && (
            <div className="flex items-center gap-1.5 ml-auto">
              <span className="text-[12px] text-[var(--color-text-tertiary)]">每</span>
              <input
                type="number"
                min={5}
                max={1440}
                value={form.autoSyncInterval}
                onChange={(e) => updateField("autoSyncInterval", parseInt(e.target.value) || 30)}
                className="w-16 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-2 py-1 text-[12px] text-[var(--color-text-primary)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-subtle)]"
              />
              <span className="text-[12px] text-[var(--color-text-tertiary)]">分钟</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function FormField({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="block text-[11px] font-medium text-[var(--color-text-tertiary)] mb-1">
        {label}
      </label>
      {children}
    </div>
  );
}

function InfoRow({
  icon: Icon,
  label,
  children,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-4 px-5 py-3.5">
      <Icon className="h-4 w-4 shrink-0 text-[var(--color-text-tertiary)]" />
      <span className="w-20 shrink-0 text-[12px] font-medium text-[var(--color-text-tertiary)]">
        {label}
      </span>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
