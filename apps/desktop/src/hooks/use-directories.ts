import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";

const QUERY_KEY = ["directories"] as const;

export function useDirectories() {
  return useQuery({
    queryKey: QUERY_KEY,
    queryFn: () => ipc.listDirectories(),
  });
}

export function useScanDirectories() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.scanDirectories(),
    onSuccess: (data) => {
      qc.setQueryData(QUERY_KEY, data);
    },
  });
}

export function useAddDirectory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ toolName, path }: { toolName: string; path: string }) =>
      ipc.addDirectory(toolName, path),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
    },
  });
}

export function useToggleDirectory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      ipc.toggleDirectory(id, enabled),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
    },
  });
}

export function useSetDefaultDirectory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => ipc.setDefaultDirectory(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
    },
  });
}

export function useDeleteDirectory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => ipc.deleteDirectory(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
    },
  });
}
