import * as Dialog from "@radix-ui/react-dialog";
import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import { useState, useEffect } from "react";
import { X, AlertTriangle, CheckCircle, Loader2, ChevronDown, Link2, Check } from "lucide-react";
import { ToolIcon } from "../ui/ToolIcon";
import type { AiToolDirectory, InstallPreview, InstallStrategy } from "@skills-pp/shared";
import { ipc } from "../../lib/ipc";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  skillName: string;
  repoUrl: string;
  skillId?: string;
  /** adapter 声明的默认策略 */
  defaultStrategy?: InstallStrategy;
  /** 切换到 copy/archive/skills_cli 时使用的归档下载地址 */
  archiveUrl?: string;
  directories: AiToolDirectory[];
  /** 已经安装了该 skill 的目录 ID 集合 */
  installedDirectoryIds?: Set<string>;
  onInstall: (directoryId: string, overwrite: boolean, strategy: InstallStrategy) => void;
  isPending: boolean;
}

export function InstallDialog({
  open,
  onOpenChange,
  skillName,
  repoUrl,
  skillId: _skillId,
  defaultStrategy = "git",
  archiveUrl,
  directories,
  installedDirectoryIds = new Set(),
  onInstall,
  isPending,
}: Props) {
  const enabledDirs = directories.filter((d) => d.enabled && d.isDetected && d.writable);
  // Sort: not-installed directories first, then installed ones
  const sortedDirs = [...enabledDirs].sort((a, b) => {
    const aInstalled = installedDirectoryIds.has(a.id) ? 1 : 0;
    const bInstalled = installedDirectoryIds.has(b.id) ? 1 : 0;
    return aInstalled - bInstalled;
  });
  // Default to the first not-installed directory (or the first dir if all are installed)
  const defaultDir =
    sortedDirs.find((d) => d.isDefault && !installedDirectoryIds.has(d.id)) ??
    sortedDirs.find((d) => d.isDefault) ??
    sortedDirs[0];

  const [selectedDirId, setSelectedDirId] = useState(defaultDir?.id ?? "");
  const [strategy, setStrategy] = useState<InstallStrategy | "">("");
  const [preview, setPreview] = useState<InstallPreview | null>(null);
  const [loadingPreview, setLoadingPreview] = useState(false);
  const [overwrite, setOverwrite] = useState(false);
  const [dirDropdownOpen, setDirDropdownOpen] = useState(false);

  useEffect(() => {
    if (open) {
      setSelectedDirId(defaultDir?.id ?? "");
      setStrategy("");
      setPreview(null);
      setOverwrite(false);
    }
  }, [open, defaultDir?.id]);

  const resolvedStrategy: InstallStrategy = strategy || defaultStrategy;

  useEffect(() => {
    if (!selectedDirId || !repoUrl) return;
    setLoadingPreview(true);
    ipc.previewInstall(skillName, repoUrl, selectedDirId, resolvedStrategy)
      .then((p: InstallPreview) => { setPreview(p); setOverwrite(false); })
      .catch(() => setPreview(null))
      .finally(() => setLoadingPreview(false));
  }, [selectedDirId, skillName, repoUrl, resolvedStrategy]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!selectedDirId) return;
    onInstall(selectedDirId, overwrite, resolvedStrategy);
  }

  const explicitNonGit = strategy !== "" && strategy !== "git";
  const missingArchiveUrl = explicitNonGit && !archiveUrl && strategy !== "skills_cli";

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

            {/* Strategy selector */}
            <div>
              <label className="mb-1.5 block text-[12px] font-medium text-[var(--color-text-secondary)]">
                安装方式
              </label>
              <div className="relative">
                <select
                  className="w-full appearance-none rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-3 py-2 pr-8 text-[13px] text-[var(--color-text-primary)] transition-colors focus:border-[var(--color-accent)] focus:outline-none cursor-pointer"
                  value={strategy}
                  onChange={(e) => setStrategy(e.target.value as InstallStrategy | "")}
                >
                  <option value="">默认</option>
                  <option value="git">Git 克隆</option>
                  <option value="copy">拷贝</option>
                  <option value="archive">压缩包</option>
                  <option value="skills_cli">软链 + 规范存储</option>
                </select>
                <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--color-text-tertiary)]" />
              </div>
              {missingArchiveUrl && (
                <p className="mt-1.5 text-[11px] text-[var(--color-warning)]">
                  该来源未提供归档下载地址，可能无法以该方式安装。
                </p>
              )}
            </div>

            {/* Directory selector */}
            <div>
              <label className="mb-1.5 block text-[12px] font-medium text-[var(--color-text-secondary)]">
                安装目录
              </label>
              {sortedDirs.length === 0 ? (
                <p className="mt-1 text-[12px] text-[var(--color-danger)]">
                  没有可用的安装目录，请先在「工具与目录」中配置。
                </p>
              ) : (
                <DropdownMenu.Root open={dirDropdownOpen} onOpenChange={setDirDropdownOpen}>
                  <DropdownMenu.Trigger asChild>
                    <button
                      type="button"
                      className="flex w-full items-center gap-2 rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] py-2 pl-2.5 pr-8 text-[13px] text-[var(--color-text-primary)] transition-colors hover:bg-[var(--color-surface-hover)] focus:border-[var(--color-accent)] focus:outline-none cursor-pointer relative"
                    >
                      {(() => {
                        const sel = sortedDirs.find((d) => d.id === selectedDirId);
                        return sel ? (
                          <>
                            <ToolIcon toolName={sel.toolName} size="sm" />
                            <span className="truncate">[{sel.toolName}] {sel.path}</span>
                            {sel.toolName === "Agents" && (
                              <span className="ml-1 shrink-0 rounded-full bg-[var(--color-accent-subtle)] px-1.5 py-[1px] text-[10px] text-[var(--color-accent-text)]">
                                通用
                              </span>
                            )}
                          </>
                        ) : null;
                      })()}
                      <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--color-text-tertiary)]" />
                    </button>
                  </DropdownMenu.Trigger>
                  <DropdownMenu.Portal>
                    <DropdownMenu.Content
                      className="z-[60] max-h-64 overflow-auto rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-overlay)] p-1 shadow-xl shadow-black/20"
                      align="start"
                      sideOffset={4}
                    >
                      {sortedDirs.map((d) => (
                        <DropdownMenu.Item
                          key={d.id}
                          className="flex cursor-pointer items-center gap-2 rounded-[var(--radius-sm)] px-2 py-1.5 text-[13px] text-[var(--color-text-primary)] outline-none data-[highlighted]:bg-[var(--color-surface-hover)]"
                          onSelect={() => setSelectedDirId(d.id)}
                        >
                          <ToolIcon toolName={d.toolName} size="sm" />
                          <span className="truncate">[{d.toolName}] {d.path}{installedDirectoryIds.has(d.id) ? " — 已安装" : ""}</span>
                          {d.toolName === "Agents" && (
                            <span className="shrink-0 rounded-full bg-[var(--color-accent-subtle)] px-1.5 py-[1px] text-[10px] text-[var(--color-accent-text)]">
                              通用
                            </span>
                          )}
                          {d.id === selectedDirId && (
                            <Check className="ml-auto h-3.5 w-3.5 shrink-0 text-[var(--color-accent)]" />
                          )}
                        </DropdownMenu.Item>
                      ))}
                    </DropdownMenu.Content>
                  </DropdownMenu.Portal>
                </DropdownMenu.Root>
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
                  <p className="text-[11px] text-[var(--color-text-tertiary)]">
                    {preview.strategy === "skills_cli" ? "将创建软链" : "安装到"}
                  </p>
                  {preview.strategy === "skills_cli" ? (
                    <div className="mt-1 space-y-1">
                      <p className="truncate font-mono text-[12px] text-[var(--color-text-secondary)]">
                        {preview.symlinkPath}
                      </p>
                      <div className="flex items-center gap-1.5 text-[10px] text-[var(--color-text-tertiary)]">
                        <Link2 className="h-3 w-3" />
                        <span className="truncate font-mono">→ {preview.canonicalPath}</span>
                      </div>
                    </div>
                  ) : (
                    <p className="mt-0.5 truncate font-mono text-[12px] text-[var(--color-text-secondary)]">
                      {preview.targetPath}
                    </p>
                  )}
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
                  sortedDirs.length === 0 ||
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
