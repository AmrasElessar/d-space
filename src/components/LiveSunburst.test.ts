// SPDX-License-Identifier: GPL-3.0-or-later
//
// LiveSunburst component testleri — Sprint 3.7 (Bölüm 9.6.5 + 15.4).
// Pure SVG arc render + canlı partial_tree akışı.

import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import LiveSunburst from "./LiveSunburst.vue";

interface PartialNode {
  id: number;
  parent: number | null;
  name: string;
  aggregate_size: number;
  depth: number;
  is_dir: boolean;
}

function root(size: number): PartialNode {
  return {
    id: 5,
    parent: null,
    name: "<volume root>",
    aggregate_size: size,
    depth: 0,
    is_dir: true,
  };
}

function child(
  id: number,
  parent: number,
  name: string,
  size: number,
  depth = 1,
  isDir = true,
): PartialNode {
  return {
    id,
    parent,
    name,
    aggregate_size: size,
    depth,
    is_dir: isDir,
  };
}

describe("LiveSunburst.vue", () => {
  it("boş partial_tree iken placeholder mesaj gösterir", () => {
    const w = mount(LiveSunburst, {
      props: { partialTree: [], emptyMessage: "İlk veriler bekleniyor…" },
    });
    expect(w.find("svg").exists()).toBe(false);
    expect(w.text()).toContain("İlk veriler bekleniyor…");
  });

  it("root aggregate=0 iken yine placeholder gösterir", () => {
    const w = mount(LiveSunburst, {
      props: {
        partialTree: [root(0)],
        emptyMessage: "İlk veriler bekleniyor…",
      },
    });
    expect(w.find("svg").exists()).toBe(false);
    expect(w.text()).toContain("İlk veriler bekleniyor…");
  });

  it("depth=1 düğüm sayısı kadar wedge path üretir", () => {
    const tree: PartialNode[] = [
      root(1000),
      child(100, 5, "docs", 600),
      child(101, 5, "bin", 300),
      child(102, 5, "tmp", 100),
    ];
    const w = mount(LiveSunburst, {
      props: { partialTree: tree, emptyMessage: "..." },
    });
    expect(w.find("svg").exists()).toBe(true);
    // 3 depth-1 wedge → 3 .wedge path; ek olarak depth-2 yok.
    const wedgePaths = w.findAll("path.wedge");
    expect(wedgePaths.length).toBe(3);
  });

  it("en büyük wedge en geniş açıyı kaplar (path 'A' yarıçaplı daha uzun yay)", () => {
    // big=900, small=100 — big wedge'in açı oranı 0.9, small 0.1.
    // Sınama: big wedge'in path string'inde largeArc=1 (>π) olmamalı
    // (90% × 2π = ~5.65 rad → 1.8π > π → largeArc=1).
    // Daha sağlam: ilk wedge (en büyük) DOM sırasında ilk gelmeli ve
    // title attribute'unda en büyük boyut görünmeli.
    const tree: PartialNode[] = [
      root(1000),
      child(100, 5, "big", 900),
      child(101, 5, "small", 100),
    ];
    const w = mount(LiveSunburst, {
      props: { partialTree: tree, emptyMessage: "..." },
    });
    const titles = w.findAll("path.wedge title");
    expect(titles.length).toBe(2);
    // İlk wedge "big" olmalı çünkü size DESC sıralı.
    expect(titles[0].text()).toContain("big");
    expect(titles[1].text()).toContain("small");
    // big wedge largeArc=1 olmalı (>180°).
    const bigPath = w.findAll("path.wedge")[0].attributes("d") ?? "";
    // arcPath: " A R R 0 largeArc 1 ..." formatında largeArc okunur.
    // 900/1000 * 2π = 5.65 rad > π → largeArc=1.
    expect(bigPath).toMatch(/A [\d.]+ [\d.]+ 0 1 /);
    const smallPath = w.findAll("path.wedge")[1].attributes("d") ?? "";
    // 100/1000 * 2π = 0.628 rad < π → largeArc=0.
    expect(smallPath).toMatch(/A [\d.]+ [\d.]+ 0 0 /);
  });

  it("depth=2 düğümler parent'ın yay aralığına yerleştirilir", () => {
    const tree: PartialNode[] = [
      root(1000),
      child(100, 5, "docs", 800),
      child(200, 100, "a", 500, 2),
      child(201, 100, "b", 300, 2),
    ];
    const w = mount(LiveSunburst, {
      props: { partialTree: tree, emptyMessage: "..." },
    });
    // Toplam wedge sayısı: 1 (docs depth1) + 2 (a, b depth2) = 3
    const wedges = w.findAll("path.wedge");
    expect(wedges.length).toBe(3);
    // wedge-d1 ve wedge-d2 sınıfları doğru ayrılmalı.
    expect(w.findAll("path.wedge-d1").length).toBe(1);
    expect(w.findAll("path.wedge-d2").length).toBe(2);
  });
});
