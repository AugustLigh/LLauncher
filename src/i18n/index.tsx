import { createContext, useContext, useMemo, ReactNode } from 'react';
import { BUNDLES, resolveLocale, getByPath, format } from './resolve';

export { resolveLocale, BUNDLES };

export interface I18nContextValue {
  locale: string;
  t: (key: string, vars?: Record<string, any>) => string;
}

const I18nContext = createContext<I18nContextValue>({
  locale: 'en',
  t: (key) => key,
});

export function I18nProvider({ language, children }: { language?: string | null; children: ReactNode }) {
  const value = useMemo(() => {
    const locale = resolveLocale(language);
    const bundle = BUNDLES[locale] || BUNDLES.en;
    const t = (key: string, vars?: Record<string, any>): string => {
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
