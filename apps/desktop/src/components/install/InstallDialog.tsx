import * as Dialog from "@radix-ui/react-dialog";
import { useState, useEffect } from "react";
import { X, AlertTriangle, CheckCircle, Loader2 } from "lucide-react";
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

  // Reset when dialog opens
  useEffect(() => {
    if (open) {
      setSelectedDirId(defaultDir?.id ?? "");
      setPreview(null);
      setOverwrite(false);
    }
  }, [open, defaultDir?.id]);

  // Load preview when dir selection changes
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
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/30" />
        <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-lg -translate-x-1/2 -translate-y-1/2 rounded-xl bg-white p-6 shadow-xl">
          <div className="flex items-center justify-between">
            <Dialog.Title className="text-base font-semibold text-gray-900">
              安装 Skill
            </Dialog.Title>
            <Dialog.Close asChild>
              <button className="rounded p-1 text-gray-400 hover:bg-gray-100">
                <X className="h-4 w-4" />
              </button>
            </Dialog.Close>
          </div>

          <form onSubmit={handleSubmit} className="mt-4 space-y-4">
            {/* Skill info */}
            <div className="rounded-lg bg-gray-50 p-3">
              <p className="text-sm font-medium text-gray-900">{skillName}</p>
              <p className="mt-0.5 truncate font-mono text-xs text-gray-400">
                {repoUrl}
              </p>
            </div>

            {/* Directory selector */}
            <div>
              <label className="block text-sm font-medium text-gray-700">
                安装目录
              </label>
              {enabledDirs.length === 0 ? (
                <p className="mt-1 text-xs text-red-500">
                  没有可用的安装目录，请先在「工具与目录」中配置。
                </p>
              ) : (
                <select
                  className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
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
              )}
            </div>

            {/* Preview */}
            {loadingPreview && (
              <div className="flex items-center gap-2 text-xs text-gray-400">
                <Loader2 className="h-3 w-3 animate-spin" />
                检查目标路径...
              </div>
            )}

            {preview && !loadingPreview && (
              <div className="space-y-2">
                <div className="rounded-lg border border-gray-200 p-3">
                  <p className="text-xs text-gray-400">安装到</p>
                  <p className="mt-0.5 truncate font-mono text-xs text-gray-700">
                    {preview.targetPath}
                  </p>
                </div>

                {preview.conflict && (
                  <div className="rounded-lg border border-yellow-200 bg-yellow-50 p-3">
                    <div className="flex items-start gap-2">
                      <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-yellow-600" />
                      <div>
                        <p className="text-xs font-medium text-yellow-800">
                          目录已存在
                        </p>
                        <p className="mt-0.5 text-xs text-yellow-700">
                          {preview.conflict.existingPath}
                        </p>
                        <label className="mt-2 flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={overwrite}
                            onChange={(e) => setOverwrite(e.target.checked)}
                            className="h-3.5 w-3.5"
                          />
                          <span className="text-xs text-yellow-800">
                            覆盖安装（删除现有目录）
                          </span>
                        </label>
                      </div>
                    </div>
                  </div>
                )}

                {!preview.conflict && (
                  <div className="flex items-center gap-2 text-xs text-green-600">
                    <CheckCircle className="h-3.5 w-3.5" />
                    目标目录可用
                  </div>
                )}
              </div>
            )}

            <div className="flex justify-end gap-3 pt-2">
              <Dialog.Close asChild>
                <button
                  type="button"
                  className="rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
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
                className="flex items-center gap-2 rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white hover:bg-brand-700 disabled:opacity-60"
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
