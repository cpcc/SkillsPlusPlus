import { useState } from "react";
import { useNavigate, useParams, useLocation } from "react-router-dom";
import { ArrowLeft, ExternalLink, GitBranch, Download, Package } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { SkillItem } from "@skills-pp/shared";
import { useSkill } from "../../hooks/use-skills";
import { useDirectories } from "../../hooks/use-directories";
import { useInstallSkill, useInstallTasks } from "../../hooks/use-install";
import { InstallDialog } from "../../components/install/InstallDialog";
import { InstallLogPanel } from "../../components/install/InstallLogPanel";

export default function SkillDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const location = useLocation();
  // 在线兜底卡片（id 形如 `online_xxx`，不在 skill_cache 里）会通过
  // location.state 传整个 SkillItem 过来；优先使用，避免 get_skill 返回 None。
  const passedSkill = (location.state as { skill?: SkillItem } | null)?.skill;
  const { data: fetchedSkill, isLoading } = useSkill(
    id ? decodeURIComponent(id) : "",
  );
  const skill = passedSkill ?? fetchedSkill;
  const { data: directories = [] } = useDirectories();
  const installMutation = useInstallSkill();
  const { data: tasks = [] } = useInstallTasks();

  const [installOpen, setInstallOpen] = useState(false);

  const relatedTasks = tasks.filter(
    (t) => skill && t.skillName === skill.name,
  );

  if (isLoading) {
    return (
      <div className="mx-auto max-w-[680px]">
        <div className="animate-pulse space-y-4">
          <div className="h-4 w-16 rounded bg-[var(--color-border-subtle)]" />
          <div className="h-7 w-48 rounded bg-[var(--color-border-subtle)]" />
          <div className="h-4 w-32 rounded bg-[var(--color-border-subtle)]" />
          <div className="h-32 w-full rounded-[var(--radius-lg)] bg-[var(--color-surface-raised)]" />
        </div>
      </div>
    );
  }

  if (!skill) {
    return (
      <div className="mt-20 flex flex-col items-center gap-3 text-center">
        <div className="flex h-12 w-12 items-center justify-center rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)]">
          <Package className="h-5 w-5 text-[var(--color-text-tertiary)]" />
        </div>
        <p className="text-[13px] text-[var(--color-text-secondary)]">
          Skill 不存在或已从缓存中移除
        </p>
        <button
          onClick={() => navigate(-1)}
          className="text-[13px] text-[var(--color-accent)] hover:text-[var(--color-accent-hover)]"
        >
          返回
        </button>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-[680px]">
      {/* Back */}
      <button
        onClick={() => navigate(-1)}
        className="flex items-center gap-1.5 text-[12px] text-[var(--color-text-tertiary)] transition-colors hover:text-[var(--color-text-secondary)]"
      >
        <ArrowLeft className="h-3.5 w-3.5" />
        返回
      </button>

      {/* Header */}
      <div className="mt-5">
        <h1 className="text-[22px] font-semibold tracking-tight text-[var(--color-text-primary)]">
          {skill.name}
        </h1>
        {skill.author && (
          <p className="mt-1 text-[12px] text-[var(--color-text-tertiary)]">
            by {skill.author}
          </p>
        )}
      </div>

      {/* Description */}
      {skill.description && (
        <p className="mt-4 text-[13px] leading-relaxed text-[var(--color-text-secondary)]">
          {skill.description}
        </p>
      )}

      {/* Meta card */}
      <div className="mt-6 rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] divide-y divide-[var(--color-border-subtle)]">
        <DetailRow label="来源">
          <span className="text-[13px] text-[var(--color-text-secondary)]">
            {skill.sourceId}
          </span>
        </DetailRow>

        {skill.updatedAt && (
          <DetailRow label="更新时间">
            <span className="text-[13px] text-[var(--color-text-secondary)]">
              {new Date(skill.updatedAt).toLocaleDateString("zh-CN")}
            </span>
          </DetailRow>
        )}

        {skill.compatibleTools && skill.compatibleTools.length > 0 && (
          <DetailRow label="兼容工具">
            <div className="flex flex-wrap gap-1.5">
              {skill.compatibleTools.map((t) => (
                <span
                  key={t}
                  className="rounded-full bg-[var(--color-accent-subtle)] px-2 py-[1px] text-[11px] text-[var(--color-accent)]"
                >
                  {t}
                </span>
              ))}
            </div>
          </DetailRow>
        )}

        {skill.tags.length > 0 && (
          <DetailRow label="标签">
            <div className="flex flex-wrap gap-1.5">
              {skill.tags.map((tag) => (
                <span
                  key={tag}
                  className="rounded-full border border-[var(--color-border-subtle)] px-2 py-[1px] text-[11px] text-[var(--color-text-tertiary)]"
                >
                  {tag}
                </span>
              ))}
            </div>
          </DetailRow>
        )}
      </div>

      {/* Actions */}
      <div className="mt-6 flex gap-2.5">
        {skill.repoUrl && (
          <button
            onClick={() => setInstallOpen(true)}
            className="flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--color-accent-muted)] px-4 py-[7px] text-[13px] font-medium text-white transition-colors hover:bg-[var(--color-accent)] active:scale-[0.98]"
          >
            <Download className="h-3.5 w-3.5" />
            安装
          </button>
        )}
        {skill.repoUrl && (
          <button
            onClick={() => openUrl(skill.repoUrl!)}
            className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-4 py-[7px] text-[13px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
          >
            <GitBranch className="h-3.5 w-3.5" />
            仓库
          </button>
        )}
        <button
          onClick={() => openUrl(skill.detailUrl)}
          className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-4 py-[7px] text-[13px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
        >
          <ExternalLink className="h-3.5 w-3.5" />
          详情
        </button>
      </div>

      {/* Install logs */}
      {relatedTasks.length > 0 && (
        <div className="mt-8 space-y-3">
          <h3 className="text-[12px] font-medium uppercase tracking-wide text-[var(--color-text-tertiary)]">
            安装记录
          </h3>
          {relatedTasks.map((t) => (
            <InstallLogPanel key={t.id} task={t} />
          ))}
        </div>
      )}

      {skill.repoUrl && (
        <InstallDialog
          open={installOpen}
          onOpenChange={setInstallOpen}
          skillName={skill.name}
          repoUrl={skill.repoUrl}
          skillId={skill.id}
          defaultStrategy={skill.installStrategy}
          archiveUrl={skill.archiveUrl}
          directories={directories}
          isPending={installMutation.isPending}
          onInstall={(directoryId, overwrite, strategy) => {
            installMutation.mutate(
              {
                skillId: skill.id,
                skillName: skill.name,
                repoUrl: skill.repoUrl!,
                directoryId,
                overwrite,
                strategy,
                archiveUrl: skill.archiveUrl,
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
    <div className="flex items-start gap-6 px-5 py-3">
      <span className="w-20 shrink-0 text-[12px] font-medium text-[var(--color-text-tertiary)]">
        {label}
      </span>
      <div className="flex-1">{children}</div>
    </div>
  );
}
