import { useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  FolderOpen,
  Trash2,
  RotateCcw,
  AlertCircle,
  CheckCircle,
  HelpCircle,
  Loader2,
  RefreshCw,
  ArrowUpCircle,
  ExternalLink,
} from "lucide-react";
import { openPath } from "@tauri-apps/plugin-opener";
import {
  useInstalledSkills,
  useInstallTasks,
  useUninstallSkill,
  useReinstallSkill,
  useRefreshInstalledSkills,
  useCheckSkillUpdate,
} from "../../hooks/use-install";
import { useDirectories } from "../../hooks/use-directories";
import { InstallDialog } from "../../components/install/InstallDialog";
import { InstallLogPanel } from "../../components/install/InstallLogPanel";
import type { InstalledSkill } from "@skills-pp/shared";

const STATUS_CONFIG = {
  ok: { icon: CheckCircle, label: "正常", cls: "text-green-600 bg-green-50" },
  missing: { icon: AlertCircle, label: "缺失", cls: "text-red-600 bg-red-50" },
  changed: { icon: HelpCircle, label: "已变更", cls: "text-yellow-600 bg-yellow-50" },
  update_available: { icon: ArrowUpCircle, label: "有更新", cls: "text-blue-600 bg-blue-50" },
} as const;

export default function InstalledPage() {
  const navigate = useNavigate();
  const { data: installed = [], isLoading } = useInstalledSkills();
  const { data: tasks = [] } = useInstallTasks();
  const { data: directories = [] } = useDirectories();

  const uninstallMutation = useUninstallSkill();
  const reinstallMutation = useReinstallSkill();
  const refreshMutation = useRefreshInstalledSkills();
  const checkUpdateMutation = useCheckSkillUpdate();

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

  function handleReinstallConfirm(directoryId: string, _overwrite: boolean) {
    if (!reinstallTarget) return;
    setActionPendingId(reinstallTarget.id);
    reinstallMutation.mutate(
      {
        skillId: reinstallTarget.skillId,
        skillName: reinstallTarget.name,
        repoUrl: reinstallTarget.repoUrl ?? "",
        directoryId,
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
    // Build the actual skill directory path
    const skillPath = skill.directoryPath
      ? `${skill.directoryPath}/${skill.name}`
      : skill.directoryId;
    openPath(skillPath);
  }

  // Recent tasks (latest 5)
  const recentTasks = tasks.slice(0, 5);

  // Compute skill full path for display
  function getSkillPath(skill: InstalledSkill): string {
    if (skill.directoryPath) {
      return `${skill.directoryPath}/${skill.name}`;
    }
    return skill.directoryId;
  }

  if (isLoading) {
    return (
      <div className="mt-20 text-center text-sm text-gray-400">加载中...</div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-gray-900">已安装</h2>
          <p className="mt-1 text-sm text-gray-500">
            查看和管理已安装到本机的 skill（共 {installed.length} 个）
          </p>
        </div>
        <button
          onClick={() => refreshMutation.mutate()}
          disabled={refreshMutation.isPending}
          className="flex items-center gap-2 rounded-lg border border-gray-300 px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
        >
          {refreshMutation.isPending ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <RefreshCw className="h-3.5 w-3.5" />
          )}
          刷新状态
        </button>
      </div>

      {/* Recent install tasks */}
      {recentTasks.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-medium text-gray-700">最近安装记录</h3>
          {recentTasks.map((t) => (
            <InstallLogPanel key={t.id} task={t} />
          ))}
        </div>
      )}

      {/* Installed list */}
      {installed.length === 0 ? (
        <div className="rounded-lg border border-dashed border-gray-300 p-12 text-center">
          <p className="text-sm text-gray-400">暂无已安装的 skill</p>
          <button
            onClick={() => navigate("/")}
            className="mt-3 text-sm text-brand-600 hover:underline"
          >
            去发现页浏览
          </button>
        </div>
      ) : (
        <div className="space-y-2">
          {installed.map((skill) => {
            const cfg = STATUS_CONFIG[skill.status];
            const StatusIcon = cfg.icon;
            const isBusy = actionPendingId === skill.id;
            const isChecking = checkingId === skill.id;
            const skillPath = getSkillPath(skill);

            return (
              <div
                key={skill.id}
                className="flex items-center gap-4 rounded-lg border border-gray-200 bg-white p-4"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => {
                        if (skill.skillId) {
                          navigate(`/skill/${encodeURIComponent(skill.skillId!)}`);
                        }
                      }}
                      className="truncate text-sm font-semibold text-gray-900 hover:text-brand-600 hover:underline"
                    >
                      {skill.name}
                    </button>
                    <span
                      className={`inline-flex shrink-0 items-center gap-1 rounded-full px-2 py-0.5 text-xs ${cfg.cls}`}
                    >
                      <StatusIcon className="h-3 w-3" />
                      {cfg.label}
                    </span>
                  </div>
                  <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-gray-400">
                    <span>{skill.toolName}</span>
                    <span className="truncate font-mono" title={skillPath}>
                      {skillPath}
                    </span>
                    {skill.sourceId && (
                      <span className="text-brand-500">{skill.sourceId}</span>
                    )}
                    {skill.installedAt && (
                      <span>
                        {new Date(skill.installedAt).toLocaleDateString("zh-CN")}
                      </span>
                    )}
                  </div>
                </div>

                <div className="flex shrink-0 items-center gap-1">
                  {/* Open directory */}
                  <button
                    onClick={() => handleOpenDir(skill)}
                    className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600"
                    title="打开目录"
                  >
                    <FolderOpen className="h-4 w-4" />
                  </button>

                  {/* View source / detail */}
                  {skill.skillId && (
                    <button
                      onClick={() => navigate(`/skill/${encodeURIComponent(skill.skillId!)}`)}
                      className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600"
                      title="查看详情"
                    >
                      <ExternalLink className="h-4 w-4" />
                    </button>
                  )}

                  {/* Check update */}
                  {skill.repoUrl && skill.status !== "missing" && (
                    <button
                      onClick={() => handleCheckUpdate(skill)}
                      disabled={isChecking || isBusy}
                      className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-green-600 disabled:opacity-50"
                      title="检查更新"
                    >
                      {isChecking ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <ArrowUpCircle className="h-4 w-4" />
                      )}
                    </button>
                  )}

                  {/* Reinstall */}
                  {skill.repoUrl && (
                    <button
                      onClick={() => setReinstallTarget(skill)}
                      disabled={isBusy}
                      className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-blue-600 disabled:opacity-50"
                      title="重装"
                    >
                      {isBusy ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <RotateCcw className="h-4 w-4" />
                      )}
                    </button>
                  )}

                  {/* Uninstall */}
                  <button
                    onClick={() => handleUninstall(skill)}
                    disabled={isBusy}
                    className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-red-600 disabled:opacity-50"
                    title="卸载"
                  >
                    <Trash2 className="h-4 w-4" />
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
          directories={directories}
          isPending={reinstallMutation.isPending}
          onInstall={handleReinstallConfirm}
        />
      )}
    </div>
  );
}
