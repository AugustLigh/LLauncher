import { createContext, useContext, useMemo } from 'react';
import { BUNDLES, resolveLocale, getByPath, format } from './resolve.js';

export { resolveLocale, BUNDLES };

const I18nContext = createContext({
  locale: 'en',
  t: (key) => key,
});

export function I18nProvider({ language, children }) {
  const value = useMemo(() => {
    const locale = resolveLocale(language);
    const bundle = BUNDLES[locale] || BUNDLES.en;
    const t = (key, vars) => {
      const value = getByPath(bundle, key);
      if (value === undefined) {
        const fallback = getByPath(BUNDLES.en, key);
        return format(fallback !== undefined ? fallback : key, vars);
      }
      return format(value, vars);
    };
    return { locale, t };
  }, [language]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useTranslation() {
  return useContext(I18nContext);
}
