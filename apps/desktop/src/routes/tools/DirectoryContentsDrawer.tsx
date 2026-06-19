import { useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  ChevronRight,
  Folder,
  FolderOpen,
  FileText,
  FileCode,
  File as FileIcon,
  Sparkles,
  AlertTriangle,
  ExternalLink,
  RefreshCw,
  FolderTree,
} from "lucide-react";
import type { AiToolDirectory, FileTreeNode } from "@skills-pp/shared";
import { Drawer } from "../../components/ui/Drawer";
import { useDirectoryTree } from "../../hooks/use-directory-tree";
import { ipc } from "../../lib/ipc";
import { useToast } from "../../components/ui/toast";

interface Props {
  directory: AiToolDirectory | null;
  onClose: () => void;
}

const SKILL_CLICKABLE_FILE_EXT = new Set(["md", "mdx", "yaml", "yml", "txt"]);

function fileIconFor(name: string) {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (ext === "md" || ext === "mdx" || ext === "txt") return FileText;
  if (
    ext === "js" || ext === "ts" || ext === "tsx" || ext === "py" ||
    ext === "sh" || ext === "rs" || ext === "json" || ext === "toml"
  ) {
    return FileCode;
  }
  return FileIcon;
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function DirectoryContentsDrawer({ directory, onClose }: Props) {
  const open = directory !== null;
  const toast = useToast();

  async function handleOpenFolder(path: string) {
    try {
      await ipc.openSkillDir(path);
    } catch {
      toast("无法打开目录", path, "error");
    }
  }

  return (
    <Drawer
      open={open}
      onOpenChange={(o) => {
        if (!o) onClose();
      }}
      title={
        directory && (
          <DrawerHeader
            directory={directory}
            onOpenFolder={() => handleOpenFolder(directory.path)}
          />
        )
      }
    >
      {directory && (
        <DrawerBody key={directory.id} directory={directory} />
      )}
    </Drawer>
  );
}

function DrawerHeader({
  directory,
  onOpenFolder,
}: {
  directory: AiToolDirectory;
  onOpenFolder: () => void;
}) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-center gap-2">
        <h2 className="text-[14px] font-semibold text-[var(--color-text-primary)]">
          {directory.toolName}
        </h2>
        {directory.isDefault && (
          <span className="rounded-full bg-[var(--color-accent-subtle)] px-2 py-[1px] text-[11px] text-[var(--color-accent)]">
            默认
          </span>
        )}
        <button
          onClick={onOpenFolder}
          className="ml-auto flex items-center gap-1 rounded-[var(--radius-sm)] border border-[var(--color-border-subtle)] px-2 py-[3px] text-[11px] text-[var(--color-text-secondary)] transition-colors hover:bg-[var(--color-surface-hover)] hover:text-[var(--color-text-primary)]"
          title="在系统文件管理器中打开"
        >
          <ExternalLink className="h-3 w-3" />
          打开
        </button>
      </div>
      <p
        className="truncate font-mono text-[11px] text-[var(--color-text-tertiary)]"
        title={directory.path}
      >
        {directory.path}
      </p>
      <p className="text-[11px] text-[var(--color-text-tertiary)]">
        {directory.isDetected
          ? `已检测到 · ${directory.skillCount ?? 0} 个 skill`
          : "目录未找到"}
      </p>
    </div>
  );
}

function DrawerBody({ directory }: { directory: AiToolDirectory }) {
  if (!directory.isDetected) {
    return (
      <div className="flex flex-col items-center gap-3 px-6 py-12 text-center">
        <div className="flex h-10 w-10 items-center justify-center rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-raised)]">
          <AlertTriangle className="h-4 w-4 text-[var(--color-warning)]" />
        </div>
        <p className="text-[12px] text-[var(--color-text-secondary)]">
          目录未找到
        </p>
        <p className="font-mono text-[11px] text-[var(--color-text-tertiary)]">
          {directory.path}
        </p>
      </div>
    );
  }

  return (
    <DirectoryTreeBody path={directory.path} />
  );
}

function DirectoryTreeBody({ path }: { path: string }) {
  const { data: tree, isLoading, isError, error, refetch, isFetching } =
    useDirectoryTree(path);

  if (isLoading) {
    return (
      <div className="space-y-2 px-3 py-4">
        {Array.from({ length: 8 }).map((_, i) => (
          <div
            key={i}
            className="h-4 animate-pulse rounded bg-[var(--color-border-subtle)]"
            style={{ width: `${60 + ((i * 17) % 30)}%` }}
          />
        ))}
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex flex-col items-center gap-3 px-6 py-12 text-center">
        <AlertTriangle className="h-4 w-4 text-[var(--color-danger)]" />
        <p className="text-[12px] text-[var(--color-text-secondary)]">
          读取目录失败
        </p>
        <p className="font-mono text-[11px] text-[var(--color-text-tertiary)]">
          {String(error)}
        </p>
        <button
          onClick={() => refetch()}
          className="flex items-center gap-1.5 rounded-[var(--radius-sm)] border border-[var(--color-border-default)] px-2 py-[3px] text-[11px] text-[var(--color-text-secondary)] hover:bg-[var(--color-surface-hover)]"
        >
          <RefreshCw className={`h-3 w-3 ${isFetching ? "animate-spin" : ""}`} />
          重试
        </button>
      </div>
    );
  }

  if (!tree) {
    return (
      <div className="flex flex-col items-center gap-3 px-6 py-12 text-center">
        <FolderTree className="h-4 w-4 text-[var(--color-text-tertiary)]" />
        <p className="text-[12px] text-[var(--color-text-tertiary)]">空目录</p>
      </div>
    );
  }

  // Root node's children = top-level entries.
  const children = tree.children ?? [];
  if (children.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 px-6 py-12 text-center">
        <FolderTree className="h-4 w-4 text-[var(--color-text-tertiary)]" />
        <p className="text-[12px] text-[var(--color-text-tertiary)]">
          目录为空
        </p>
      </div>
    );
  }

  return (
    <div className="py-2">
      {tree.truncated && (
        <div className="mb-1 px-4 py-1 text-[11px] text-[var(--color-text-tertiary)]">
          列表过长，部分内容已截断
        </div>
      )}
      <TreeViewChildren nodes={children} depth={0} />
    </div>
  );
}

