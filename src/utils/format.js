export function formatSize(bytes) {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

export function formatSpeed(bytesPerSecond) {
  if (!bytesPerSecond || bytesPerSecond <= 0) return "0 B/s";
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  const i = Math.floor(Math.log(bytesPerSecond) / Math.log(1024));
  return `${(bytesPerSecond / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

export function formatEta(bytesRemaining, speedBps, locale = "en") {
  if (!(speedBps > 0) || !(bytesRemaining > 0)) return "";
  let seconds = Math.ceil(bytesRemaining / speedBps);
  const h = Math.floor(seconds / 3600);
  seconds %= 3600;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  const isZh = locale === "zh-cn" || (typeof locale === "string" && locale.startsWith("zh"));
  const isJa = typeof locale === "string" && locale.startsWith("ja");
  const isId = typeof locale === "string" && locale.startsWith("id");
  const isKo = typeof locale === "string" && locale.startsWith("ko");
  const isVi = typeof locale === "string" && locale.startsWith("vi");
  const isTh = typeof locale === "string" && locale.startsWith("th");
  const units = locale === "ru" ? ["ч", "мин", "с"] : isZh ? ["小时", "分", "秒"] : isJa ? ["時間", "分", "秒"] : isId ? ["jam", "mnt", "dtk"] : isKo ? ["시간", "분", "초"] : isVi ? ["giờ", "phút", "giây"] : isTh ? ["ชม.", "นาที", "วิ."] : ["h", "m", "s"];
  if (h > 0) return `${h} ${units[0]} ${m} ${units[1]}`;
  if (m > 0) return `${m} ${units[1]} ${s} ${units[2]}`;
  return `${s} ${units[2]}`;
}

export function formatPercent(current, total) {
  if (total === 0) return "0%";
  return `${Math.round((current / total) * 100)}%`;
}

export function formatPlaytime(totalSecs, locale = "en") {
  const isZh = locale === "zh-cn" || (typeof locale === "string" && locale.startsWith("zh"));
  const isJa = typeof locale === "string" && locale.startsWith("ja");
  const isId = typeof locale === "string" && locale.startsWith("id");
  const isKo = typeof locale === "string" && locale.startsWith("ko");
  const isVi = typeof locale === "string" && locale.startsWith("vi");
  const isTh = typeof locale === "string" && locale.startsWith("th");
  const [hours, minutes] = locale === "ru" ? ["ч", "мин"] : isZh ? ["小时", "分钟"] : isJa ? ["時間", "分"] : isId ? ["jam", "menit"] : isKo ? ["시간", "분"] : isVi ? ["giờ", "phút"] : isTh ? ["ชม.", "นาที"] : ["h", "m"];
  if (!totalSecs || totalSecs <= 0) return `0 ${minutes}`;
  const h = Math.floor(totalSecs / 3600);
  const m = Math.floor((totalSecs % 3600) / 60);
  if (h > 0) return `${h} ${hours} ${m} ${minutes}`;
  return `${m} ${minutes}`;
}

export function formatDate(unixSecs, locale) {
  if (!unixSecs) return "";
  return new Date(unixSecs * 1000).toLocaleDateString(locale);
}
