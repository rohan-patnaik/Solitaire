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
    assert!(metadata.contains("5f40bdcabe87db420f15ab34d71aa26ff5f9e3bb"));
    assert!(metadata.contains("b50faa54c520f49ea27a478786b640b91c8ca9f1"));
    assert!(metadata.contains("fa15999d04876160337bd13c0126b20e78873132"));
    assert!(metadata.contains("2ebbe7edaa0beb04588ead7897e38ecd35a70648"));
    assert!(metadata.contains("9dca631ad3ae5b3f6ca3fb1b35c355a259539c3b"));
    assert!(metadata.contains("d23382b9ec62c7e18dcec9b84f13bb16072338b4"));
    assert!(metadata.contains("0c806cbe8d26ed71bbef888620a5a77cbeaa12e1"));
    assert!(metadata.contains("720fab04ab3528d1e8e66768ebf47a85dc2f94b1"));
    assert!(metadata.contains("478a10a9aed6751c1cd9b90b0122d85faad021dd"));
    assert!(metadata.contains("c0f61e85126072f74a10ffea1fcd831c8b3e3d34"));
    assert!(metadata.contains("d3c384a188bd1281291bc02acf16ebbef8077849"));
    assert!(metadata.contains("4e80b44b4ea3d4820fa5e38f1e8e71aa4e33386a"));
    assert!(metadata.contains("709f93ba55680ecbafb332ee95c345d6aa8ad016"));
    assert!(metadata.contains("actions/runs/33179782126"));
    assert!(metadata.contains("\"scope\": \"application_source\""));
    assert!(generator.contains("data.get(\"current_tip_ci\")"));
    assert!(generated.contains("Pinned application-source CI:"));
    assert!(generated.contains("709f93ba55680ecbafb332ee95c345d6aa8ad016"));
    assert!(generated.contains("actions/runs/33179782126"));
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
fn klondike_new_deal_options_declare_keyboard_accessibility_contracts() {
    let ui = include_str!("../ui/app.slint");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let release = include_str!("../docs/ALPHA_RELEASE.md");

    for contract in [
        "model: [\"Draw 1\", \"Draw 3\"]",
        "model: [\"Standard\", \"Vegas\"]",
        "model: [\"Unlimited\", \"1 redeal\", \"3 redeals\"]",
        "model: [\"Untimed\", \"Timed\"]",
        "accessible-label: \"Draw mode for a new Klondike deal\"",
        "accessible-label: \"Scoring mode for a new Klondike deal\"",
        "accessible-label: \"Stock redeal limit for a new Klondike deal\"",
        "accessible-label: \"Timing mode for a new Klondike deal\"",
        "accessible-label: \"Start a new Klondike deal\"",
        "current-index <=> root.klondike-draw-index",
        "current-index <=> root.klondike-scoring-index",
        "current-index <=> root.klondike-redeal-index",
        "current-index <=> root.klondike-timing-index",
        "text: \"Next Klondike deal\"",
        "enabled: !root.has-pending-new-deal",
        "root.new-game(root.klondike-draw-mode + \" · \" + root.klondike-scoring-mode + \" · \" + root.klondike-redeal-limit + \" · \" + root.klondike-timing-mode)",
    ] {
        assert!(
            ui.contains(contract),
            "missing Klondike option contract: {contract}"
        );
    }
    assert!(catalog.contains("klondike_new_deal_choices_are_saved_and_reopen_with_exact_options"));
    assert!(
        catalog.contains(
            "The Vegas and redeal-limit UI workflows and exact installed final foundation move/process identity remain open"
        )
    );
    assert!(release.contains("or Timed play"));
    let recovery_row = ui
        .find("Discard all unsaved progress and close Solitaire")
        .expect("recovery controls must exist");
    let options_row = ui
        .find("text: \"Next Klondike deal\"")
        .expect("dedicated Klondike options row must exist");
    let status_surface = ui
        .find("status-surface := Rectangle")
        .expect("status surface must exist");
    assert!(recovery_row < options_row && options_row < status_surface);
}

#[test]
fn klondike_vegas_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_C22C173.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "c22c1737d04cfd7cb8fa09b8c65caebec2206eec",
        "bc8b1e621104d23b9fb3fc2a646cfda74882edf5",
        "actions/runs/33112983537",
        "job/98660344058",
        "job/98660344289",
        "solitaire-omarchy 0.1.0.r0.gc22c173-1",
        "enabled:false",
        "active:false",
        "No plugin layer or Solitaire",
        "c29c1cfc0c7e4a2664450524c146e1c346c4c216bd1080bcd6257787326a229b",
        "b5f0a0916ac9a6627ae81eb36b9ce65e926fa70c5e87faf0e28f3e677f5e6eac",
        "used the exact source-built release binary, not the",
        "Exact-package Vegas selection",
    ] {
        assert!(
            evidence.contains(contract),
            "missing c22c173 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_C22C173.md"));
    assert!(
        catalog.contains(
            "The Vegas and redeal-limit UI workflows and exact installed final foundation move/process identity remain open"
        )
    );
}

#[test]
fn freecell_numbered_deal_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_D9D0498.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "d9d0498d8854fb9268b6105c2a767710a63e40e6",
        "c22c1737d04cfd7cb8fa09b8c65caebec2206eec",
        "3637242543b9791bc5e81114dc38ddde11db9e51",
        "actions/runs/33116183706",
        "job/98671301926",
        "job/98671302227",
        "solitaire-omarchy 0.1.0.r0.gd9d0498-1",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or Solitaire",
        "acceptance therefore remain open",
    ] {
        assert!(
            evidence.contains(contract),
            "missing d9d0498 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_D9D0498.md"));
}

#[test]
fn tripeaks_complete_deal_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_8604137.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "860413721e62d8967a938a04562491919b219ab5",
        "d9d0498d8854fb9268b6105c2a767710a63e40e6",
        "36a188bec20487dc914e610eb46f87bb409f25e8",
        "actions/runs/33118432350",
        "job/98678969667",
        "job/98678969895",
        "solitaire-omarchy 0.1.0.r0.g8604137-1",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or Solitaire process was",
        "installed keyboard/AT-SPI final-transition gate remains open",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 8604137 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_8604137.md"));
    assert!(catalog.contains("The exact installed final transition"));
}

#[test]
fn pyramid_complete_deal_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_D1F82F8.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "d1f82f8eb90c29ce25c80963083634dd6e1a105e",
        "860413721e62d8967a938a04562491919b219ab5",
        "6fb553cced72ce63f1cfd4ff3b28e638d88b4d89",
        "actions/runs/33121031503",
        "job/98687625682",
        "job/98687625950",
        "solitaire-omarchy 0.1.0.r0.gd1f82f8-1",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or Solitaire process was",
        "installed keyboard/AT-SPI final-transition gate remains open",
    ] {
        assert!(
            evidence.contains(contract),
            "missing d1f82f8 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_D1F82F8.md"));
    assert!(catalog.contains("pyramid_complete_deal_publication_gate_is_pinned_without_overclaim"));
}

