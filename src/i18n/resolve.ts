// @ts-nocheck

import en from "./en.ts";
import ru from "./ru.ts";
import zhCn from "./zh-cn.ts";
import zhTw from "./zh-tw.ts";
import jaJp from "./ja-jp.ts";
import idId from "./id-id.ts";
import koKr from "./ko-kr.ts";
import deDe from "./de-de.ts";
import frFr from "./fr-fr.ts";
import esEs from "./es-es.ts";
import ptBr from "./pt-br.ts";
import viVn from "./vi-vn.ts";
import thTh from "./th-th.ts";

export const BUNDLES = {
  en,
  ru,
  "zh-cn": zhCn,
  "zh-tw": zhTw,
  "ja-jp": jaJp,
  "id-id": idId,
  "ko-kr": koKr,
  "de-de": deDe,
  "fr-fr": frFr,
  "es-es": esEs,
  "pt-br": ptBr,
  "vi-vn": viVn,
  "th-th": thTh,
};

export function resolveLocale(language) {
  if (!language) return "en";
  const lower = language.toLowerCase();
  if (lower.startsWith("ru")) return "ru";
  if (lower.startsWith("zh-tw") || lower.startsWith("zh-hk") || lower.startsWith("zh-hant")) return "zh-tw";
  if (lower.startsWith("zh")) return "zh-cn";
  if (lower.startsWith("ja")) return "ja-jp";
  if (lower.startsWith("id")) return "id-id";
  if (lower.startsWith("ko")) return "ko-kr";
  if (lower.startsWith("de")) return "de-de";
  if (lower.startsWith("fr")) return "fr-fr";
  if (lower.startsWith("es")) return "es-es";
  if (lower.startsWith("pt")) return "pt-br";
  if (lower.startsWith("vi")) return "vi-vn";
  if (lower.startsWith("th")) return "th-th";
  return "en";
}

export function getByPath(obj, path) {
  const parts = path.split(".");
  let cur = obj;
  for (const p of parts) {
    if (cur == null) return undefined;
    cur = cur[p];
  }
  return cur;
}

export function format(template, vars) {
  if (!vars || typeof template !== "string") return template;
  return template.replace(/\{(\w+)\}/g, (m, key) =>
    vars[key] != null ? String(vars[key]) : m,
  );
}
