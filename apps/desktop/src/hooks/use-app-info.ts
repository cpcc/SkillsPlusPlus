import { useQuery } from "@tanstack/react-query";
import { ipc } from "../lib/ipc";

export function useAppInfo() {
  return useQuery({
    queryKey: ["app-info"],
    queryFn: () => ipc.getAppInfo(),
    staleTime: Infinity,
  });
}
