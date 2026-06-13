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
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  useInstalledSkills,
  useInstallTasks,
  useUninstallSkill,
  useReinstallSkill,
} from "../../hooks/use-install";
import { useDirectories } from "../../hooks/use-directories";
import { InstallDialog } from "../../components/install/InstallDialog";
import { InstallLogPanel } from "../../components/install/InstallLogPanel";
import type { InstalledSkill } from "@skills-pp/shared";

const STATUS_CONFIG = {
  ok: { icon: CheckCircle, label: "正常", cls: "text-green-600 bg-green-50" },
  missing: { icon: AlertCircle, label: "缺失", cls: "text-red-600 bg-red-50" },
  changed: { icon: HelpCircle, label: "已变更", cls: "text-yellow-600 bg-yellow-50" },
  update_available: { icon: RotateCcw, label: "有更新", cls: "text-blue-600 bg-blue-50" },
} as const;

export default function InstalledPage() {
  const navigate = useNavigate();
  const { data: installed = [], isLoading } = useInstalledSkills();
  const { data: tasks = [] } = useInstallTasks();
  const { data: directories = [] } = useDirectories();

  const uninstallMutation = useUninstallSkill();
  const reinstallMutation = useReinstallSkill();

  const [reinstallTarget, setReinstallTarget] = useState<InstalledSkill | null>(null);
  const [actionPendingId, setActionPendingId] = useState<string | null>(null);

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

  // Recent tasks (latest 5)
  const recentTasks = tasks.slice(0, 5);

  if (isLoading) {
    return (
      <div className="mt-20 text-center text-sm text-gray-400">加载中...</div>
    );
  }

  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-xl font-semibold text-gray-900">已安装</h2>
        <p className="mt-1 text-sm text-gray-500">
          查看和管理已安装到本机的 skill（共 {installed.length} 个）
        </p>
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

            return (
              <div
                key={skill.id}
                className="flex items-center gap-4 rounded-lg border border-gray-200 bg-white p-4"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate text-sm font-semibold text-gray-900">
                      {skill.name}
                    </span>
                    <span
                      className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs ${cfg.cls}`}
                    >
                      <StatusIcon className="h-3 w-3" />
                      {cfg.label}
                    </span>
                  </div>
                  <div className="mt-1 flex items-center gap-3 text-xs text-gray-400">
                    <span>{skill.toolName}</span>
                    <span>·</span>
                    <span className="truncate">{skill.directoryId}</span>
                    {skill.installedAt && (
                      <>
                        <span>·</span>
                        <span>
                          {new Date(skill.installedAt).toLocaleDateString("zh-CN")}
                        </span>
                      </>
                    )}
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  {/* Open directory */}
                  <button
                    onClick={() => openUrl(`file://${skill.directoryId}`)}
                    className="rounded p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600"
                    title="打开目录"
                  >
                    <FolderOpen className="h-4 w-4" />
                  </button>

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
