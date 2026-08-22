#[test]
fn plugin_uses_detached_process_lifecycle() {
    let plugin = include_str!("../Plugin.qml");
    assert!(plugin.contains("launcher.startDetached()"));
    assert!(!plugin.contains("launcher.running = true"));
    assert!(plugin.contains("native binary not found on PATH"));
    assert!(plugin.contains("failed during startup"));
    assert!(plugin.contains("exit $status"));
}

#[test]
fn arch_recipe_packages_the_expected_revision() {
    let recipe = include_str!("../packaging/arch/PKGBUILD");
    assert!(recipe.contains("SOLITAIRE_SOURCE_ARCHIVE"));
    assert!(recipe.contains("SOLITAIRE_EXPECTED_REVISION"));
    assert!(recipe.contains("SOLITAIRE_SOURCE_SHA256"));
    assert!(!recipe.contains("sha256sums=('SKIP')"));
    assert!(recipe.contains("Solitaire/archive/${_commit}.tar.gz"));
    assert!(recipe.contains(".solitaire-source-revision"));
    assert!(recipe.contains("cargo test --locked --all-targets --all-features"));
    assert!(recipe.contains("$pkgdir/usr/share/$pkgname/source-revision"));
    assert!(!recipe.contains("_commit=49fda3a"));
}

#[test]
fn arch_ci_verifies_the_installed_revision() {
    let workflow = include_str!("../.github/workflows/ci.yml");
    assert!(workflow.contains("git archive \"$GITHUB_SHA\""));
    assert!(workflow.contains("SOLITAIRE_EXPECTED_REVISION=\"$GITHUB_SHA\""));
    assert!(workflow.contains("/usr/share/solitaire-omarchy/source-revision"));
    assert!(workflow.contains("pacman -Q solitaire-omarchy"));
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

#[test]
fn spider_and_freecell_surfaces_are_accessible_and_interactive() {
    let ui = include_str!("../ui/app.slint");
    for contract in [
        "callback spider-deal-stock",
        "callback spider-tableau-activated",
        "callback freecell-cascade-activated",
        "callback freecell-cell-activated",
        "callback freecell-foundation-activated",
        "accessible-label: \"Game picker\"",
        "accessible-live-region: polite",
    ] {
        assert!(ui.contains(contract), "missing UI contract: {contract}");
    }
    assert!(ui.contains("model: [\"1 suit\", \"2 suits\", \"4 suits\"]"));
    assert!(ui.contains("Open free cell"));
}

#[test]
fn unsaved_changes_have_retry_and_close_warning_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    assert!(ui.contains("callback retry-save-requested"));
    assert!(ui.contains("callback discard-progress-and-start-requested"));
    assert!(ui.contains("callback cancel-new-deal-requested"));
    assert!(ui.contains("callback discard-and-close-requested"));
    assert!(ui.contains("Retry saving changes kept in memory"));
    assert!(ui.contains("Discard current unsaved progress and start the pending new deal"));
    assert!(ui.contains("Cancel the pending new deal and preserve the current game"));
    assert!(ui.contains("Discard all unsaved progress and close Solitaire"));
    assert!(ui.contains("Discard in-memory changes and reload the newer disk copy"));
    assert!(controller.contains("CloseRequestResponse::KeepWindowShown"));
    assert!(controller.contains("Unsaved changes remain"));
    assert!(controller.contains("commit_pending_new_deal"));
}
