import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, ExternalLink, GitBranch, Download } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useSkill } from "../../hooks/use-skills";
import { useDirectories } from "../../hooks/use-directories";
import { useInstallSkill, useInstallTasks } from "../../hooks/use-install";
import { InstallDialog } from "../../components/install/InstallDialog";
import { InstallLogPanel } from "../../components/install/InstallLogPanel";

export default function SkillDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: skill, isLoading } = useSkill(
    id ? decodeURIComponent(id) : "",
  );
  const { data: directories = [] } = useDirectories();
  const installMutation = useInstallSkill();
  const { data: tasks = [] } = useInstallTasks();

  const [installOpen, setInstallOpen] = useState(false);

  // Find tasks related to this skill
  const relatedTasks = tasks.filter(
    (t) => skill && t.skillName === skill.name,
  );

  if (isLoading) {
    return (
      <div className="mt-20 text-center text-sm text-gray-400">加载中...</div>
    );
  }

  if (!skill) {
    return (
      <div className="mt-20 text-center">
        <p className="text-sm text-gray-400">Skill 不存在或已从缓存中移除</p>
        <button
          onClick={() => navigate(-1)}
          className="mt-4 text-sm text-brand-600 hover:underline"
        >
          返回
        </button>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-2xl">
      <button
        onClick={() => navigate(-1)}
        className="flex items-center gap-2 text-sm text-gray-500 hover:text-gray-700"
      >
        <ArrowLeft className="h-4 w-4" />
        返回
      </button>

      <div className="mt-6">
        <h2 className="text-2xl font-bold text-gray-900">{skill.name}</h2>
        {skill.author && (
          <p className="mt-1 text-sm text-gray-400">by {skill.author}</p>
        )}

        {skill.description && (
          <p className="mt-4 text-sm leading-relaxed text-gray-600">
            {skill.description}
          </p>
        )}

        <div className="mt-6 space-y-4 rounded-lg border border-gray-200 bg-white p-4">
          <DetailRow label="来源">
            <span className="text-sm text-gray-600">{skill.sourceId}</span>
          </DetailRow>

          {skill.updatedAt && (
            <DetailRow label="更新时间">
              <span className="text-sm text-gray-600">
                {new Date(skill.updatedAt).toLocaleDateString("zh-CN")}
              </span>
            </DetailRow>
          )}

          {skill.compatibleTools && skill.compatibleTools.length > 0 && (
            <DetailRow label="兼容工具">
              <div className="flex flex-wrap gap-1">
                {skill.compatibleTools.map((t) => (
                  <span
                    key={t}
                    className="rounded-full bg-brand-50 px-2 py-0.5 text-xs text-brand-700"
                  >
                    {t}
                  </span>
                ))}
              </div>
            </DetailRow>
          )}

          {skill.tags.length > 0 && (
            <DetailRow label="标签">
              <div className="flex flex-wrap gap-1">
                {skill.tags.map((tag) => (
                  <span
                    key={tag}
                    className="rounded-full bg-gray-100 px-2 py-0.5 text-xs text-gray-500"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </DetailRow>
          )}
        </div>

        <div className="mt-6 flex gap-3">
          {skill.repoUrl && (
            <button
              onClick={() => setInstallOpen(true)}
              className="flex items-center gap-2 rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white hover:bg-brand-700"
            >
              <Download className="h-4 w-4" />
              安装
            </button>
          )}
          {skill.repoUrl && (
            <button
              onClick={() => openUrl(skill.repoUrl!)}
              className="flex items-center gap-2 rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
            >
              <GitBranch className="h-4 w-4" />
              查看仓库
            </button>
          )}
          <button
            onClick={() => openUrl(skill.detailUrl)}
            className="flex items-center gap-2 rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
          >
            <ExternalLink className="h-4 w-4" />
            打开详情
          </button>
        </div>

        {/* Install result logs */}
        {relatedTasks.length > 0 && (
          <div className="mt-6 space-y-3">
            <h3 className="text-sm font-medium text-gray-700">安装记录</h3>
            {relatedTasks.map((t) => (
              <InstallLogPanel key={t.id} task={t} />
            ))}
          </div>
        )}
      </div>

      {skill.repoUrl && (
        <InstallDialog
          open={installOpen}
          onOpenChange={setInstallOpen}
          skillName={skill.name}
          repoUrl={skill.repoUrl}
          skillId={skill.id}
          directories={directories}
          isPending={installMutation.isPending}
          onInstall={(directoryId, overwrite) => {
            installMutation.mutate(
              {
                skillId: skill.id,
                skillName: skill.name,
                repoUrl: skill.repoUrl!,
                directoryId,
                overwrite,
              },
              { onSuccess: () => setInstallOpen(false) },
            );
          }}
        />
      )}
    </div>
  );
}

function DetailRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-start gap-4">
      <span className="w-20 shrink-0 text-xs font-medium text-gray-400">
        {label}
      </span>
      <div className="flex-1">{children}</div>
    </div>
  );
}
