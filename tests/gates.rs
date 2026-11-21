//! M0.4: repo gates are configured with the measured MSRV and run deny.

const MSRV: &str = "1.85.1"; // measured with `cargo msrv find`

#[test]
fn cargo_toml_declares_measured_msrv() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains(&format!("rust-version = \"{MSRV}\"")),
        "rust-version must be {MSRV}"
    );
}

#[test]
fn check_script_runs_cargo_deny() {
    assert!(include_str!("../scripts/check.sh").contains("cargo deny check"));
}

// M8.1 (P12): the lockfile stays buildable on the MSRV, and CI checks the dev-dependencies too.

#[test]
fn cargo_config_prefers_msrv_compatible_versions() {
    let cfg = include_str!("../.cargo/config.toml");
    let resolver = cfg
        .split("[resolver]")
        .nth(1)
        .expect("a [resolver] table in .cargo/config.toml");
    assert!(
        resolver.contains("incompatible-rust-versions = \"fallback\""),
        "cargo update must pick versions that build on {MSRV}"
    );
}

#[test]
fn ci_msrv_job_builds_and_compiles_tests_locked() {
    let ci = include_str!("../.github/workflows/ci.yml");
    let job = ci
        .split("\n  msrv:")
        .nth(1)
        .and_then(|rest| rest.split("\n  deny:").next())
        .expect("an msrv job followed by the deny job");
    assert!(job.contains("cargo build --locked"), "{job}");
    assert!(job.contains("cargo test --locked --no-run"), "{job}");
}
