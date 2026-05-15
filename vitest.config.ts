// SPDX-License-Identifier: GPL-3.0-or-later
//
// Vitest config — Bölüm 20.1 (Test Piramidi, TypeScript unit).

import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "happy-dom",
    include: ["src/**/*.{test,spec}.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
    },
  },
});
