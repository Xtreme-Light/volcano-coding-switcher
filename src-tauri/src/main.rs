//! 二进制入口，把所有逻辑委托给库 crate。

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    volcano_coding_switcher_lib::run();
}
