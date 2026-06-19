import { useQuery } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";

/**
 * Walk a directory tree for the drawer.
 *
 * `path` 为 undefined / 空串时禁用查询；其他时候 30 秒内视为新鲜
 * （避免每次重开抽屉都重打 Rust 后端）。
 */
export function useDirectoryTree(path: string | undefined | null) {
  return useQuery({
    queryKey: ["directory-tree", path],
    queryFn: () => ipc.listDirectoryTree(path!),
    enabled: !!path,
    staleTime: 30_000,
  });
}
