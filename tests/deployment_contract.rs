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
    assert!(recipe.contains("_commit=dc193f4cea8510ccb8bc803ebd457ecd28d9d8bc"));
    assert!(recipe.contains(
        "_release_sha256=6db5400d5d384302d43bb218618468233ab27f850e76580f21fb46d25fac43bf"
    ));
    assert!(recipe.contains("pkgrel=1"));
    assert!(recipe.contains("SOLITAIRE_RELEASE_SHA256"));
    assert!(recipe.contains("^[0-9a-f]{64}$"));
    assert!(recipe.contains("${1,,}"));
    let checksum_tests = include_str!("../packaging/arch/test-checksums.sh");
    for invalid in ["''", "SKIP", "abc", "abcdef0", "z123"] {
        assert!(checksum_tests.contains(invalid));
    }
    let release = include_str!("../packaging/arch/release.json");
    assert!(release.contains("\"source_revision\": \"dc193f4cea8510ccb8bc803ebd457ecd28d9d8bc\""));
    assert!(release.contains(
        "\"source_sha256\": \"6db5400d5d384302d43bb218618468233ab27f850e76580f21fb46d25fac43bf\""
    ));
    assert!(release.contains("actions/runs/32551758881"));
    assert!(release.contains("verified_application_revision"));
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
fn capability_catalog_distinguishes_baseline_from_pinned_application_source() {
    let metadata = include_str!("../docs/offline-capabilities.json");
    let generated = include_str!("../docs/OFFLINE_CAPABILITIES.md");
    let generator = include_str!("../scripts/generate_offline_capabilities.py");
    assert!(metadata.contains("f6b0cb7e55d296bdf77714efc48a1775b858c041"));
    assert!(metadata.contains("d20ba4111deb2e948e593fbeec4ca2c45b597bef"));
    assert!(metadata.contains("4b31024426b73fafe93597e4cd42312eef2b26b0"));
    assert!(metadata.contains("actions/runs/32645102863"));
    assert!(metadata.contains("\"scope\": \"application_source\""));
    assert!(generator.contains("data.get(\"current_tip_ci\")"));
    assert!(generated.contains("Pinned application-source CI:"));
    assert!(generated.contains("4b31024426b73fafe93597e4cd42312eef2b26b0"));
    assert!(generated.contains("actions/runs/32645102863"));
    assert!(generated.contains("(success)."));
    assert!(generated.contains("| Complete | 1 |"));
    assert!(generated.contains("| Partial | 10 |"));
}

#[test]
fn alpha_release_evidence_keeps_acceptance_gaps_explicit() {
    let evidence = include_str!("../docs/ALPHA_RELEASE.md");
    for contract in [
        "f6b0cb7e55d296bdf77714efc48a1775b858c041",
        "all five games",
        "omarchy-shell shell summon",
        "omarchy restart shell",
        "Solitaire is not installed",
        "Solitaire could not start",
        "Orca was not installed",
        "No five complete wins were played through the installed UI",
        "Achievements are deferred",
        "one applicable capability row is Complete",
    ] {
        assert!(
            evidence.contains(contract),
            "missing acceptance boundary: {contract}"
        );
    }
}

#[test]
fn spider_variant_acceptance_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/OMARCHY_WAYLAND_ACCEPTANCE_D20BA41.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "d20ba4111deb2e948e593fbeec4ca2c45b597bef",
        "d01c6f635bfb73e0e47838ac3f0287c6889c1069",
        "actions/runs/32638817768",
        "solitaire-omarchy 0.1.0.r0.gd20ba41-1",
        "13c046bd855a7d03ef3652361a59983aa98d20ad0306a3d24181ee79219361c6",
        "a9978c21b0c303ecdf741455066617b6def6b467a328dde27b9b07600668cd05",
        "9 total files, 0 altered files",
        "3a964a66fdc02dbb8b91dcf22b6b4467f1f10fe7354cc3f772892768c290c986",
        "c23d413fd52598590c9809c2844ee447679dbc396feef9c9f60a9b5d22babe08",
        "one, two, and four suits",
        "50 cards\\nDeal row",
        "50/500/0",
        "40/499/1",
        "visible fourth column",
        "Move { from: 3, to: 4, count: 1 }",
        "Only a descending same-suit run can move together",
        "Face-down card",
        "Page Down reached its final",
        "Status details. Use arrow keys to scroll status messages",
        "No complete Spider deal was won",
        "Broader hostile/property coverage and drag/touch behavior remain pending",
        "Full keyboard-only traversal and Orca spoken-output acceptance remain",
        "Live missing-binary and immediate-startup-failure launcher notifications",
        "Reproducibility, signed checksums, SBOM",
    ] {
        assert!(
            evidence.contains(contract),
            "missing focused acceptance boundary: {contract}"
        );
    }
    assert!(
        catalog.contains(
            "{\"id\":\"game.spider\",\"title\":\"Playable Spider\",\"status\":\"partial\""
        )
    );
}

