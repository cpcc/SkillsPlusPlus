import * as Dialog from "@radix-ui/react-dialog";
import { useState } from "react";
import { X, ChevronDown } from "lucide-react";

const TOOL_NAMES = [
  "Codex",
  "Claude",
  "Cursor",
  "OpenCode",
  "GitHub Copilot",
  "Antigravity",
  "Gemini CLI",
  "Kimi Code CLI",
  "OpenClaw",
  "CodeBuddy",
  "其他",
];

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdd: (toolName: string, path: string) => void;
  isPending: boolean;
}

const inputCls =
  "w-full rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-3 py-2 text-[13px] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] transition-colors focus:border-[var(--color-accent)] focus:outline-none";

const selectCls =
  "w-full appearance-none rounded-[var(--radius-md)] border border-[var(--color-border-default)] bg-[var(--color-surface-raised)] px-3 py-2 pr-8 text-[13px] text-[var(--color-text-primary)] transition-colors focus:border-[var(--color-accent)] focus:outline-none cursor-pointer";

export function AddDirectoryDialog({
  open,
  onOpenChange,
  onAdd,
  isPending,
}: Props) {
  const [toolName, setToolName] = useState(TOOL_NAMES[0]);
  const [customTool, setCustomTool] = useState("");
  const [path, setPath] = useState("");

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const finalTool = toolName === "其他" ? customTool.trim() : toolName;
    if (!finalTool || !path.trim()) return;
    onAdd(finalTool, path.trim());
  }

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm" />
        <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-[var(--radius-xl)] border border-[var(--color-border-default)] bg-[var(--color-surface-overlay)] p-6 shadow-2xl shadow-black/30">
          <div className="flex items-center justify-between">
            <Dialog.Title className="text-[15px] font-semibold text-[var(--color-text-primary)]">
              新增目录
            </Dialog.Title>
            <Dialog.Close asChild>
              <button className="rounded-[var(--radius-sm)] p-1 text-[var(--color-text-tertiary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-secondary)]">
                <X className="h-4 w-4" />
              </button>
            </Dialog.Close>
          </div>

          <form onSubmit={handleSubmit} className="mt-5 space-y-4">
            <div>
              <label className="mb-1.5 block text-[12px] font-medium text-[var(--color-text-secondary)]">
                AI 工具
              </label>
              <div className="relative">
                <select
                  className={selectCls}
                  value={toolName}
                  onChange={(e) => setToolName(e.target.value)}
                >
                  {TOOL_NAMES.map((t) => (
                    <option key={t} value={t}>
                      {t}
                    </option>
                  ))}
                </select>
                <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--color-text-tertiary)]" />
              </div>
            </div>

            {toolName === "其他" && (
              <div>
                <label className="mb-1.5 block text-[12px] font-medium text-[var(--color-text-secondary)]">
                  工具名称
                </label>
                <input
                  type="text"
                  className={inputCls}
                  placeholder="例如：MyCopilot"
                  value={customTool}
                  onChange={(e) => setCustomTool(e.target.value)}
                  required
                />
              </div>
            )}

            <div>
              <label className="mb-1.5 block text-[12px] font-medium text-[var(--color-text-secondary)]">
                目录路径
              </label>
              <input
                type="text"
                className={`${inputCls} font-mono text-[12px]`}
                placeholder="例如：/Users/you/.mytool/skills"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                required
              />
            </div>

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
                disabled={isPending}
                className="rounded-[var(--radius-md)] bg-[var(--color-accent-muted)] px-4 py-[7px] text-[13px] font-medium text-white transition-colors hover:bg-[var(--color-accent)] disabled:opacity-40 active:scale-[0.98]"
              >
                {isPending ? "添加中..." : "添加"}
              </button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
