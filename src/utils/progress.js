// Pack downloads report a zero-based current file; VFS updates report the
// number of completed files, and verification has no byte totals.
export function progressDetails(progress) {
  const finite = (value) => (Number.isFinite(value) ? Math.max(0, value) : 0);
  const totalFiles = finite(progress.total_files);
  const completedFiles = progress.files_done != null;
  const currentFile = Math.min(
    totalFiles,
    completedFiles
      ? finite(progress.files_done)
      : finite(progress.file_index) + 1,
  );
  const byFiles = progress.stage === "verifying" && completedFiles;
  const done = byFiles
    ? currentFile
    : finite(
        progress.stage === "extracting"
          ? progress.bytes_processed
          : progress.bytes_downloaded,
      );
  const total = byFiles ? totalFiles : finite(progress.bytes_total);
  const rawPercent =
    progress.stage === "extracting"
      ? finite(progress.percent)
      : total > 0
        ? Math.round((done / total) * 100)
        : 0;
  return {
    currentFile,
    totalFiles,
    byFiles,
    done,
    total,
    percent: Math.min(100, rawPercent),
  };
}
