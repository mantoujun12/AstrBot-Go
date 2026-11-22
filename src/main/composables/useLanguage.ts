import { useI18n } from "vue-i18n";
import { useLocale as useVuetifyLocale } from "vuetify";
import { applyLanguagePreference } from "../i18n";
import type { LanguagePreference } from "../types/launcher";

export function useLanguage() {
  const { locale: i18nLocale } = useI18n();
  const { current: vuetifyLocale } = useVuetifyLocale();

  function apply(preference: LanguagePreference): void {
    const locale = applyLanguagePreference(preference);
    i18nLocale.value = locale;
    vuetifyLocale.value = locale;
  }

  return { apply };
}