function TreeViewChildren({
  nodes,
  depth,
}: {
  nodes: FileTreeNode[];
  depth: number;
}) {
  return (
    <ul role={depth === 0 ? "tree" : "group"}>
      {nodes.map((node) => (
        <TreeNode key={node.relativePath} node={node} depth={depth} />
      ))}
    </ul>
  );
}

function TreeNode({
  node,
  depth,
}: {
  node: FileTreeNode;
  depth: number;
}) {
  const navigate = useNavigate();
  const [expanded, setExpanded] = useState<boolean>(
    // Default-expand top-level skill folders for discoverability.
    depth === 0 && node.kind === "dir" && node.isSkill,
  );

  const isDir = node.kind === "dir";
  const indentPx = 8 + depth * 14;

  function handleActivate() {
    if (isDir) {
      // Skill folder → jump to local-skill page; plain folder → toggle.
      if (node.isSkill) {
        navigate("/local-skill", {
          state: { absolutePath: node.absolutePath, name: node.name },
        });
        return;
      }
      setExpanded((v) => !v);
      return;
    }
    // File: jump only for previewable text types.
    const ext = node.name.split(".").pop()?.toLowerCase() ?? "";
    if (SKILL_CLICKABLE_FILE_EXT.has(ext)) {
      navigate("/local-skill", {
        state: { absolutePath: node.absolutePath, name: node.name },
      });
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleActivate();
    }
  }

  const showExpandChevron = isDir && !node.isSkill;
  const showSkillBadge = isDir && node.isSkill;
  const children = node.children ?? [];
  const showChildren = isDir && expanded && children.length > 0;

  const Icon = !isDir
    ? fileIconFor(node.name)
    : expanded
      ? FolderOpen
      : Folder;

  return (
    <li role="treeitem" aria-expanded={isDir ? expanded : undefined}>
      <div
        role="button"
        tabIndex={0}
        onClick={handleActivate}
        onKeyDown={handleKeyDown}
        className={`group flex cursor-pointer items-center gap-1.5 py-[5px] pr-3 transition-colors hover:bg-[var(--color-surface-hover)] ${
          showSkillBadge ? "bg-[var(--color-accent-subtle)]/40" : ""
        }`}
        style={{ paddingLeft: `${indentPx}px` }}
        title={node.absolutePath}
      >
        {showExpandChevron ? (
          <ChevronRight
            className={`h-3 w-3 shrink-0 text-[var(--color-text-tertiary)] transition-transform ${
              expanded ? "rotate-90" : ""
            }`}
          />
        ) : (
          <span className="inline-block w-[12px] shrink-0" />
        )}
        <Icon
          className={`h-3.5 w-3.5 shrink-0 ${
            showSkillBadge
              ? "text-[var(--color-accent)]"
              : "text-[var(--color-text-tertiary)]"
          }`}
        />
        <span
          className={`min-w-0 flex-1 truncate text-[12px] ${
            showSkillBadge
              ? "font-medium text-[var(--color-text-primary)]"
              : "text-[var(--color-text-secondary)]"
          }`}
        >
          {node.name}
        </span>
        {showSkillBadge && (
          <Sparkles className="h-3 w-3 shrink-0 text-[var(--color-accent)]" />
        )}
        {!isDir && node.size > 0 && (
          <span className="shrink-0 text-[10px] text-[var(--color-text-tertiary)]">
            {formatSize(node.size)}
          </span>
        )}
      </div>

      {node.error && (
        <div
          className="px-3 pb-1 text-[11px] text-[var(--color-danger)]"
          style={{ paddingLeft: `${indentPx + 20}px` }}
        >
          {node.error}
        </div>
      )}

      {node.truncated && !showChildren && isDir && (
        <div
          className="py-[3px] text-[11px] text-[var(--color-text-tertiary)]"
          style={{ paddingLeft: `${indentPx + 20}px` }}
        >
          …
        </div>
      )}

      {showChildren && (
        <TreeViewChildren nodes={children} depth={depth + 1} />
      )}
    </li>
  );
}