#[test]
fn freecell_complete_deal_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_5F81B4B.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "5f81b4b1a76ddc99636a895ca5900059bd523299",
        "d1f82f8eb90c29ce25c80963083634dd6e1a105e",
        "6315af1ad8762c8861ce5c7915ed11464ef47df3",
        "actions/runs/33123378335",
        "job/98695558469",
        "job/98695558244",
        "solitaire-omarchy 0.1.0.r0.g5f81b4b-1",
        "hash and structurally pin the actual",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or Solitaire process was",
        "installed keyboard/AT-SPI final-transition gate remains open",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 5f81b4b publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_5F81B4B.md"));
    assert!(catalog.contains("0.1.0.r0.g5f81b4b-1"));
}

#[test]
fn klondike_complete_deal_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_5F40BDC.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "5f40bdcabe87db420f15ab34d71aa26ff5f9e3bb",
        "5f81b4b1a76ddc99636a895ca5900059bd523299",
        "ddab109bc00743eeb5c60f4fc4339a39e1f1c452",
        "actions/runs/33125817274",
        "job/98703592996",
        "job/98703593128",
        "solitaire-omarchy 0.1.0.r0.g5f40bdc-1",
        "no actionable findings",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or Solitaire process was",
        "installed keyboard/AT-SPI final-transition gate remains open",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 5f40bdc publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_5F40BDC.md"));
    assert!(catalog.contains("0.1.0.r0.g5f40bdc-1"));
}

#[test]
fn klondike_redeal_limit_candidate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/KLONDIKE_REDEAL_LIMIT_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let controller = include_str!("../src/main.rs");
    let ui = include_str!("../ui/app.slint");
    for contract in [
        "Unlimited`, `1 redeal`, and `3 redeals",
        "persisted replay schema is unchanged",
        "klondike_redeal_limits_are_atomic_reopenable_and_enforced",
        "reopened_klondike_options_map_values_and_indices_without_a_display",
        "No redeals remain",
        "exact in-memory game and on-disk bytes",
        "4 KiB hostile field",
        "57b8dc6e223f0fdf9590ae7893c84253bfa168dc",
        "33128045066",
        "failed run is not accepted as publication evidence",
        "continuously preserve the active desktop",
        "rows Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing Klondike redeal boundary: {contract}"
        );
    }
    assert!(catalog.contains("klondike_redeal_limits_are_atomic_reopenable_and_enforced"));
    assert!(catalog.contains("reopened_klondike_options_map_values_and_indices_without_a_display"));
    assert!(catalog.contains("The Vegas and redeal-limit UI workflows"));
    assert!(controller.contains("Some(\"1 redeal\") => Some(1)"));
    assert!(controller.contains("Some(\"3 redeals\") => Some(3)"));
    assert!(controller.contains("fn klondike_ui_options_for_render"));
    assert!(ui.contains("model: [\"Unlimited\", \"1 redeal\", \"3 redeals\"]"));
    assert!(ui.contains("current-index <=> root.klondike-draw-index"));
    assert!(ui.contains("current-index <=> root.klondike-scoring-index"));
    assert!(ui.contains("current-index <=> root.klondike-redeal-index"));
    assert!(ui.contains("accessible-label: \"Stock redeal limit for a new Klondike deal\""));
    assert!(ui.contains("root.redeals-remaining < 0 ? \" / unlimited\""));
}

#[test]
fn klondike_redeal_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_A72F3CE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "a72f3ce093fbabfd02a64bdc6680b0fff30652c1",
        "57b8dc6e223f0fdf9590ae7893c84253bfa168dc",
        "fc49a07d0795f8ec52c58feb84cb26e55504e675",
        "actions/runs/33128758547",
        "job/98713067551",
        "job/98713067274",
        "solitaire-omarchy 0.1.0.r0.ga72f3ce-1",
        "signed off with no actionable findings",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or native window was",
    ] {
        assert!(
            evidence.contains(contract),
            "missing a72f3ce publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_A72F3CE.md"));
    assert!(catalog.contains("0.1.0.r0.ga72f3ce-1"));
}

#[test]
fn tripeaks_wraparound_candidate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/TRIPEAKS_WRAPAROUND_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let controller = include_str!("../src/main.rs");
    let ui = include_str!("../ui/app.slint");
    for contract in [
        "Standard` and `Ace-King wrap",
        "replay and save schema shapes are",
        "published build through",
        "rejected `wraparound: true`",
        "quarantines that save",
        "preserves the source bytes",
        "Standard saves remain",
        "tripeaks_wraparound_is_strict_atomic_reopenable_and_history_safe",
        "wraparound_tripeaks_checked_save_reopens_equivalent",
        "wraparound_tripeaks_setup_is_accepted_and_preserved",
        "4 KiB hostile rule requests",
        "continuously preserve the user's active application",
        "rows Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing TriPeaks wraparound boundary: {contract}"
        );
    }
    assert!(catalog.contains("tripeaks_wraparound_is_strict_atomic_reopenable_and_history_safe"));
    assert!(
        catalog
            .contains("The exact installed final transition/process identity and rule-selection")
    );
    assert!(catalog.contains("Published builds through a72f3ce reject and quarantine"));
    assert!(controller.contains("(GameKind::TriPeaks, \"Ace-King wrap\")"));
    assert!(controller.contains("fn tripeaks_ui_rule_for_render"));
    assert!(ui.contains("model: [\"Standard\", \"Ace-King wrap\"]"));
    assert!(ui.contains("current-index <=> root.tripeaks-rule-index"));
    assert!(ui.contains("accessible-label: \"Rank rule for a new TriPeaks deal\""));
    assert!(ui.contains("root.tripeaks-wraparound-active"));
}

#[test]
fn tripeaks_wraparound_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_B50FAA5.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "b50faa54c520f49ea27a478786b640b91c8ca9f1",
        "a72f3ce093fbabfd02a64bdc6680b0fff30652c1",
        "9209f7b73fac79529ec680aa1d3abe74fd03dd0a",
        "actions/runs/33130626248",
        "job/98719044292",
        "job/98719044070",
        "solitaire-omarchy 0.1.0.r0.gb50faa5-1",
        "signed off with no actionable findings",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or native window was",
    ] {
        assert!(
            evidence.contains(contract),
            "missing b50faa5 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_B50FAA5.md"));
    assert!(catalog.contains("0.1.0.r0.gb50faa5-1"));
}

