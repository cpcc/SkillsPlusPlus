import { useEffect, useState } from "react";
import { RefreshCw, PlusCircle } from "lucide-react";
import { openPath } from "@tauri-apps/plugin-opener";
import type { AiToolDirectory } from "@skills-pp/shared";
import {
  useDirectories,
  useScanDirectories,
  useAddDirectory,
  useToggleDirectory,
  useSetDefaultDirectory,
  useDeleteDirectory,
} from "../../hooks/use-directories";
import { DirectoryCard } from "./DirectoryCard";
import { AddDirectoryDialog } from "./AddDirectoryDialog";
import { useToast } from "../../components/ui/toast";

function groupByTool(dirs: AiToolDirectory[]): Map<string, AiToolDirectory[]> {
  const map = new Map<string, AiToolDirectory[]>();
  for (const d of dirs) {
    const list = map.get(d.toolName) ?? [];
    list.push(d);
    map.set(d.toolName, list);
  }
  return map;
}

export default function ToolsPage() {
  const { data: dirs = [], isLoading } = useDirectories();
  const scan = useScanDirectories();
  const add = useAddDirectory();
  const toggle = useToggleDirectory();
  const setDefault = useSetDefaultDirectory();
  const del = useDeleteDirectory();
  const toast = useToast();
  const [dialogOpen, setDialogOpen] = useState(false);

  // Auto-scan on first mount
  useEffect(() => {
    scan.mutate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleOpenFolder(path: string) {
    openPath(path).catch(() => toast("无法打开目录", path, "error"));
  }

  function handleAdd(toolName: string, path: string) {
    add.mutate(
      { toolName, path },
      {
        onSuccess: () => {
          setDialogOpen(false);
          toast("目录已添加", path);
        },
        onError: (e) => toast("添加失败", String(e), "error"),
      },
    );
  }

  function handleToggle(id: string, enabled: boolean) {
    toggle.mutate(
      { id, enabled },
      { onError: (e) => toast("操作失败", String(e), "error") },
    );
  }

  function handleSetDefault(id: string) {
    setDefault.mutate(id, {
      onSuccess: () => toast("已设为默认目录"),
      onError: (e) => toast("操作失败", String(e), "error"),
    });
  }

  function handleDelete(id: string) {
    del.mutate(id, {
      onError: (e) => toast("删除失败", String(e), "error"),
    });
  }

  const grouped = groupByTool(dirs);
  const detectedCount = dirs.filter((d) => d.isDetected).length;

  return (
    <div>
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-gray-900">工具与目录</h2>
          <p className="mt-1 text-sm text-gray-500">
            {isLoading || scan.isPending
              ? "扫描中..."
              : `共 ${dirs.length} 个目录，已找到 ${detectedCount} 个`}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => scan.mutate()}
            disabled={scan.isPending}
            className="flex items-center gap-2 rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-60"
          >
            <RefreshCw
              className={`h-4 w-4 ${scan.isPending ? "animate-spin" : ""}`}
            />
            重新扫描
          </button>
          <button
            onClick={() => setDialogOpen(true)}
            className="flex items-center gap-2 rounded-lg bg-brand-600 px-3 py-2 text-sm font-medium text-white hover:bg-brand-700"
          >
            <PlusCircle className="h-4 w-4" />
            新增目录
          </button>
        </div>
      </div>

      {isLoading && dirs.length === 0 ? (
        <div className="mt-12 text-center text-sm text-gray-400">加载中...</div>
      ) : (
        <div className="mt-6 space-y-6">
          {Array.from(grouped.entries()).map(([toolName, toolDirs]) => (
            <div key={toolName}>
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-400">
                {toolName}
              </h3>
              <div className="space-y-2">
                {toolDirs.map((d) => (
                  <DirectoryCard
                    key={d.id}
                    dir={d}
                    onToggle={handleToggle}
                    onSetDefault={handleSetDefault}
                    onDelete={handleDelete}
                    onOpenFolder={handleOpenFolder}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      <AddDirectoryDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onAdd={handleAdd}
        isPending={add.isPending}
      />
    </div>
  );
}
