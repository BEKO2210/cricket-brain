import { en, type Dictionary } from '@/i18n/locales/en';

export const supportedLocales = ['en'] as const;
export type SupportedLocale = (typeof supportedLocales)[number];

const localeLoaders: Record<SupportedLocale, () => Promise<Dictionary>> = {
  en: async () => en
};

const isSupportedLocale = (locale: string): locale is SupportedLocale =>
  supportedLocales.includes(locale as SupportedLocale);

export const loadDictionary = async (locale: string): Promise<Dictionary> => {
  if (!isSupportedLocale(locale)) {
    return localeLoaders.en();
  }
  return localeLoaders[locale]();
};

export const getDictionary = (lang = 'en'): Dictionary => {
  if (!isSupportedLocale(lang)) {
    return en;
  }
  return en;
};

export const t = (dict: Dictionary, key: string): string => {
  const resolved = key.split('.').reduce<unknown>((acc, segment) => {
    if (acc && typeof acc === 'object' && segment in acc) {
      return (acc as Record<string, unknown>)[segment];
    }
    return undefined;
  }, dict);

  return typeof resolved === 'string' ? resolved : key;
};
