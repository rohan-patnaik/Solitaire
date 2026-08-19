#[test]
fn plugin_uses_detached_process_lifecycle() {
    let plugin = include_str!("../Plugin.qml");
    assert!(plugin.contains("launcher.startDetached()"));
    assert!(!plugin.contains("launcher.running = true"));
    assert!(plugin.contains("native binary not found on PATH"));
}

#[test]
fn arch_recipe_is_immutable_and_checksum_pinned() {
    let recipe = include_str!("../packaging/arch/PKGBUILD");
    assert!(recipe.contains("_commit=49fda3a72376e3925fb092b65351a7b157dcb2e6"));
    assert!(recipe.contains("078d71dff0246f6b72107e303e668bc5ccd0b38171d8d04938ea2a0eeb2e38b0"));
    assert!(!recipe.contains("refs/tags"));
    assert!(!recipe.contains("'SKIP'"));
}

#[test]
fn card_keyboard_controls_ignore_repeat_and_show_focus() {
    let ui = include_str!("../ui/app.slint");
    assert_eq!(
        ui.matches("if (!event.repeat) { root.activated(); }")
            .count(),
        2
    );
    assert!(ui.contains("card-focus.has-focus ? #ffffff"));
    assert!(ui.contains("slot-focus.has-focus ? #ffffff"));
}
