import { useQuery } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";

interface LocalSkillMdResult {
  /** Resolved SKILL.md absolute path (or the passed-in path if it's a file). */
  skillMdPath: string;
  content: string | null;
  /** True while the lookup+read is in flight. */
  isLoading: boolean;
  isError: boolean;
  error: unknown;
}

/**
 * Resolve and read the SKILL.md (or text file) for a local skill.
 *
 * - If `absolutePath` is a directory: walk it at depth 1 to find a
 *   case-insensitive `SKILL.md`; fall back to first `.md` at top level.
 * - If `absolutePath` is a file: read it directly.
 *
 * Returns `content: null` if no readable text file is found.
 */
export function useLocalSkillMd(
  absolutePath: string | undefined,
): LocalSkillMdResult {
  const treeQ = useQuery({
    queryKey: ["local-skill-tree", absolutePath],
    queryFn: () => ipc.listDirectoryTree(absolutePath!, 1),
    enabled: !!absolutePath,
    staleTime: 30_000,
  });

  // Determine which file to read.
  let target: string | undefined;
  if (absolutePath) {
    const lower = absolutePath.toLowerCase();
    const isFileLike =
      lower.endsWith(".md") ||
      lower.endsWith(".mdx") ||
      lower.endsWith(".yaml") ||
      lower.endsWith(".yml") ||
      lower.endsWith(".txt");
    if (isFileLike) {
      target = absolutePath;
    } else if (treeQ.data) {
      const root = treeQ.data;
      // Prefer SKILL.md (any case) at top level.
      const skillMd = (root.children ?? []).find(
        (c) =>
          c.kind === "file" &&
          c.name.toLowerCase() === "skill.md",
      );
      if (skillMd) {
        target = skillMd.absolutePath;
      } else {
        // Fall back to first top-level .md file.
        const anyMd = (root.children ?? []).find(
          (c) =>
            c.kind === "file" &&
            (c.name.toLowerCase().endsWith(".md") ||
              c.name.toLowerCase().endsWith(".mdx")),
        );
        if (anyMd) target = anyMd.absolutePath;
      }
    }
  }

  const fileQ = useQuery({
    queryKey: ["local-skill-text", target],
    queryFn: () => ipc.readTextFile(target!),
    enabled: !!target,
    staleTime: 30_000,
  });

  const isLoading =
    !!absolutePath &&
    (treeQ.isLoading ||
      fileQ.isLoading ||
      // Tree loaded but we haven't yet issued the file query.
      (!absolutePath.toLowerCase().match(/\.(md|mdx|yaml|yml|txt)$/) &&
        treeQ.data !== undefined &&
        !!target &&
        fileQ.data === undefined &&
        !fileQ.isError));
  // ^ the above "in-between" state is rare; react-query will usually flip
  //   fileQ into loading immediately. We keep it defensive to avoid a flash
  //   of "no content" UI.

  return {
    skillMdPath: target ?? absolutePath ?? "",
    content: fileQ.data ?? null,
    isLoading,
    isError: treeQ.isError || fileQ.isError,
    error: treeQ.error ?? fileQ.error,
  };
}
