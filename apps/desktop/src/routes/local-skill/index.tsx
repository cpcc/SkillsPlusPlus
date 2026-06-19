import { useNavigate, useLocation } from "react-router-dom";
import { ArrowLeft, FolderOpen, FileText } from "lucide-react";
import { SkillMdContent } from "../../components/SkillMdContent";
import { useLocalSkillMd } from "../../hooks/use-local-skill";
import { ipc } from "../../lib/ipc";
import { useToast } from "../../components/ui/toast";

interface NavState {
  absolutePath?: string;
  name?: string;
}

export default function LocalSkillPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const toast = useToast();
  const state = (location.state ?? {}) as NavState;
  const absolutePath = state.absolutePath;
  const name = state.name ?? "本地 Skill";

  const { skillMdPath, content, isLoading, isError } =
    useLocalSkillMd(absolutePath);

  if (!absolutePath) {
    return (
      <div className="mx-auto max-w-[680px]">
        <button
          onClick={() => navigate(-1)}
          className="flex items-center gap-1.5 text-[12px] text-[var(--color-text-tertiary)] hover:text-[var(--color-text-secondary)]"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
          返回
        </button>
        <div className="mt-12 flex flex-col items-center gap-3 text-center">
          <FileText className="h-5 w-5 text-[var(--color-text-tertiary)]" />
          <p className="text-[13px] text-[var(--color-text-secondary)]">
            缺少路径参数
          </p>
        </div>
      </div>
    );
  }

  async function handleShowInFinder(path: string) {
    try {
      await ipc.openSkillDir(path);
    } catch {
      toast("无法打开目录", path, "error");
    }
  }

  return (
    <div className="mx-auto max-w-[680px]">
      <button
        onClick={() => navigate(-1)}
        className="flex items-center gap-1.5 text-[12px] text-[var(--color-text-tertiary)] transition-colors hover:text-[var(--color-text-secondary)]"
      >
        <ArrowLeft className="h-3.5 w-3.5" />
        返回
      </button>

      <div className="mt-5">
        <h1 className="text-[22px] font-semibold tracking-tight text-[var(--color-text-primary)]">
          {name}
        </h1>
        <p className="mt-1 text-[12px] text-[var(--color-text-tertiary)]">
          本地 Skill
        </p>
      </div>

      <div className="mt-6 rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] divide-y divide-[var(--color-border-subtle)]">
        <DetailRow label="路径">
          <span
            className="block break-all font-mono text-[12px] text-[var(--color-text-secondary)]"
            title={absolutePath}
          >
            {absolutePath}
          </span>
        </DetailRow>
        {skillMdPath && skillMdPath !== absolutePath && (
          <DetailRow label="SKILL.md">
            <span
              className="block break-all font-mono text-[12px] text-[var(--color-text-secondary)]"
              title={skillMdPath}
            >
              {skillMdPath}
            </span>
          </DetailRow>
        )}
      </div>

      <div className="mt-6 flex gap-2.5">
        <button
          onClick={() => handleShowInFinder(absolutePath)}
          className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-4 py-[7px] text-[13px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
        >
          <FolderOpen className="h-3.5 w-3.5" />
          在文件管理器中显示
        </button>
      </div>

      <div className="mt-8">
        <SkillMdContent
          markdown={content ?? undefined}
          isLoading={isLoading}
          isError={isError}
        />
        {!isLoading && !isError && !content && (
          <div className="flex items-center gap-3 rounded-[var(--radius-lg)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-5">
            <FileText className="h-4 w-4 text-[var(--color-text-tertiary)]" />
            <p className="text-[13px] text-[var(--color-text-secondary)]">
              该路径下未找到可渲染的 SKILL.md / 文本文件
            </p>
          </div>
        )}
      </div>
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
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
