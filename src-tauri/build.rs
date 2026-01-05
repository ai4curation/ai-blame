fn main() {
    // Tauri expects `icons/icon.png` to exist at compile-time (via `generate_context!()`).
    // Keep the repo binary-free by generating a tiny placeholder icon when missing.
    ensure_placeholder_icon();

    tauri_build::build()
}

fn ensure_placeholder_icon() {
    use std::fs;
    use std::path::PathBuf;

    let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("icons")
        .join("icon.png");

    if icon_path.exists() {
        return;
    }

    if let Some(parent) = icon_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Minimal 1x1 transparent PNG (68 bytes) used as a placeholder icon.
    // This is a temporary icon created during the build process.
    // For production, replace with proper icons - see src-tauri/icons/README.md for icon setup.
    // Base64 source: iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMBAFhPqV8AAAAASUVORK5CYII=
    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 252, 255, 31, 0, 3, 3,
        1, 0, 88, 79, 169, 95, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    let _ = fs::write(&icon_path, PNG_1X1);
}
