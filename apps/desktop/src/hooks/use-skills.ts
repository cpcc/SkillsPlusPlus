import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";

export const SKILLS_KEY = ["skills"] as const;

export function useSkills() {
  return useQuery({
    queryKey: SKILLS_KEY,
    queryFn: () => ipc.listSkills(),
  });
}

export function useRefreshAllSources() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.refreshAllSources(),
    onSuccess: (data) => qc.setQueryData(SKILLS_KEY, data),
  });
}

export function useRefreshSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (sourceId: string) => ipc.refreshSource(sourceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: SKILLS_KEY }),
  });
}

export function useSkill(id: string) {
  return useQuery({
    queryKey: ["skill", id],
    queryFn: () => ipc.getSkill(id),
    enabled: !!id,
  });
}

export function useSkillMd(id: string) {
  return useQuery({
    queryKey: ["skill-md", id],
    queryFn: () => ipc.fetchSkillMd(id),
    enabled: !!id,
    staleTime: 1000 * 60 * 30, // 30 min
  });
}