#[test]
fn pyramid_redeal_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_FA15999.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "fa15999d04876160337bd13c0126b20e78873132",
        "e1b3b5fbd4ef10ab92b5756522b34717c0d4d3d8",
        "e13313983abac6b03e8972502bbab2ec4241cf80",
        "actions/runs/33133432510",
        "job/98728013394",
        "job/98728013604",
        "solitaire-omarchy 0.1.0.r0.gfa15999-1",
        "signed off this exact tip with no actionable findings",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or native window was",
    ] {
        assert!(
            evidence.contains(contract),
            "missing fa15999 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_FA15999.md"));
    assert!(catalog.contains("0.1.0.r0.gfa15999-1"));
}

#[test]
fn spider_selector_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_2EBBE7E.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "2ebbe7edaa0beb04588ead7897e38ecd35a70648",
        "fa15999d04876160337bd13c0126b20e78873132",
        "a67f4a0a7b0c5d3bc149558b8ea00322c22971f5",
        "actions/runs/33134886017",
        "job/98732525776",
        "job/98732525616",
        "solitaire-omarchy 0.1.0.r0.g2ebbe7e-1",
        "signed off with no actionable findings",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or native window was",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 2ebbe7e publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_2EBBE7E.md"));
    assert!(catalog.contains("0.1.0.r0.g2ebbe7e-1"));
}

#[test]
fn spider_restart_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_9DCA631.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "9dca631ad3ae5b3f6ca3fb1b35c355a259539c3b",
        "b4224998eb94b5b81edf65bfdc9fd29c89becaa5",
        "ba97d08268923f037a1c32e9cc48d81b3a2442e7",
        "actions/runs/33136360301",
        "job/98737098834",
        "job/98737098996",
        "solitaire-omarchy 0.1.0.r0.g9dca631-1",
        "ten-second monotonic deadline",
        "signed off with no",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or native window was",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 9dca631 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_9DCA631.md"));
    assert!(catalog.contains("0.1.0.r0.g9dca631-1"));
}

#[test]
fn klondike_restart_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_D23382B.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "2bf6aa7d4529370051f83552646422a56d020498",
        "d23382b9ec62c7e18dcec9b84f13bb16072338b4",
        "9dca631ad3ae5b3f6ca3fb1b35c355a259539c3b",
        "1ecf716d36752ff96415032d2f08c0747d023fa0",
        "failure paths could leave task-owned restart roots",
        "signed off with no actionable findings",
        "actions/runs/33137835181",
        "job/98741750892",
        "job/98741750767",
        "solitaire-omarchy 0.1.0.r0.gd23382b-1",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or native window was",
        "Klondike final-action and process/window identity gates remain",
    ] {
        assert!(
            evidence.contains(contract),
            "missing d23382b publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_D23382B.md"));
    assert!(catalog.contains("0.1.0.r0.gd23382b-1"));
}

#[test]
fn freecell_restart_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_0C806CB.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "0c806cbe8d26ed71bbef888620a5a77cbeaa12e1",
        "d23382b9ec62c7e18dcec9b84f13bb16072338b4",
        "5d3bc88b77952ab7057e2311dc28434dd2ffe646",
        "full 205-test suite",
        "signed off with no actionable findings",
        "actions/runs/33138972312",
        "job/98745289144",
        "job/98745289065",
        "solitaire-omarchy 0.1.0.r0.g0c806cb-1",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or native window was",
        "FreeCell final-action and process/window identity gates remain",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 0c806cb publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_0C806CB.md"));
    assert!(catalog.contains("0.1.0.r0.g0c806cb-1"));
}

#[test]
fn tripeaks_restart_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_720FAB0.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "720fab04ab3528d1e8e66768ebf47a85dc2f94b1",
        "0c806cbe8d26ed71bbef888620a5a77cbeaa12e1",
        "96322866fe5d8c33126f594f0dd7f9590463adea",
        "signed off with no",
        "actions/runs/33141845041",
        "job/98754184672",
        "job/98754184768",
        "solitaire-omarchy 0.1.0.r0.g720fab0-1",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No plugin layer or native window was",
        "TriPeaks final-action and process/window identity gates remain",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 720fab0 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_720FAB0.md"));
    assert!(catalog.contains("0.1.0.r0.g720fab0-1"));
}

#[test]
fn pyramid_restart_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_478A10A.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "478a10a9aed6751c1cd9b90b0122d85faad021dd",
        "720fab04ab3528d1e8e66768ebf47a85dc2f94b1",
        "a08ac5814c3a352a536883019267af2fff3fc040",
        "returned PASS with no actionable findings",
        "briefly invoked a non-test Solitaire artifact",
        "actions/runs/33144862140",
        "job/98763550798",
        "job/98763550953",
        "solitaire-omarchy 0.1.0.r0.g478a10a-1",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "summoned, focused, cycled, or restarted",
        "Pyramid final-action and process/window identity remain",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 478a10a publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_478A10A.md"));
    assert!(catalog.contains("0.1.0.r0.g478a10a-1"));
}

#[test]
fn restart_current_deal_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_C0F61E8.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "c0f61e85126072f74a10ffea1fcd831c8b3e3d34",
        "478a10a9aed6751c1cd9b90b0122d85faad021dd",
        "a72922d69d82c235cb8fe6712187b66dc49c0e49",
        "returned PASS with zero",
        "The first candidate was not published",
        "actions/runs/33150141503",
        "job/98779976691",
        "job/98779976934",
        "214 tests",
        "solitaire-omarchy 0.1.0.r0.gc0f61e8-1",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No Solitaire process existed before or after",
        "summoned, focused, cycled, or restarted",
        "authoritative `Stuff` mount was unavailable",
        "deferred numbered-deal",
    ] {
        assert!(
            evidence.contains(contract),
            "missing c0f61e8 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_C0F61E8.md"));
    assert!(catalog.contains("0.1.0.r0.gc0f61e8-1"));
}

#[test]
fn timed_klondike_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_D3C384A.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "d3c384a188bd1281291bc02acf16ebbef8077849",
        "c0f61e85126072f74a10ffea1fcd831c8b3e3d34",
        "dcbb1877f18b4f39880afbfb121991c9ef0be96c",
        "returned PASS with zero actionable findings",
        "actions/runs/33155179947",
        "job/98796148931",
        "job/98796149278",
        "all 222",
        "solitaire-omarchy 0.1.0.r0.gd3c384a-1",
        "38dd27d4b5aee221c7adbd40843fb08d13300a2b197667a9cc1e163412b58b3e",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No Solitaire process existed",
        "Nothing was enabled, summoned, focused, cycled, or restarted",
        "authoritative `Stuff` mount was unavailable",
        "deferred numbered-deal work remained untouched",
    ] {
        assert!(
            evidence.contains(contract),
            "missing d3c384a publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_D3C384A.md"));
    assert!(catalog.contains("0.1.0.r0.gd3c384a-1"));
}