#[test]
fn spider_hostile_property_coverage_is_catalogued_without_overclaim() {
    let catalog = include_str!("../docs/offline-capabilities.json");
    let roadmap = include_str!("../ROADMAP.md");
    for test in [
        "hostile_actions_are_exact_and_fully_atomic",
        "fixed_seed_mode_action_space_preserves_spider_invariants",
        "synthetic_final_run_wins_but_is_not_a_full_deal_replay",
    ] {
        assert!(catalog.contains(test), "missing Spider coverage: {test}");
    }
    assert!(
        catalog.contains(
            "{\"id\":\"game.spider\",\"title\":\"Playable Spider\",\"status\":\"partial\""
        )
    );
    assert!(catalog.contains(
        "The exact installed final-transition and normal Controller startup/reopen gate and drag/touch behavior remain open."
    ));
    assert!(roadmap.contains(
        "Spider's dependency-free hostile-action and fixed seed/mode action-space tests"
    ));
}

fn assert_spider_complete_deal_acceptance_contract(evidence: &str) {
    for contract in [
        "e45da16bbd19e0c04f7a76696d309eac7681f4db",
        "caf35da8ed4a60d55f87ad2967e80a89f804b993b40619fd85a3b0af341f394e",
        "4,340 bytes",
        "113 moves and",
        "Move { from: 0, to: 2, count: 10 }",
        "score 1,082, move 118",
        "1,181, move 119",
        "Score  1082     Moves  118     Runs  7/8",
        "Score  1181     Moves  119     Runs  8/8",
        "Pointer clicks and AT-SPI default-action invocation are not substitutes for this keyboard-only gate.",
        "Column 0 is exactly Ten, Nine, Eight, Seven, Six",
        "Five, Four, Three, Two, Ace; column 2 is exactly King, Queen, Jack.",
        "production persistence loaders",
        "does not exercise installed `Controller`",
        "`HEAD == origin/main ==` the recorded full",
        "terminal success is recorded",
        "exact-head",
        "Rust and exact-revision Arch package CI jobs",
        "package version, source archive SHA-256, package archive",
        "SHA-256, full installed source marker and its SHA-256",
        "and installed binary",
        "exact candidate package on the normal host with an isolated",
        "`XDG_DATA_HOME`",
        "pacman -Q solitaire-omarchy",
        "pacman -Qkk solitaire-omarchy",
        "9 total files, 0 altered files",
        "Require exactly one Solitaire process and one native Wayland window",
        "client size 1180x820 and `xwayland=false`",
        "Resolve `/proc/$PID/exe` to",
        "`/usr/bin/solitaire` and require its SHA-256 to equal the recorded installed",
        "binary hash.",
        "original-resolution 1180x820 screenshots",
        "transition, with hashes",
        "new normal `Controller` startup",
        "normal user Solitaire data",
        "require no Solitaire process or window",
        "restore the original",
        "AT-SPI enabled state",
        "checksum the complete evidence bundle",
        "not a record of a passed package run",
        "seed `local-profile.json`",
        "0 played · 0 won",
        "1 played · 1 won",
        "Spider complete — all eight runs are home",
        "Game status: Spider complete — all eight runs are home",
        "bytes and SHA-256 must not change",
        "drag/drop, touch input",
    ] {
        assert!(
            evidence.contains(contract),
            "missing complete-deal boundary: {contract}"
        );
    }
}

