fn main() {
    eprintln!("DEP_TAURI_DEV = {:?}", std::env::var("DEP_TAURI_DEV"));
    tauri_build::build()
}