#[test]
fn left_handed_klondike_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_4E80B44.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "4e80b44b4ea3d4820fa5e38f1e8e71aa4e33386a",
        "d3c384a188bd1281291bc02acf16ebbef8077849",
        "cb4756907d2791a1975f550c7dcd438aa10e9a6f",
        "returned PASS with zero actionable findings",
        "actions/runs/33163743373",
        "job/98824121006",
        "job/98824120717",
        "all 225",
        "solitaire-omarchy 0.1.0.r0.g4e80b44-1",
        "5467caed810b1e903eed151ec57655f4bc56204cfaf2a40f889da13c8ef34f77",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No Solitaire process existed",
        "Nothing was enabled, summoned, focused, cycled, or restarted",
        "authoritative `Stuff` mount was unavailable",
        "deferred numbered-deal work remained untouched",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 4e80b44 publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_4E80B44.md"));
    assert!(catalog.contains("0.1.0.r0.g4e80b44-1"));
}

#[test]
fn klondike_double_click_publication_gate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PUBLICATION_709F93B.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "7feccc63f8ab05eca1fda948fd72061c9b2fca71",
        "709f93ba55680ecbafb332ee95c345d6aa8ad016",
        "2d92490fa78526a7f8631e6536d83775febb9460",
        "no history was rewritten",
        "returned PASS with zero actionable findings",
        "actions/runs/33179782126",
        "job/98877740992",
        "job/98877741273",
        "232 tests",
        "solitaire-omarchy 0.1.0.r0.g709f93b-1",
        "a80f3c39aa2b4c7ca9915f75291b4e35d78b24eb768e3acb543ee7bcb1a39863",
        "omarchy plugin update io.github.rohan-patnaik.solitaire --yes",
        "enabled:false",
        "active:false",
        "No Solitaire process existed",
        "Nothing was enabled, summoned, focused, cycled, or restarted",
        "authoritative `Stuff` mount was unavailable",
        "deferred numbered-deal work remained untouched",
    ] {
        assert!(
            evidence.contains(contract),
            "missing 709f93b publication boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/PUBLICATION_709F93B.md"));
    assert!(catalog.contains("0.1.0.r0.g709f93b-1"));
}

#[test]
fn spider_suit_selector_candidate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/SPIDER_SUIT_SELECTOR_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let controller = include_str!("../src/main.rs");
    let ui = include_str!("../ui/app.slint");
    for contract in [
        "`1 suit`, `2 suits`, and `4 suits`",
        "No replay or save schema changes",
        "spider_suit_options_are_strict_atomic_reopenable_and_mapped",
        "4 KiB hostile values",
        "mode `0600`",
        "d20ba41",
        "did not verify the corrected reopened ComboBox value/index",
        "no-focus policy",
        "rows therefore remain Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing Spider selector boundary: {contract}"
        );
    }
    assert!(catalog.contains("spider_suit_options_are_strict_atomic_reopenable_and_mapped"));
    assert!(catalog.contains("reopened selector value/index synchronization"));
    assert!(controller.contains("fn spider_ui_mode_for_render"));
    assert!(ui.contains("current-value <=> root.spider-suit-mode"));
    assert!(ui.contains("current-index <=> root.spider-suit-index"));
    assert!(ui.contains("root.spider-suit-mode-active"));
}

#[test]
fn pyramid_redeal_limit_candidate_is_pinned_without_overclaim() {
    let evidence = include_str!("../docs/PYRAMID_REDEAL_LIMIT_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let controller = include_str!("../src/main.rs");
    let ui = include_str!("../ui/app.slint");
    for contract in [
        "No redeals`, `1 redeal`, and `2 redeals",
        "existing serialized `Options::max_redeals`",
        "Published builds through `b50faa5`",
        "preserves its source bytes",
        "pyramid_redeal_limits_are_strict_atomic_reopenable_and_enforced",
        "bounded_pyramid_redeal_setups_are_accepted_and_preserved",
        "reopened_pyramid_options_map_values_and_indices_without_a_display",
        "No Pyramid redeals remain",
        "4 KiB hostile fields",
        "mode `0600`",
        "no-focus policy",
        "rows remain Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing Pyramid redeal boundary: {contract}"
        );
    }
    assert!(catalog.contains("pyramid_redeal_limits_are_strict_atomic_reopenable_and_enforced"));
    assert!(catalog.contains("Published builds through b50faa5 reject and quarantine"));
    assert_eq!(
        catalog
            .matches("docs/PYRAMID_REDEAL_LIMIT_ACCEPTANCE.md")
            .count(),
        3
    );
    assert_eq!(
        catalog
            .matches("pyramid_redeal_limit_candidate_is_pinned_without_overclaim")
            .count(),
        3
    );
    assert!(catalog.contains("the Pyramid redeal selector's value/index behavior"));
    assert!(catalog.contains("TriPeaks and Pyramid selectors, and all five normal complete-deal fixtures are automated candidate"));
    assert!(controller.contains("(GameKind::Pyramid, \"No redeals\")"));
    assert!(controller.contains("fn pyramid_ui_options_for_render"));
    assert!(ui.contains("model: [\"No redeals\", \"1 redeal\", \"2 redeals\"]"));
    assert!(ui.contains("current-index <=> root.pyramid-redeal-index"));
    assert!(ui.contains("accessible-label: \"Stock redeal limit for a new Pyramid deal\""));
    assert!(ui.contains("root.pyramid-max-redeals-active"));
}

