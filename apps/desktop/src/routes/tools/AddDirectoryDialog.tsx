import * as Dialog from "@radix-ui/react-dialog";
import { useState } from "react";
import { X } from "lucide-react";

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
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/30" />
        <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-xl bg-white p-6 shadow-xl">
          <div className="flex items-center justify-between">
            <Dialog.Title className="text-base font-semibold text-gray-900">
              新增目录
            </Dialog.Title>
            <Dialog.Close asChild>
              <button className="rounded p-1 text-gray-400 hover:bg-gray-100">
                <X className="h-4 w-4" />
              </button>
            </Dialog.Close>
          </div>

          <form onSubmit={handleSubmit} className="mt-4 space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">
                AI 工具
              </label>
              <select
                className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                value={toolName}
                onChange={(e) => setToolName(e.target.value)}
              >
                {TOOL_NAMES.map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </select>
            </div>

            {toolName === "其他" && (
              <div>
                <label className="block text-sm font-medium text-gray-700">
                  工具名称
                </label>
                <input
                  type="text"
                  className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                  placeholder="例如：MyCopilot"
                  value={customTool}
                  onChange={(e) => setCustomTool(e.target.value)}
                  required
                />
              </div>
            )}

            <div>
              <label className="block text-sm font-medium text-gray-700">
                目录路径
              </label>
              <input
                type="text"
                className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                placeholder="例如：/Users/you/.mytool/skills"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                required
              />
            </div>

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
                disabled={isPending}
                className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white hover:bg-brand-700 disabled:opacity-60"
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