#[test]
fn spider_complete_deal_candidate_is_pinned_without_overclaim() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/spider-one-suit-near-win.json");
    let fixture = include_str!("fixtures/spider-one-suit-near-win.json");
    let evidence = include_str!("../docs/SPIDER_COMPLETE_DEAL_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let controller = include_str!("../src/main.rs");
    let ui = include_str!("../ui/app.slint");

    let envelope: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["game"], "spider");
    assert_eq!(envelope["payload"]["version"], 2);
    assert_eq!(envelope["payload"]["game"], "spider");
    assert_eq!(envelope["payload"]["seed"], 3);
    assert_eq!(envelope["payload"]["setup"], "One");
    let actions = envelope["payload"]["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 118);
    assert_eq!(
        actions.iter().filter(|action| action.is_string()).count(),
        5
    );
    assert!(envelope["payload"].get("state").is_none());
    assert!(envelope.get("profile").is_none());
    let checksum = std::process::Command::new("sha256sum")
        .arg(&fixture_path)
        .output()
        .unwrap();
    assert!(checksum.status.success());
    assert_eq!(
        String::from_utf8(checksum.stdout)
            .unwrap()
            .split_whitespace()
            .next(),
        Some("caf35da8ed4a60d55f87ad2967e80a89f804b993b40619fd85a3b0af341f394e")
    );
    assert_spider_complete_deal_acceptance_contract(evidence);

    for test in [
        "legal_one_suit_replay_reaches_a_one_move_near_win",
        "controller_completes_legal_spider_replay_once_and_reopens",
        "spider_complete_deal_candidate_is_pinned_without_overclaim",
    ] {
        assert!(catalog.contains(test), "missing Spider evidence: {test}");
    }
    assert!(
        catalog.contains(
            "{\"id\":\"game.spider\",\"title\":\"Playable Spider\",\"status\":\"partial\""
        )
    );
    assert!(catalog.contains(
        "The exact installed final-transition and normal Controller startup/reopen gate and drag/touch behavior remain open."
    ));
    assert!(controller.contains("Spider complete — all eight runs are home"));
    assert!(ui.contains("callback spider-tableau-activated(int, int)"));
    assert!(ui.contains("accessible-action-default => { root.activated(); }"));
    assert!(ui.contains("accessible-live-region: polite"));
}

#[test]
fn launcher_failure_acceptance_is_pinned_and_scoped() {
    let evidence = include_str!("../docs/OMARCHY_LAUNCHER_ACCEPTANCE_4B31024.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "4b31024426b73fafe93597e4cd42312eef2b26b0",
        "54d9c6ee27c9e0839e21579aac8a918549a4dbc1",
        "actions/runs/32645102863",
        "7f3fc9770c4af2d44024a5a67151a1df8ba4414dd80bcf05e737ba72808e2914",
        "fa61e0f95cc992dd783bfa701bb8787b3442d215d407132c0749e5abf3850c1e",
        "solitaire-omarchy 0.1.0.r0.g4b31024-1",
        "b7b2d8f7befe8ca2be10bbf92d784bde0ade18aa4bf3367fb6c0cfc0cb52bfd1",
        "9 total files, 0 altered files",
        "f6b0cb7e55d296bdf77714efc48a1775b858c041",
        "omarchy-shell shell summon io.github.rohan-patnaik.solitaire '{}'",
        "Solitaire is not installed",
        "Install the native solitaire binary and try again.",
        "launcher exited 127",
        "Solitaire could not start",
        "Solitaire failed during startup (exit 42). Run solitaire in a terminal for details.",
        "final status 42",
        "Bubblewrap mounted the",
        "without editing `Plugin.qml` or any host file",
        "verified all 29 listed",
        "makes only `foundation.omarchy-launcher` Complete",
        "full keyboard traversal, Orca output, drag/touch",
    ] {
        assert!(
            evidence.contains(contract),
            "missing launcher acceptance boundary: {contract}"
        );
    }
    assert!(catalog.contains(
        "{\"id\":\"foundation.omarchy-launcher\",\"title\":\"Detached Omarchy launcher\",\"status\":\"complete\""
    ));
    assert!(catalog.contains(
        "{\"id\":\"quality.real-omarchy\",\"title\":\"Real Omarchy and Wayland acceptance\",\"status\":\"partial\""
    ));
    assert!(!catalog.contains("live missing-binary/startup-failure notifications remain pending"));
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
fn tripeaks_surface_declares_keyboard_and_accessibility_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    for contract in [
        "model: [\"Klondike\", \"Spider\", \"FreeCell\", \"Pyramid\", \"TriPeaks\"]",
        "callback tripeaks-draw-stock",
        "callback tripeaks-tableau-activated",
        "Start the next standard TriPeaks deal",
        "Standard TriPeaks uses no rank wraparound",
        "accessible-action-default => { root.activated(); }",
        "accessible-live-region: polite",
    ] {
        assert!(
            ui.contains(contract),
            "missing TriPeaks UI contract: {contract}"
        );
    }
    assert!(controller.contains("tableau position {position}, exposed"));
    assert!(controller.contains("Tableau position {position}, covered, face-down"));
    assert!(controller.contains("Waste card, {}; activate to draw the next stock card"));
    assert!(ui.contains("Deal  \" + root.deal-number"));
}

