import * as Dialog from "@radix-ui/react-dialog";
import { useState, useEffect } from "react";
import { X, AlertTriangle, CheckCircle, Loader2, ChevronDown } from "lucide-react";
import type { AiToolDirectory, InstallPreview } from "@skills-pp/shared";
import { ipc } from "../../lib/ipc";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  skillName: string;
  repoUrl: string;
  skillId?: string;
  directories: AiToolDirectory[];
  onInstall: (directoryId: string, overwrite: boolean) => void;
  isPending: boolean;
}

export function InstallDialog({
  open,
  onOpenChange,
  skillName,
  repoUrl,
  skillId: _skillId,
  directories,
  onInstall,
  isPending,
}: Props) {
  const enabledDirs = directories.filter((d) => d.enabled && d.isDetected && d.writable);
  const defaultDir = enabledDirs.find((d) => d.isDefault) ?? enabledDirs[0];

  const [selectedDirId, setSelectedDirId] = useState(defaultDir?.id ?? "");
  const [preview, setPreview] = useState<InstallPreview | null>(null);
  const [loadingPreview, setLoadingPreview] = useState(false);
  const [overwrite, setOverwrite] = useState(false);

  useEffect(() => {
    if (open) {
      setSelectedDirId(defaultDir?.id ?? "");
      setPreview(null);
      setOverwrite(false);
    }
  }, [open, defaultDir?.id]);

  useEffect(() => {
    if (!selectedDirId || !repoUrl) return;
    setLoadingPreview(true);
    ipc.previewInstall(skillName, repoUrl, selectedDirId)
      .then((p: InstallPreview) => { setPreview(p); setOverwrite(false); })
      .catch(() => setPreview(null))
      .finally(() => setLoadingPreview(false));
  }, [selectedDirId, skillName, repoUrl]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!selectedDirId) return;
    onInstall(selectedDirId, overwrite);
  }

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm" />
        <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-lg -translate-x-1/2 -translate-y-1/2 rounded-[var(--radius-xl)] border border-[var(--color-border-default)] bg-[var(--color-surface-overlay)] p-6 shadow-2xl shadow-black/30">
          <div className="flex items-center justify-between">
            <Dialog.Title className="text-[15px] font-semibold text-[var(--color-text-primary)]">
              安装 Skill
            </Dialog.Title>
            <Dialog.Close asChild>
              <button className="rounded-[var(--radius-sm)] p-1 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-secondary)]">
                <X className="h-4 w-4" />
              </button>
            </Dialog.Close>
          </div>

          <form onSubmit={handleSubmit} className="mt-5 space-y-4">
            {/* Skill info */}
            <div className="rounded-[var(--radius-md)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-3.5">
              <p className="text-[13px] font-medium text-[var(--color-text-primary)]">
                {skillName}
              </p>
              <p className="mt-0.5 truncate font-mono text-[11px] text-[var(--color-text-tertiary)]">
                {repoUrl}
              </p>
            </div>

            {/* Directory selector */}
            <div>
              <label className="mb-1.5 block text-[12px] font-medium text-[var(--color-text-secondary)]">
                安装目录
              </label>
              {enabledDirs.length === 0 ? (
                <p className="mt-1 text-[12px] text-[var(--color-danger)]">
                  没有可用的安装目录，请先在「工具与目录」中配置。
                </p>
              ) : (
                <div className="relative">
                  <select
                    className="w-full appearance-none rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-3 py-2 pr-8 text-[13px] text-[var(--color-text-primary)] transition-colors focus:border-[var(--color-accent)] focus:outline-none cursor-pointer"
                    value={selectedDirId}
                    onChange={(e) => setSelectedDirId(e.target.value)}
                    required
                  >
                    {enabledDirs.map((d) => (
                      <option key={d.id} value={d.id}>
                        [{d.toolName}] {d.path}
                      </option>
                    ))}
                  </select>
                  <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--color-text-tertiary)]" />
                </div>
              )}
            </div>

            {/* Preview */}
            {loadingPreview && (
              <div className="flex items-center gap-2 text-[12px] text-[var(--color-text-tertiary)]">
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                检查目标路径...
              </div>
            )}

            {preview && !loadingPreview && (
              <div className="space-y-2.5">
                <div className="rounded-[var(--radius-md)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)] p-3">
                  <p className="text-[11px] text-[var(--color-text-tertiary)]">安装到</p>
                  <p className="mt-0.5 truncate font-mono text-[12px] text-[var(--color-text-secondary)]">
                    {preview.targetPath}
                  </p>
                </div>

                {preview.conflict && (
                  <div className="rounded-[var(--radius-md)] border border-[var(--color-warning)]/30 bg-[var(--color-warning-subtle)] p-3">
                    <div className="flex items-start gap-2">
                      <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-[var(--color-warning)]" />
                      <div>
                        <p className="text-[12px] font-medium text-[var(--color-warning)]">
                          目录已存在
                        </p>
                        <p className="mt-0.5 truncate font-mono text-[11px] text-[var(--color-text-tertiary)]">
                          {preview.conflict.existingPath}
                        </p>
                        <label className="mt-2.5 flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={overwrite}
                            onChange={(e) => setOverwrite(e.target.checked)}
                            className="h-3.5 w-3.5 accent-[var(--color-accent)]"
                          />
                          <span className="text-[12px] text-[var(--color-text-secondary)]">
                            覆盖安装（删除现有目录）
                          </span>
                        </label>
                      </div>
                    </div>
                  </div>
                )}

                {!preview.conflict && (
                  <div className="flex items-center gap-2 text-[12px] text-[var(--color-success)]">
                    <CheckCircle className="h-3.5 w-3.5" />
                    目标目录可用
                  </div>
                )}
              </div>
            )}

            <div className="flex justify-end gap-2.5 pt-2">
              <Dialog.Close asChild>
                <button
                  type="button"
                  className="rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-4 py-[7px] text-[13px] font-medium text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
                >
                  取消
                </button>
              </Dialog.Close>
              <button
                type="submit"
                disabled={
                  isPending ||
                  !selectedDirId ||
                  enabledDirs.length === 0 ||
                  (!!preview?.conflict && !overwrite)
                }
                className="flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--color-accent-muted)] px-4 py-[7px] text-[13px] font-medium text-white transition-colors hover:bg-[var(--color-accent)] disabled:opacity-40 active:scale-[0.98]"
              >
                {isPending && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
                {isPending ? "安装中..." : "安装"}
              </button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