#[test]
fn klondike_complete_deal_candidate_is_pinned_without_overclaim() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/klondike-seed-zero-near-win.json");
    let fixture = include_str!("fixtures/klondike-seed-zero-near-win.json");
    let evidence = include_str!("../docs/KLONDIKE_COMPLETE_DEAL_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let controller = include_str!("../src/main.rs");
    let ui = include_str!("../ui/app.slint");

    let envelope: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["game"], "klondike");
    assert_eq!(envelope["payload"]["version"], 2);
    assert_eq!(envelope["payload"]["game"], "klondike");
    assert_eq!(envelope["payload"]["seed"], 0);
    assert_eq!(envelope["payload"]["setup"]["options"]["draw_mode"], "One");
    assert_eq!(
        envelope["payload"]["setup"]["options"]["scoring"],
        "Standard"
    );
    assert!(envelope["payload"]["setup"]["options"]["max_redeals"].is_null());
    assert_eq!(envelope["payload"]["setup"]["options"]["timed"], false);
    assert_eq!(envelope["payload"]["setup"]["elapsed_seconds"], 0);
    assert_eq!(
        envelope["payload"]["actions"].as_array().unwrap().len(),
        155
    );
    assert!(envelope.get("state").is_none());
    assert!(envelope.get("profile").is_none());
    assert!(envelope["payload"].get("state").is_none());
    assert!(envelope["payload"].get("profile").is_none());
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
        Some("64c2c0ac7f7900ae019bb406d5cdb4b33cb1a27f6b6de2e1273373a7840c86ef")
    );
    for contract in [
        "155 actions: 72 draws, three",
        "King of Diamonds in tableau column 1",
        "action 156 and wins at score",
        "controller_completes_legal_klondike_replay_once_and_reopens",
        "klondike_complete_deal_survives_normal_controller_restart",
        "Deal complete — beautifully played",
        "exactly one played and one won",
        "two fresh source processes",
        "ten-second monotonic deadline",
        "8 KiB per stream",
        "complete task-owned root",
        "fails the test visibly",
        "continuously preserves the active desktop",
        "rows Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing Klondike complete-deal boundary: {contract}"
        );
    }
    for test in [
        "legal_seed_zero_replay_reaches_a_one_move_near_win",
        "controller_completes_legal_klondike_replay_once_and_reopens",
        "klondike_complete_deal_survives_normal_controller_restart",
        "klondike_complete_deal_candidate_is_pinned_without_overclaim",
    ] {
        assert!(catalog.contains(test), "missing Klondike evidence: {test}");
    }
    assert!(catalog.contains("exact installed final foundation move/process identity"));
    assert!(controller.contains("Deal complete — beautifully played"));
    assert!(ui.contains("callback tableau-activated(int, int)"));
    assert!(ui.contains("callback foundation-activated(int)"));
    assert!(ui.contains("accessible-action-default => { root.activated(); }"));
    assert!(ui.contains("accessible-live-region: polite"));
}

#[test]
fn freecell_complete_deal_candidate_is_pinned_without_overclaim() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/freecell-seed-zero-near-win.json");
    let fixture = include_str!("fixtures/freecell-seed-zero-near-win.json");
    let evidence = include_str!("../docs/FREECELL_COMPLETE_DEAL_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");

    let envelope: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["game"], "freecell");
    assert_eq!(envelope["payload"]["version"], 2);
    assert_eq!(envelope["payload"]["game"], "freecell");
    assert_eq!(envelope["payload"]["seed"], 0);
    assert!(envelope["payload"]["setup"].is_null());
    assert_eq!(
        envelope["payload"]["actions"].as_array().unwrap().len(),
        105
    );
    assert!(envelope.get("state").is_none());
    assert!(envelope.get("profile").is_none());
    assert!(envelope["payload"].get("state").is_none());
    assert!(envelope["payload"].get("profile").is_none());
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
        Some("3824f1f98ec7e5f0a0c4038198765f02428606f0f1e7133649a146d8d2afdc82")
    );

    for contract in [
        "tests/fixtures/freecell-seed-zero-near-win.json",
        "3824f1f98ec7e5f0a0c4038198765f02428606f0f1e7133649a146d8d2afdc82",
        "105 recorded actions",
        "King of Spades in free cell 2",
        "action 106 and wins",
        "controller_completes_legal_freecell_replay_once_and_reopens",
        "freecell_complete_deal_survives_normal_controller_restart",
        "FreeCell complete — every suit is home",
        "exactly one played and one won",
        "two fresh source processes",
        "ten-second kill-and-reap",
        "deadline, bounded diagnostics",
        "task-root cleanup on success or unwind",
        "continuously preserves the active desktop",
        "rows Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing FreeCell complete-deal boundary: {contract}"
        );
    }
    assert!(catalog.contains("tests/fixtures/freecell-seed-zero-near-win.json"));
    assert!(catalog.contains("docs/FREECELL_COMPLETE_DEAL_ACCEPTANCE.md"));
    assert!(catalog.contains("freecell_complete_deal_survives_normal_controller_restart"));
    assert!(catalog.contains("exact installed final foundation move/process identity"));
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
        "Exact-package reopened selector value/index synchronization, final transition/process identity, and drag/touch remain open."
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
        "starts a fresh normal `Controller`",
        "starts another fresh `Controller`",
        "not an installed process/window identity check",
        "parent-created temporary root and nonce marker",
        "ten-second deadline",
        "kills and reaps a stalled child",
        "bounded diagnostics",
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
        "spider_complete_deal_survives_normal_controller_restart",
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
        "Exact-package reopened selector value/index synchronization, final transition/process identity, and drag/touch remain open."
    ));
    assert!(controller.contains("Spider complete — all eight runs are home"));
    assert!(ui.contains("callback spider-tableau-activated(int, int)"));
    assert!(ui.contains("accessible-action-default => { root.activated(); }"));
    assert!(ui.contains("accessible-live-region: polite"));
}

#[test]
fn tripeaks_complete_deal_candidate_is_pinned_without_overclaim() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tripeaks-seed-zero-near-win.json");
    let fixture = include_str!("fixtures/tripeaks-seed-zero-near-win.json");
    let evidence = include_str!("../docs/TRIPEAKS_COMPLETE_DEAL_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let controller = include_str!("../src/main.rs");
    let ui = include_str!("../ui/app.slint");

    let envelope: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["game"], "tripeaks");
    assert_eq!(envelope["payload"]["version"], 2);
    assert_eq!(envelope["payload"]["game"], "tripeaks");
    assert_eq!(envelope["payload"]["seed"], 0);
    assert_eq!(envelope["payload"]["setup"]["wraparound"], false);
    assert_eq!(envelope["payload"]["actions"].as_array().unwrap().len(), 48);
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
        Some("c7063d5b9a9c99c1c2034c7f17a8c4a5322807b9242de3f5bdb73d2e832e3f9c")
    );
    for contract in [
        "one keyboard-routable move",
        "Production `Game::from_replay` reconstructs 52 conserved cards",
        "TriPeaks complete — all three peaks are clear",
        "controller_completes_legal_tripeaks_replay_once_and_reopens",
        "tripeaks_complete_deal_survives_normal_controller_restart",
        "exactly one played and one won observation",
        "two fresh source processes",
        "ten-second kill-and-",
        "reap deadline, bounded diagnostics",
        "task-root cleanup on success or unwind",
        "Remaining installed gate",
        "position 1, the sole remaining top card",
        "engine/callback index `0`",
        "accessibility, real-platform, and profile rows Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing TriPeaks complete-deal boundary: {contract}"
        );
    }
    for test in [
        "legal_seed_zero_replay_reaches_a_one_move_near_win",
        "controller_completes_legal_tripeaks_replay_once_and_reopens",
        "tripeaks_complete_deal_survives_normal_controller_restart",
        "tripeaks_complete_deal_candidate_is_pinned_without_overclaim",
    ] {
        assert!(catalog.contains(test), "missing TriPeaks evidence: {test}");
    }
    assert!(catalog.contains(
        "{\"id\":\"game.tripeaks\",\"title\":\"Playable TriPeaks\",\"status\":\"partial\""
    ));
    assert!(catalog.contains("exact installed final transition/process identity"));
    assert!(controller.contains("TriPeaks complete — all three peaks are clear"));
    assert!(ui.contains("callback tripeaks-tableau-activated(int)"));
    assert!(ui.contains("accessible-action-default => { root.activated(); }"));
    assert!(ui.contains("accessible-live-region: polite"));
}

