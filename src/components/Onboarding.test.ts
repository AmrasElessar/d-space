// SPDX-License-Identifier: GPL-3.0-or-later
//
// Onboarding flow — slide ilerleme + mod seçim (Bölüm 20.1).

import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";
import Onboarding from "./Onboarding.vue";
import tr from "../locales/tr.json";

const i18n = createI18n({
  legacy: false,
  locale: "tr",
  messages: { tr },
});

function makeWrapper(visible = true) {
  return mount(Onboarding, {
    props: { visible },
    global: { plugins: [i18n] },
  });
}

describe("Onboarding.vue", () => {
  it("visible=false → render etmez", () => {
    const w = makeWrapper(false);
    expect(w.find(".onboard-card").exists()).toBe(false);
  });

  it("visible=true → ilk slide görünür", () => {
    const w = makeWrapper(true);
    expect(w.find(".onboard-card").exists()).toBe(true);
    expect(w.text()).toContain("Görmek");
  });

  it("İleri butonu slide ilerletir", async () => {
    const w = makeWrapper(true);
    const buttons = w.findAll("button");
    const ileri = buttons.find((b) => b.text().includes("İleri"));
    expect(ileri).toBeTruthy();
    await ileri!.trigger("click");
    expect(w.text()).toContain("Anlamak");
  });

  it("Atla → mode seçim ekranına geçer", async () => {
    const w = makeWrapper(true);
    const buttons = w.findAll("button");
    const atla = buttons.find((b) => b.text().includes("Atla"));
    await atla!.trigger("click");
    expect(w.text()).toContain("Hızlı Mod");
    expect(w.text()).toContain("Standart Mod");
  });

  it("mode seçilmeden Başla disabled", async () => {
    const w = makeWrapper(true);
    const atla = w.findAll("button").find((b) => b.text().includes("Atla"));
    await atla!.trigger("click");
    const basla = w.findAll("button").find((b) => b.text().includes("Başla"));
    expect(basla?.attributes("disabled")).toBeDefined();
  });

  it("mode seçildikten sonra finish event'i mode parametresiyle emit eder", async () => {
    const w = makeWrapper(true);
    const atla = w.findAll("button").find((b) => b.text().includes("Atla"));
    await atla!.trigger("click");
    const fastCard = w.find(".mode-card");
    await fastCard.trigger("click");
    const basla = w.findAll("button").find((b) => b.text().includes("Başla"));
    await basla!.trigger("click");
    expect(w.emitted("finish")).toBeTruthy();
    expect(w.emitted("finish")![0]).toEqual(["fast"]);
  });
});
