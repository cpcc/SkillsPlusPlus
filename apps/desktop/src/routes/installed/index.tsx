import { useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  FolderOpen,
  FolderInput,
  Trash2,
  RotateCcw,
  AlertCircle,
  CheckCircle,
  HelpCircle,
  Loader2,
  RefreshCw,
  ArrowUpCircle,
  ExternalLink,
  Package,
} from "lucide-react";
import { ipc } from "../../lib/ipc";
import {
  useInstalledSkills,
  useUninstallSkill,
  useReinstallSkill,
  useRefreshInstalledSkills,
  useCheckSkillUpdate,
  useImportExistingSkills,
} from "../../hooks/use-install";
import { useDirectories } from "../../hooks/use-directories";
import { useToast } from "../../components/ui/toast";
import { InstallDialog } from "../../components/install/InstallDialog";
import type { InstalledSkill } from "@skills-pp/shared";

const STATUS_CONFIG = {
  ok: { icon: CheckCircle, label: "正常", cls: "text-[var(--color-success)] bg-[var(--color-success-subtle)]" },
  missing: { icon: AlertCircle, label: "缺失", cls: "text-[var(--color-danger)] bg-[var(--color-danger-subtle)]" },
  changed: { icon: HelpCircle, label: "已变更", cls: "text-[var(--color-warning)] bg-[var(--color-warning-subtle)]" },
  update_available: { icon: ArrowUpCircle, label: "有更新", cls: "text-[var(--color-info)] bg-[var(--color-info-subtle)]" },
} as const;