fn assert_pyramid_restart_source_contract(controller: &str) {
    assert!(controller.contains("fn exercise_pyramid_restart_child"));
    assert!(controller.contains("pyramid_complete_deal_survives_normal_controller_restart"));
    assert!(controller.contains("Pyramid complete — every tableau card is clear"));
}

#[test]
fn pyramid_complete_deal_candidate_is_pinned_without_overclaim() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pyramid-seed-zero-near-win.json");
    let fixture = include_str!("fixtures/pyramid-seed-zero-near-win.json");
    let evidence = include_str!("../docs/PYRAMID_COMPLETE_DEAL_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let controller = include_str!("../src/main.rs");
    let ui = include_str!("../ui/app.slint");

    let envelope: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["game"], "pyramid");
    assert_eq!(envelope["payload"]["version"], 2);
    assert_eq!(envelope["payload"]["game"], "pyramid");
    assert_eq!(envelope["payload"]["seed"], 0);
    assert_eq!(envelope["payload"]["setup"]["max_redeals"], 2);
    let actions = envelope["payload"]["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 62);
    assert_eq!(
        actions
            .iter()
            .filter(|action| action.as_str() == Some("Draw"))
            .count(),
        38
    );
    assert_eq!(
        actions
            .iter()
            .filter(|action| action.as_str() == Some("Recycle"))
            .count(),
        2
    );
    assert_eq!(
        actions
            .iter()
            .filter(|action| action.get("RemovePair").is_some())
            .count(),
        18
    );
    assert_eq!(
        actions
            .iter()
            .filter(|action| action.get("RemoveKing").is_some())
            .count(),
        4
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
        Some("c2ed2aba92d2af5ea4beff5a15d94c0892a6bf54b3b05f3eec7b6cb1a49777aa")
    );
    for contract in [
        "pair before victory",
        "Production `Game::from_replay` accepts every action",
        "represented cards account for the original 52-card deck",
        "Pyramid complete — every tableau card is clear",
        "controller_completes_legal_pyramid_replay_once_and_reopens",
        "pyramid_complete_deal_survives_normal_controller_restart",
        "two fresh source processes",
        "shared ten-second",
        "hostile ambient-environment guard",
        "display-independent source-process lifecycle evidence",
        "exactly one played and one won observation",
        "Pyramid tableau position 1",
        "internal callback index is `0`",
        "real-platform, and profile rows Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing Pyramid complete-deal boundary: {contract}"
        );
    }
    for test in [
        "legal_seed_zero_replay_reaches_a_one_pair_near_win",
        "controller_completes_legal_pyramid_replay_once_and_reopens",
        "pyramid_complete_deal_survives_normal_controller_restart",
        "pyramid_complete_deal_candidate_is_pinned_without_overclaim",
    ] {
        assert!(catalog.contains(test), "missing Pyramid evidence: {test}");
    }
    assert!(catalog.contains(
        "{\"id\":\"game.pyramid\",\"title\":\"Playable Pyramid\",\"status\":\"partial\""
    ));
    assert!(
        catalog.contains(
            "Exact-package redeal selection/display and final transition/process identity"
        )
    );
    assert_pyramid_restart_source_contract(controller);
    assert!(ui.contains("callback pyramid-tableau-activated(int)"));
    assert!(ui.contains("callback pyramid-waste-activated"));
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
    assert!(ui.contains("current-value <=> root.spider-suit-mode"));
    assert!(ui.contains("current-index <=> root.spider-suit-index"));
    assert!(ui.contains("Start the next Spider deal with the selected suit count"));
    assert!(ui.contains("Open free cell"));
}

#[test]
fn freecell_numbered_deal_entry_declares_keyboard_and_accessibility_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    let catalog = include_str!("../docs/offline-capabilities.json");
    let evidence = include_str!("../docs/FREECELL_NUMBERED_DEAL_ACCEPTANCE.md");
    for contract in [
        "callback new-freecell-deal(string)",
        "text: \"Choose FreeCell deal\"",
        "placeholder-text: \"Decimal deal number\"",
        "input-type: InputType.number",
        "accessible-label: \"FreeCell deal number\"",
        "accepted(value) => { root.new-freecell-deal(value); }",
        "accessible-label: \"Open the entered FreeCell deal number\"",
        "accessible-label: \"Start the next numbered FreeCell deal\"",
        "enabled: !root.has-pending-new-deal",
    ] {
        assert!(
            ui.contains(contract),
            "missing numbered-deal UI contract: {contract}"
        );
    }
    for contract in [
        "fn parse_freecell_deal_number",
        "MAX_U64_DECIMAL_DIGITS",
        "state.new_freecell_game(deal_number.as_str())",
        "FreeCellDeal::Exact(deal_number)",
        "app.set_deal_number(freecell_deal_number(state.deal_number))",
    ] {
        assert!(
            controller.contains(contract),
            "missing numbered-deal controller contract: {contract}"
        );
    }
    assert!(catalog.contains(
        "exact_freecell_deal_is_strict_atomic_reopenable_and_does_not_consume_next_deal"
    ));
    assert!(ui.contains("Deal  \" + root.deal-number"));
    for contract in [
        "c90dd474c0d3cbbabe03fe105031d1713e316041f2e58db37bc191cb7d682e8b",
        "9d358022af547259a531ba5083b4fe1e1fd8300abfbec1c46f4512ce882ac9b4",
        "beac885a1d0fca2d911e2411f1e2450da5a4d39fa649a36aac8bc7366ce2f2fb",
        "did not expose the virtual-keyboard protocol",
        "skipped rather than sending input to the user's active desktop",
        "Remaining exact-package gate",
    ] {
        assert!(
            evidence.contains(contract),
            "missing numbered-deal acceptance boundary: {contract}"
        );
    }
    let recovery_row = ui
        .find("Discard all unsaved progress and close Solitaire")
        .expect("recovery controls must exist");
    let numbered_deal_row = ui
        .find("text: \"Choose FreeCell deal\"")
        .expect("dedicated FreeCell deal row must exist");
    let status_surface = ui
        .find("status-surface := Rectangle")
        .expect("status surface must exist");
    assert!(recovery_row < numbered_deal_row && numbered_deal_row < status_surface);
}

