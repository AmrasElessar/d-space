// SPDX-License-Identifier: GPL-3.0-or-later
//
// Treemap layout — squarified algoritma kenar durumları (Bölüm 20.1).

import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import Treemap from "./Treemap.vue";

function mockNode(id: number, name: string, aggregate: number, isDir = false) {
  return {
    id,
    parent: null,
    name,
    data_size: aggregate,
    aggregate_size: aggregate,
    is_dir: isDir,
    score: null,
    score_rule: null,
    score_reason: null,
    modified_unix: 0,
  };
}

describe("Treemap.vue", () => {
  it("data null iken empty state gösterir", () => {
    const w = mount(Treemap, { props: { data: null } });
    expect(w.text()).toContain("Treemap için yeterli veri yok");
  });

  it("parent_aggregate_size = 0 iken empty state", () => {
    const w = mount(Treemap, {
      props: {
        data: {
          parent_id: 1,
          parent_name: "root",
          parent_aggregate_size: 0,
          total_children: 1,
          returned: 1,
          nodes: [mockNode(2, "small", 0)],
        },
      },
    });
    expect(w.text()).toContain("Treemap için yeterli veri yok");
  });

  it("aggregate > 0 olan tek node SVG cell üretir", () => {
    const w = mount(Treemap, {
      props: {
        data: {
          parent_id: 1,
          parent_name: "root",
          parent_aggregate_size: 1_000_000,
          total_children: 1,
          returned: 1,
          nodes: [mockNode(2, "big.bin", 1_000_000)],
        },
      },
    });
    expect(w.find("svg").exists()).toBe(true);
    expect(w.findAll("rect").length).toBeGreaterThan(0);
  });

  it("klasör tıklaması drill emit eder, dosya etmez", async () => {
    const w = mount(Treemap, {
      props: {
        data: {
          parent_id: 1,
          parent_name: "root",
          parent_aggregate_size: 200,
          total_children: 2,
          returned: 2,
          nodes: [
            mockNode(2, "subdir", 150, true),
            mockNode(3, "file.txt", 50, false),
          ],
        },
      },
    });
    const groups = w.findAll("g.cell");
    expect(groups.length).toBe(2);
    // İlk cell (en büyük = subdir, klasör)
    await groups[0].trigger("click");
    expect(w.emitted("drill")).toBeTruthy();
    const initialEmits = w.emitted("drill")?.length ?? 0;
    // Dosya tıklaması drill etmez
    await groups[1].trigger("click");
    const afterFileEmits = w.emitted("drill")?.length ?? 0;
    expect(afterFileEmits).toBe(initialEmits);
  });
});