export default function InstalledPage() {
  const navigate = useNavigate();
  const { data: installed = [], isLoading } = useInstalledSkills();
  const { data: directories = [] } = useDirectories();

  const uninstallMutation = useUninstallSkill();
  const reinstallMutation = useReinstallSkill();
  const refreshMutation = useRefreshInstalledSkills();
  const checkUpdateMutation = useCheckSkillUpdate();
  const importMutation = useImportExistingSkills();
  const toast = useToast();

  function handleImportLocal() {
    importMutation.mutate(undefined, {
      onSuccess: (count) => {
        if (count > 0) {
          toast(`导入了 ${count} 个本地 skill`);
        } else {
          toast("没有新的本地 skill 可导入");
        }
      },
      onError: (e) => toast("导入失败", String(e), "error"),
    });
  }

  const [reinstallTarget, setReinstallTarget] = useState<InstalledSkill | null>(null);
  const [actionPendingId, setActionPendingId] = useState<string | null>(null);
  const [checkingId, setCheckingId] = useState<string | null>(null);

  function handleUninstall(skill: InstalledSkill) {
    if (!confirm(`确认卸载「${skill.name}」？这将删除本地目录中的相关文件。`)) return;
    setActionPendingId(skill.id);
    uninstallMutation.mutate(
      { skillName: skill.name, directoryId: skill.directoryId },
      { onSettled: () => setActionPendingId(null) },
    );
  }

  function handleReinstallConfirm(directoryId: string, _overwrite: boolean, strategy: InstalledSkill["installStrategy"]) {
    if (!reinstallTarget) return;
    setActionPendingId(reinstallTarget.id);
    reinstallMutation.mutate(
      {
        skillId: reinstallTarget.skillId,
        skillName: reinstallTarget.name,
        repoUrl: reinstallTarget.repoUrl ?? "",
        directoryId,
        strategy,
      },
      {
        onSettled: () => {
          setReinstallTarget(null);
          setActionPendingId(null);
        },
      },
    );
  }

  function handleCheckUpdate(skill: InstalledSkill) {
    setCheckingId(skill.id);
    checkUpdateMutation.mutate(skill.id, {
      onSettled: () => setCheckingId(null),
    });
  }

  function handleOpenDir(skill: InstalledSkill) {
    const skillPath = skill.directoryPath
      ? `${skill.directoryPath}/${skill.name}`
      : skill.directoryId;
    ipc.openSkillDir(skillPath).catch((e) => {
      console.error("openSkillDir failed", e);
    });
  }

  function getSkillPath(skill: InstalledSkill): string {
    if (skill.directoryPath) {
      return `${skill.directoryPath}/${skill.name}`;
    }
    return skill.directoryId;
  }

  if (isLoading) {
    return (
      <div className="mx-auto max-w-[960px]">
        <div className="animate-pulse space-y-4">
          <div className="h-6 w-24 rounded bg-[var(--color-border-subtle)]" />
          <div className="h-4 w-48 rounded bg-[var(--color-border-subtle)]" />
          <div className="space-y-2">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="h-16 rounded-[var(--radius-lg)] bg-[var(--color-surface-raised)]" />
            ))}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-[960px] space-y-8">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold tracking-tight text-[var(--color-text-primary)]">
            已安装
          </h1>
          <p className="mt-1 text-[13px] text-[var(--color-text-secondary)]">
            查看和管理已安装到本机的 skill（共 {installed.length} 个）
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleImportLocal}
            disabled={importMutation.isPending}
            className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-3 py-[6px] text-[13px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)] disabled:opacity-40"
          >
            {importMutation.isPending ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <FolderInput className="h-3.5 w-3.5" />
            )}
            导入本地
          </button>
          <button
            onClick={() => refreshMutation.mutate()}
            disabled={refreshMutation.isPending}
            className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-3 py-[6px] text-[13px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)] disabled:opacity-40"
          >
            {refreshMutation.isPending ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <RefreshCw className="h-3.5 w-3.5" />
            )}
            刷新状态
          </button>
        </div>
      </div>

      {/* Installed list */}
      {installed.length === 0 ? (
        <div className="flex flex-col items-center gap-4 rounded-[var(--radius-lg)] border border-dashed border-[var(--color-border-default)] p-16 text-center">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)]">
            <Package className="h-5 w-5 text-[var(--color-text-tertiary)]" />
          </div>
          <div>
            <p className="text-[13px] font-medium text-[var(--color-text-secondary)]">
              暂无已安装的 skill
            </p>
            <button
              onClick={() => navigate("/")}
              className="mt-2 text-[12px] text-[var(--color-accent)] hover:text-[var(--color-accent-hover)]"
            >
              去发现页浏览
            </button>
          </div>
        </div>
      ) : (
        <div className="space-y-1.5">
          {installed.map((skill) => {
            const cfg = STATUS_CONFIG[skill.status];
            const StatusIcon = cfg.icon;
            const isBusy = actionPendingId === skill.id;
            const isChecking = checkingId === skill.id;
            const skillPath = getSkillPath(skill);

            return (
              <div
                key={skill.id}
                className="group flex items-center gap-4 rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] px-4 py-3 transition-colors hover:border-[var(--color-border-default)]"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => {
                        if (skill.skillId) {
                          navigate(`/skill/${encodeURIComponent(skill.skillId!)}`);
                        } else {
                          handleOpenDir(skill);
                        }
                      }}
                      className="truncate text-[13px] font-semibold text-[var(--color-text-primary)] transition-colors hover:text-[var(--color-accent-text)]"
                      title={skill.skillId ? "查看详情" : "打开目录"}
                    >
                      {skill.name}
                    </button>
                    {skill.author && (
                      <span className="shrink-0 text-[11px] text-[var(--color-text-tertiary)]">
                        by {skill.author}
                      </span>
                    )}
                    <span
                      className={`inline-flex shrink-0 items-center gap-1 rounded-full px-2 py-[1px] text-[11px] ${cfg.cls}`}
                    >
                      <StatusIcon className="h-3 w-3" />
                      {cfg.label}
                    </span>
                  </div>
                  {skill.description && (
                    <p
                      className="mt-0.5 truncate text-[12px] text-[var(--color-text-secondary)]"
                      title={skill.description}
                    >
                      {skill.description}
                    </p>
                  )}
                  <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-[11px] text-[var(--color-text-tertiary)]">
                    <span>{skill.toolName}</span>
                    <span className="truncate font-mono text-[10px]" title={skillPath}>
                      {skillPath}
                    </span>
                    {skill.sourceId && (
                      <span className="text-[var(--color-accent)]">{skill.sourceId}</span>
                    )}
                    {skill.installedAt && (
                      <span>
                        {new Date(skill.installedAt).toLocaleDateString("zh-CN")}
                      </span>
                    )}
                  </div>
                </div>

                {/* Actions */}
                <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                  <button
                    onClick={() => handleOpenDir(skill)}
                    className="rounded-[var(--radius-sm)] p-1.5 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-secondary)]"
                    title="打开目录"
                  >
                    <FolderOpen className="h-3.5 w-3.5" />
                  </button>

                  {skill.skillId && (
                    <button
                      onClick={() => navigate(`/skill/${encodeURIComponent(skill.skillId!)}`)}
                      className="rounded-[var(--radius-sm)] p-1.5 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-secondary)]"
                      title="查看详情"
                    >
                      <ExternalLink className="h-3.5 w-3.5" />
                    </button>
                  )}

                  {skill.repoUrl && skill.status !== "missing" && (
                    <button
                      onClick={() => handleCheckUpdate(skill)}
                      disabled={isChecking || isBusy}
                      className="rounded-[var(--radius-sm)] p-1.5 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-success)] disabled:opacity-40"
                      title="检查更新"
                    >
                      {isChecking ? (
                        <Loader2 className="h-3.5 w-3.5 animate-spin" />
                      ) : (
                        <ArrowUpCircle className="h-3.5 w-3.5" />
                      )}
                    </button>
                  )}

                  {skill.repoUrl && (
                    <button
                      onClick={() => setReinstallTarget(skill)}
                      disabled={isBusy}
                      className="rounded-[var(--radius-sm)] p-1.5 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-info)] disabled:opacity-40"
                      title="重装"
                    >
                      {isBusy ? (
                        <Loader2 className="h-3.5 w-3.5 animate-spin" />
                      ) : (
                        <RotateCcw className="h-3.5 w-3.5" />
                      )}
                    </button>
                  )}

                  <button
                    onClick={() => handleUninstall(skill)}
                    disabled={isBusy}
                    className="rounded-[var(--radius-sm)] p-1.5 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-danger)] disabled:opacity-40"
                    title="卸载"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Reinstall dialog */}
      {reinstallTarget && (
        <InstallDialog
          open={!!reinstallTarget}
          onOpenChange={(open) => { if (!open) setReinstallTarget(null); }}
          skillName={reinstallTarget.name}
          repoUrl={reinstallTarget.repoUrl ?? ""}
          skillId={reinstallTarget.skillId}
          defaultStrategy={reinstallTarget.installStrategy}
          directories={directories}
          isPending={reinstallMutation.isPending}
          onInstall={handleReinstallConfirm}
        />
      )}
    </div>
  );
}
