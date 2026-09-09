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
}) {
  if (running) return "running";
  if (launching) return "launching";
  if (importing) return "importing";
  if (["running", "pausing", "cancelling"].includes(protonTask?.status))
    return "protonDownloading";
  if (["running", "pausing", "cancelling"].includes(task?.status))
    return task.status === "running"
      ? task.progress?.stage || "fetching"
      : "pausing";
  if (task?.status === "paused") return "paused";
  if (task?.status === "error") return "downloadError";
  if (protonTask?.status === "error" || protonTask?.status === "paused")
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
