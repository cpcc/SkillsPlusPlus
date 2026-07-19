import { useAppInfo } from "../../hooks/use-app-info";
import { useTheme, type ThemePreference } from "../../hooks/use-theme";
import { useUpdateCheck } from "../../hooks/use-update-check";
import { useMirrorConfig, useSetMirrorConfig, useMirrorHealth } from "../../hooks/use-mirror";
import { useToast } from "../../components/ui/toast";
import { ipc } from "../../lib/ipc";
import { Info, Database, Monitor, SunMoon, RefreshCw, Download, Globe, Plus, Trash2, Activity } from "lucide-react";

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
  const toast = useToast();

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
