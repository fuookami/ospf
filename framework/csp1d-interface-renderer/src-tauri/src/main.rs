// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows", allow(static_mut_refs))]

fn main() {
    csp1d_interface_renderer_lib::run()
}
