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
    assert!(recipe.contains("_commit=df9f4f3cf4b49482f031ea5b890a117a31b93408"));
    assert!(!recipe.contains("deec66f09fa9d3afda9831f6cf258da3d660b873"));
    assert!(recipe.contains("SOLITAIRE_RELEASE_SHA256"));
    assert!(recipe.contains("^[0-9a-f]{64}$"));
    assert!(recipe.contains("${1,,}"));
    let checksum_tests = include_str!("../packaging/arch/test-checksums.sh");
    for invalid in ["''", "SKIP", "abc", "abcdef0", "z123"] {
        assert!(checksum_tests.contains(invalid));
    }
    let release = include_str!("../packaging/arch/release.json");
    assert!(release.contains("\"source_sha256\": null"));
    assert!(release.contains("blocked_pending_verified_archive_checksum"));
    assert!(release.contains("actions/runs/32498442489"));
}

#[test]
fn arch_ci_verifies_the_installed_revision() {
    let workflow = include_str!("../.github/workflows/ci.yml");
    assert!(workflow.contains("git archive \"$GITHUB_SHA\""));
    assert!(workflow.contains("SOLITAIRE_EXPECTED_REVISION=\"$GITHUB_SHA\""));
    assert!(workflow.contains("/usr/share/solitaire-omarchy/source-revision"));
    assert!(workflow.contains("pacman -Q solitaire-omarchy"));
    assert!(workflow.contains("bash packaging/arch/test-checksums.sh"));
    assert!(workflow.contains("test \"${#release_fields[@]}\" -eq 5"));
    assert!(workflow.contains("expected_pkgver"));
    assert!(workflow.contains("expected_pkgrel=1"));
    assert!(!workflow.contains("0.1.0.r0.g$short_sha-2"));
}

#[test]
fn capability_catalog_distinguishes_baseline_from_unpublished_current_tip() {
    let metadata = include_str!("../docs/offline-capabilities.json");
    let generated = include_str!("../docs/OFFLINE_CAPABILITIES.md");
    let generator = include_str!("../scripts/generate_offline_capabilities.py");
    assert!(metadata.contains("df9f4f3cf4b49482f031ea5b890a117a31b93408"));
    assert!(metadata.contains("actions/runs/32498442489"));
    assert!(metadata.contains("\"current_tip_ci\": null"));
    assert!(generator.contains("data.get(\"current_tip_ci\")"));
    assert!(generated.contains("Current remediation exact-tip CI:"));
    assert!(generated.contains("Current remediation exact-tip CI: not yet recorded."));
    assert!(generated.contains("| Complete | 0 |"));
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
    assert!(ui.contains("callback confirm-missing-save-requested"));
    assert!(ui.contains("callback discard-and-close-requested"));
    assert!(ui.contains("has-any-unsaved-changes"));
    assert!(ui.contains("has-pending-save-conflict"));
    assert!(ui.contains("Retry saving changes kept in memory"));
    assert!(ui.contains("Discard current unsaved progress and start the pending new deal"));
    assert!(ui.contains("Cancel the pending new deal and preserve the current game"));
    assert!(ui.contains("Refresh ownership only if a locked recheck confirms the save is missing"));
    assert!(ui.contains("Discard all unsaved progress and close Solitaire"));
    assert!(ui.contains("Discard in-memory changes and reload the newer disk copy"));
    assert!(ui.contains("Reload the newer disk copy and resolve pending new deal ownership"));
    assert!(controller.contains("CloseRequestResponse::KeepWindowShown"));
    assert!(controller.contains("Unsaved changes remain"));
    assert!(controller.contains("commit_pending_new_deal"));
    assert!(controller.contains("on-disk entry now contains the current game"));
    assert!(controller.contains("the original path is gone"));
}