#[test]
fn tripeaks_surface_declares_keyboard_and_accessibility_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    for contract in [
        "model: [\"Klondike\", \"Spider\", \"FreeCell\", \"Pyramid\", \"TriPeaks\"]",
        "callback tripeaks-draw-stock",
        "callback tripeaks-tableau-activated",
        "Start the next TriPeaks deal with the selected rank rule",
        "Rank rule for a new TriPeaks deal",
        "model: [\"Standard\", \"Ace-King wrap\"]",
        "current-index <=> root.tripeaks-rule-index",
        "Standard TriPeaks uses no rank wraparound",
        "TriPeaks Ace-King wrap treats Ace and King as adjacent",
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
        "Start the next Pyramid deal with the selected redeal limit",
        "Stock redeal limit for a new Pyramid deal",
        "model: [\"No redeals\", \"1 redeal\", \"2 redeals\"]",
        "current-index <=> root.pyramid-redeal-index",
        "Pyramid uses pair-to-13 rules and ",
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
fn restart_current_deal_declares_keyboard_accessibility_and_recovery_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    let evidence = include_str!("../docs/RESTART_CURRENT_DEAL_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "callback restart-deal-requested",
        "text: \"Restart deal\"",
        "Restart the current deal with the same deal number and rules",
        "root.restart-deal-requested()",
        "root.pending-deal-is-restart ? \"Discard and restart\" : \"Discard and start\"",
        "Cancel the pending restart and preserve the current game",
    ] {
        assert!(
            ui.contains(contract),
            "missing restart UI contract: {contract}"
        );
    }
    for contract in [
        "fn restart_current_deal(&mut self)",
        "restart_seed: Some(self.game.state.seed)",
        "timed: self.game.state.options.timed",
        "restart_seed: Some(self.spider.state.seed)",
        "restart_seed: Some(self.freecell.state.deal_number)",
        "restart_seed: Some(self.tripeaks.state.seed)",
        "restart_seed: Some(self.pyramid.state.seed)",
        "if let Some(seed) = request.restart_seed",
        "Restart cancelled; current game preserved",
    ] {
        assert!(
            controller.contains(contract),
            "missing restart controller contract: {contract}"
        );
    }
    for contract in [
        "repository-defined seed or deal number",
        "never reserves, increments, lowers, or rewrites a next-deal sequence",
        "Restart is a deal boundary rather than a replay action",
        "stale-writer conflict",
        "mode-0600",
        "exact packaged control has not been invoked",
        "rows remain Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing restart evidence boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/RESTART_CURRENT_DEAL_ACCEPTANCE.md"));
    assert!(catalog.contains("exact-package input remains open"));
}

#[test]
fn klondike_timed_play_declares_keyboard_accessibility_and_checkpoint_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    let domain = include_str!("../src/klondike.rs");
    let evidence = include_str!("../docs/KLONDIKE_TIMED_PLAY_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "model: [\"Untimed\", \"Timed\"]",
        "accessible-label: \"Timing mode for a new Klondike deal\"",
        "current-index <=> root.klondike-timing-index",
        "root.klondike-timed-active ? \"     Time  \" + root.elapsed-time",
        "elapsed time \" + root.elapsed-time",
        "root.klondike-timing-mode",
    ] {
        assert!(
            ui.contains(contract),
            "missing timed UI contract: {contract}"
        );
    }
    for contract in [
        "const KLONDIKE_TIMER_CHECKPOINT_SECONDS: u64 = 15",
        "fn klondike_timer_running(&self) -> bool",
        "fn advance_klondike_timer(&mut self, seconds: u64) -> bool",
        "fn checkpoint_klondike_elapsed(&mut self) -> bool",
        "let last_tick = Cell::new(Instant::now())",
        "now.saturating_duration_since(previous).as_secs()",
        "let _ = self.checkpoint_klondike_elapsed()",
    ] {
        assert!(
            controller.contains(contract),
            "missing timed controller contract: {contract}"
        );
    }
    assert!(domain.contains("if self.state.options.timed"));
    for contract in [
        "keyboard-focusable",
        "Legacy three-field new-deal requests remain untimed",
        "Undo and redo change cards and moves but never rewind",
        "before leaving Klondike or closing",
        "since the last checkpoint",
        "never write deal counters or the local profile",
        "exact packaged selector and clock have not been exercised",
        "Partial.",
    ] {
        assert!(
            evidence.contains(contract),
            "missing timed evidence boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/KLONDIKE_TIMED_PLAY_ACCEPTANCE.md"));
    assert!(catalog.contains("timed_klondike_checkpoint_failure_is_recoverable_and_fail_closed"));
}

#[test]
fn klondike_left_handed_layout_declares_keyboard_accessibility_and_scope() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    let evidence = include_str!("../docs/KLONDIKE_LEFT_HANDED_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "klondike-layout-mode: \"Right-handed\"",
        "model: [\"Right-handed\", \"Left-handed\"]",
        "current-value <=> root.klondike-layout-mode",
        "current-index <=> root.klondike-layout-index",
        "accessible-label: \"Klondike table layout for this session\"",
        "pure callback klondike-top-x(int, float, bool) -> float",
        "root.klondike-top-x(0, parent.width / 1px",
        "root.klondike-top-x(1, parent.width / 1px",
        "root.klondike-top-x(index + 2, parent.width / 1px",
        "activated => { root.draw-stock(); }",
        "activated => { root.waste-activated(); }",
        "activated => { root.foundation-activated(index); }",
    ] {
        assert!(
            ui.contains(contract),
            "missing handed UI contract: {contract}"
        );
    }
    for contract in [
        "app.on_klondike_top_x(klondike_top_x)",
        "fn klondike_top_x(slot: i32, available_width: f32, left_handed: bool)",
        "if !available_width.is_finite()",
        "maximum - right_handed",
        "klondike_handed_layout_is_an_exact_bounded_mirror",
    ] {
        assert!(
            controller.contains(contract),
            "missing handed geometry contract: {contract}"
        );
    }
    assert!(!controller.contains("set_klondike_layout_mode"));
    for contract in [
        "session-scoped",
        "Right-handed remains the startup",
        "Pile identities, suit indices, action routes, accessible names",
        "seven tableau columns are intentionally unchanged",
        "returns to Right-handed after process exit",
        "fixed work and allocates nothing",
        "No save, profile, counter, replay, fixture schema",
        "remain Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing handed evidence boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/KLONDIKE_LEFT_HANDED_ACCEPTANCE.md"));
    assert!(catalog.contains("owner-approved persisted settings model remain open"));
}