#[test]
fn pyramid_surface_declares_keyboard_and_accessibility_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    for contract in [
        "model: [\"Klondike\", \"Spider\", \"FreeCell\", \"Pyramid\", \"TriPeaks\"]",
        "callback pyramid-draw-stock",
        "callback pyramid-waste-activated",
        "callback pyramid-tableau-activated",
        "Start the next standard Pyramid deal",
        "Standard Pyramid uses pair-to-13 rules and two redeals",
        "accessible-action-default => { root.activated(); }",
        "accessible-live-region: polite",
    ] {
        assert!(
            ui.contains(contract),
            "missing Pyramid UI contract: {contract}"
        );
    }
    assert!(controller.contains("Pyramid tableau position {position}, exposed"));
    assert!(controller.contains("Pyramid tableau position {position}, covered, face-down"));
    assert!(controller.contains("Pyramid waste, {}{}; activate to select or remove"));
    assert!(ui.contains("Deal  \" + root.deal-number"));
}

#[test]
fn local_statistics_surface_declares_scope_and_dirty_state_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    let profile = include_str!("../src/profile.rs");
    for contract in [
        "Local: 0 played · 0 won",
        "Device-local statistics for the active game",
        "controller.local_profile_dirty",
        "Retry before closing",
        "deals_played",
        "deals_won",
    ] {
        assert!(
            ui.contains(contract) || controller.contains(contract) || profile.contains(contract),
            "missing local-statistics contract: {contract}"
        );
    }
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

#[test]
fn long_recovery_status_has_a_dedicated_scrollable_surface() {
    let ui = include_str!("../ui/app.slint");
    for contract in [
        "width: 1180px",
        "height: 820px",
        "spacing: 12px",
        "vertical-stretch: 0",
        "min-height: 44px",
        "preferred-height: 44px",
        "max-height: 44px",
        "min-height: 40px",
        "preferred-height: 40px",
        "max-height: 40px",
        "status-surface := Rectangle",
        "min-height: 80px",
        "preferred-height: 80px",
        "max-height: 80px",
        "Status details — arrow keys scroll",
        "Standard TriPeaks uses no rank wraparound. All play stays on this device.",
        "Standard Pyramid uses pair-to-13 rules and two redeals. All play stays on this device.",
        "status-scroll := ScrollView",
        "width: parent.width - 28px",
        "viewport-width: self.visible-width",
        "viewport-height: max(self.visible-height, status-text.preferred-height + 4px)",
        "vertical-scrollbar-policy: always-on",
        "horizontal-scrollbar-policy: always-off",
        "event.text == Key.PageDown",
        "accessible-live-region: polite",
        "accessible-label: \"Game status: \" + root.status-text",
        "accessible-role: groupbox",
        "accessible-label: \"Status details. Use arrow keys to scroll status messages\"",
        "wrap: char-wrap",
    ] {
        assert!(
            ui.contains(contract),
            "missing status-surface contract: {contract}"
        );
    }
    assert_eq!(ui.matches("vertical-stretch: 0").count(), 3);

    let recovery_action = ui
        .find("Discard all unsaved progress and close Solitaire")
        .expect("recovery action must remain available");
    let status_surface = ui
        .find("status-surface := Rectangle")
        .expect("status surface must exist");
    assert!(recovery_action < status_surface);
}
