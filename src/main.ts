// SPDX-License-Identifier: GPL-3.0-or-later
import { createApp } from "vue";
import App from "./App.vue";
import { i18n } from "./i18n";

createApp(App).use(i18n).mount("#app");
