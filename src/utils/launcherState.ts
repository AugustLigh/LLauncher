import type { GameState, SystemCheck, Transfer } from '../bindings';

export interface LauncherStateParams {
  running?: boolean;
  launching?: boolean;
  importing?: boolean;
  gameLoading?: boolean;
  gameError?: boolean | string | null | unknown;
  systemLoading?: boolean;
  systemError?: boolean | string | null | unknown;
  systemCheck?: SystemCheck | null;
  gameState?: GameState | null;
  task?: Transfer | null;
  protonTask?: Transfer | null;
  tasksLoading?: boolean;
  tasksError?: boolean | string | null | unknown;
}

export function launcherState({
  running,
  launching,
  importing,
  gameLoading,
  gameError,
  systemLoading,
  systemError,
  systemCheck,
  gameState,
  task,
  protonTask,
  tasksLoading,
  tasksError,
}: LauncherStateParams): string {
  if (running) return "running";
  if (launching) return "launching";
  if (importing) return "importing";
  if (["running", "pausing", "cancelling"].includes(protonTask?.status as string))
    return "protonDownloading";
  if (["running", "pausing", "cancelling"].includes(task?.status as string))
    return task?.status === "running"
      ? task?.progress?.stage || "fetching"
      : "pausing";
  if (task?.status === "paused") return "paused";
  if (task?.status === "error") return "downloadError";
  if (
    (!systemCheck || !systemCheck.has_proton) &&
    (protonTask?.status === "error" || protonTask?.status === "paused")
  )
    return "protonError";
  if (gameLoading || tasksLoading) return "checking";
  if (tasksError) return "taskError";
  if (gameError || !gameState) return "versionError";
  if (gameState.status === "not_installed") return "notInstalled";
  if (gameState.status === "update_available") return "update";
  if (systemLoading) return "systemChecking";
  if (systemError || !systemCheck) return "systemError";
  if (systemCheck.platform !== "windows" && !systemCheck.has_proton)
    return "needsProton";
  return "ready";
}