#[test]
fn klondike_double_click_declares_pointer_keyboard_and_recovery_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    let evidence = include_str!("../docs/KLONDIKE_DOUBLE_CLICK_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "pointer-double-activation: false",
        "in property <string> pointer-context",
        "in property <string> pointer-generation",
        "callback pointer-pressed(string, string, string)",
        "callback pointer-activated(string, string, string)",
        "callback double-activated(string, string, string)",
        "event.button == PointerEventButton.left",
        "event.kind == PointerEventKind.down",
        "root.pointer-pressed(root.card.label, root.pointer-context, root.pointer-generation)",
        "root.pointer-activated(root.card.label, root.pointer-context, root.pointer-generation)",
        "root.double-activated(root.card.label, root.pointer-context, root.pointer-generation)",
        "accessible-action-default => { root.activated(); }",
        "if (!event.repeat) { root.activated(); }",
        "pointer-double-activation: true",
        "pointer-context: root.klondike-deal-instance",
        "pointer-generation: root.interaction-generation",
        "pointer-double-activation: card.face-up && card-index == column.cards.length - 1",
        "root.waste-pointer-pressed(card, context, generation)",
        "root.tableau-pointer-pressed(column-index, card-index, card, context, generation)",
        "root.waste-pointer-activated(card, context, generation)",
        "root.waste-double-activated(card, context, generation)",
        "root.tableau-pointer-activated(column-index, card-index, card, context, generation)",
        "root.tableau-double-activated(column-index, card-index, card, context, generation)",
    ] {
        assert!(
            ui.contains(contract),
            "missing double-click UI contract: {contract}"
        );
    }
    assert!(!ui.contains("single-click := Timer"));
    for contract in [
        "const POINTER_DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(500)",
        "struct PointerClickTimer",
        "struct KlondikePointerIdentity",
        "double_armed",
        "TimerMode::SingleShot",
        "app.on_waste_pointer_pressed",
        "app.on_tableau_pointer_pressed",
        "fn pointer_pressed",
        "fn pointer_clicked",
        "fn take_double",
        "fn close_requested",
        "fn advance_interaction_generation",
        "self.interaction_generation = self.interaction_generation.wrapping_add(1)",
        "set_interaction_generation",
        "fn activate_tableau_pointer",
        "fn double_activate_tableau",
        "fn klondike_tableau_top",
        "fn activate_waste_pointer",
        "fn double_activate_waste",
        "fn klondike_waste_top",
        "klondike_deal_instance.wrapping_add(1)",
        "set_klondike_deal_instance",
        "That Klondike card is no longer available; click again",
        "pointer_click_timer_is_idle_single_shot_and_cancelable",
        "deferred_pointer_click_cannot_overtake_keyboard_or_stock_input",
        "double_click_requires_matching_first_click_identity",
        "blocked_close_invalidates_a_pending_pointer_click",
        "klondike_double_activation_is_exact_atomic_and_undoable",
        "app.on_waste_pointer_activated",
        "app.on_waste_double_activated",
        "app.on_tableau_pointer_activated",
        "app.on_tableau_double_activated",
    ] {
        assert!(
            controller.contains(contract),
            "missing double-click controller contract: {contract}"
        );
    }
    for contract in [
        "single pointer click",
        "double-click",
        "Keyboard Enter/Space and the accessibility default action remain immediate",
        "4 KiB tokens",
        "Klondike deal-instance token",
        "interaction generation",
        "zero idle callbacks",
        "intervening immediate card activation and stock mutation",
        "direct double callback without a first click",
        "first click's complete source/card/deal/generation",
        "blocked or confirmed close requests",
        "dirty/profile-dirty close guard",
        "actionable retry status",
        "Drag/drop remains explicitly deferred",
        "remain Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing double-click evidence boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/KLONDIKE_DOUBLE_CLICK_ACCEPTANCE.md"));
    assert!(catalog.contains("exact-package pointer timing, touch, AT-SPI"));
}

#[test]
fn klondike_safe_finish_declares_atomic_keyboard_and_recovery_contracts() {
    let ui = include_str!("../ui/app.slint");
    let controller = include_str!("../src/main.rs");
    let domain = include_str!("../src/klondike.rs");
    let evidence = include_str!("../docs/KLONDIKE_SAFE_FINISH_ACCEPTANCE.md");
    let catalog = include_str!("../docs/offline-capabilities.json");
    for contract in [
        "text: \"Finish safe moves\"",
        "accessible-label: \"Move every currently safe Klondike card to a foundation\"",
        "clicked => { root.autocomplete-requested(); }",
    ] {
        assert!(
            ui.contains(contract),
            "missing safe-finish UI contract: {contract}"
        );
    }
    for contract in [
        "fn autocomplete(&mut self)",
        "self.clear_selections();",
        "let count = self.game.autocomplete();",
        "Moved 1 safe card to a foundation",
        "Moved {count} safe cards to foundations",
        "klondike_safe_finish_is_atomic_reopenable_and_history_safe",
        "klondike_safe_finish_conflict_preserves_both_owners_until_reload",
        "app.on_autocomplete_requested",
    ] {
        assert!(
            controller.contains(contract),
            "missing safe-finish controller contract: {contract}"
        );
    }
    for contract in [
        "while let Some(action) = self.foundation_hint()",
        "if self.apply(action).is_err()",
        "completed += 1",
    ] {
        assert!(
            domain.contains(contract),
            "missing safe-finish domain contract: {contract}"
        );
    }
    for contract in [
        "keyboard-focusable",
        "bounded by the 52-card deck",
        "write-free no-op",
        "checked mode-0600 save",
        "remain per card;",
        "stale-writer test",
        "variable-sized payload.",
        "No production GUI or live Omarchy shell is invoked",
        "remain Partial",
    ] {
        assert!(
            evidence.contains(contract),
            "missing safe-finish evidence boundary: {contract}"
        );
    }
    assert!(catalog.contains("docs/KLONDIKE_SAFE_FINISH_ACCEPTANCE.md"));
    assert!(catalog.contains("klondike_safe_finish_is_atomic_reopenable_and_history_safe"));
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
    assert!(ui.contains("Discard current unsaved progress and restart this deal"));
    assert!(ui.contains("Cancel the pending new deal and preserve the current game"));
    assert!(ui.contains("Cancel the pending restart and preserve the current game"));
    assert!(ui.contains("Refresh ownership only if a locked recheck confirms the save is missing"));
    assert!(ui.contains("Discard all unsaved progress and close Solitaire"));
    assert!(ui.contains("Discard in-memory changes and reload the newer disk copy"));
    assert!(ui.contains("Reload the newer disk copy and resolve pending deal-change ownership"));
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
        "Pyramid uses pair-to-13 rules and ",
        "root.pyramid-max-redeals-active == 1 ? \" stock redeal. \" : \" stock redeals. \"",
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
    assert_eq!(ui.matches("vertical-stretch: 0").count(), 5);

    let recovery_action = ui
        .find("Discard all unsaved progress and close Solitaire")
        .expect("recovery action must remain available");
    let status_surface = ui
        .find("status-surface := Rectangle")
        .expect("status surface must exist");
    assert!(recovery_action < status_surface);
}
