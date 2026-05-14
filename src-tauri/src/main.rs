// SPDX-License-Identifier: GPL-3.0-or-later
// D-Space — Windows için akıllı disk analiz ve geri kazanım platformu
// Copyright (C) 2026 D-Space contributors

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    dspace_lib::run();
}
