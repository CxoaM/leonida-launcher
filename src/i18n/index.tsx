import { createContext, useContext, useMemo, type ReactNode } from "react";
import { en, ru, type Dictionary, type MessageKey } from "./dictionary";

export type Language = "en" | "ru" | "system";
export type ResolvedLanguage = "en" | "ru";

const dictionaries: Record<ResolvedLanguage, Dictionary> = { en, ru };

export function resolveLanguage(language: Language | undefined): ResolvedLanguage {
  if (language === "ru") return "ru";
  if (language === "en") return "en";
  const navigatorLanguage =
    typeof navigator !== "undefined" ? navigator.language ?? "" : "";
  return navigatorLanguage.toLowerCase().startsWith("ru") ? "ru" : "en";
}

export type TranslateVars = Record<string, string | number>;
export type Translate = (key: MessageKey, vars?: TranslateVars) => string;

/** Locale tag used for dates, times and numbers, kept in sync with the UI language. */
export function dateLocale(language: ResolvedLanguage): string {
  return language === "ru" ? "ru-RU" : "en-US";
}

export function createTranslator(language: ResolvedLanguage): Translate {
  const dictionary = dictionaries[language];
  return (key, vars) => {
    // Falling back to English keeps a missing translation readable instead of
    // showing a raw key, but the typed dictionary makes that path unreachable.
    const template = dictionary[key] ?? en[key] ?? key;
    if (!vars) return template;
    return template.replace(/\{(\w+)\}/g, (match, name: string) => {
      const value = vars[name];
      return value === undefined ? match : String(value);
    });
  };
}

interface I18nValue {
  language: Language;
  resolved: ResolvedLanguage;
  t: Translate;
}

const fallback: I18nValue = {
  language: "en",
  resolved: "en",
  t: createTranslator("en"),
};

const I18nContext = createContext<I18nValue>(fallback);

export function I18nProvider({
  language,
  children,
}: {
  language: Language | undefined;
  children: ReactNode;
}) {
  const resolved = resolveLanguage(language);
  const value = useMemo<I18nValue>(
    () => ({ language: language ?? "system", resolved, t: createTranslator(resolved) }),
    [language, resolved],
  );
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  return useContext(I18nContext);
}

export function useT(): Translate {
  return useContext(I18nContext).t;
}
