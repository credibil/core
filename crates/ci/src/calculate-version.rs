extern crate cargo_semver_checks;

use cargo_semver_checks::{ActualSemverUpdate, Check, GlobalConfig, ReleaseType, Rustdoc};

fn main() {
    println!("Hello, world!");

    // let current = Rustdoc::from_root("test_crates/trait_missing/old/");
    let current = Rustdoc::from_git_revision(".", "v0.1.0");
    let baseline = Rustdoc::from_root(".");
    let mut config = GlobalConfig::new();
    let mut check = Check::new(current);
    let check = check.set_baseline(baseline);
    let report = check.check_release(&mut config).unwrap();
    assert!(!report.success());

    let (_crate_name, crate_report) = report.crate_reports().iter().next().unwrap();
    let required_bump = crate_report.required_bump().unwrap();
    assert_eq!(required_bump, ReleaseType::Major);

    // The "old" and "new" crates have the same version.
    // The detected bump is the minimum-possible SemVer bump in a new release.
    // Since the crates are v0.1.0, the minimum possible bump is minor.
    assert_eq!(crate_report.detected_bump(), ActualSemverUpdate::Minor);
}
