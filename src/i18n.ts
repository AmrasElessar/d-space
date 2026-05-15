// SPDX-License-Identifier: GPL-3.0-or-later
//
// vue-i18n v10 setup — Bölüm 19 (Çoklu Dil Stratejisi).
// Master dil TR. İlk çeviri EN. Lazy-load locale dosyaları v0.2'de
// (10+ dil hedefi, Bölüm 19.3); şu an iki dil JSON inline.

import { createI18n } from "vue-i18n";
import tr from "./locales/tr.json";
import en from "./locales/en.json";

export type SupportedLocale = "tr" | "en";

export const SUPPORTED_LOCALES: SupportedLocale[] = ["tr", "en"];

export const i18n = createI18n({
  legacy: false,
  locale: "tr",
  fallbackLocale: "tr",
  messages: { tr, en },
  globalInjection: true,
  missingWarn: false,
  fallbackWarn: false,
});

export function setLocale(locale: SupportedLocale) {
  i18n.global.locale.value = locale;
}
