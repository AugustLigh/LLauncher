import "./LanguageSelector.css";

// Endonyms: a language is listed in itself, so anyone can find their own
// entry regardless of the UI language currently in effect.
const LANGUAGES = [
  { code: "en-us", name: "English" },
  { code: "ru-ru", name: "Русский" },
  { code: "zh-cn", name: "简体中文" },
  { code: "zh-tw", name: "繁體中文" },
  { code: "ja-jp", name: "日本語" },
  { code: "ko-kr", name: "한국어" },
  { code: "de-de", name: "Deutsch" },
  { code: "fr-fr", name: "Français" },
  { code: "es-es", name: "Español" },
  { code: "pt-br", name: "Português (Brasil)" },
  { code: "id-id", name: "Bahasa Indonesia" },
  { code: "vi-vn", name: "Tiếng Việt" },
  { code: "th-th", name: "ไทย" },
];

export default function LanguageSelector({ value, onChange, id }) {
  return (
    <select
      id={id}
      className="language-selector"
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {LANGUAGES.map((lang) => (
        <option key={lang.code} value={lang.code}>
          {lang.name}
        </option>
      ))}
    </select>
  );
}
