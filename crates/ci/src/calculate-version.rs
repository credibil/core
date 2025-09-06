//! Calculate Semver Bump
//!
//! This utility calculates the next semantic version bump required based on
//! API changes.

use anyhow::{Result, anyhow};
use cargo_semver_checks::{Check, GlobalConfig, Rustdoc};

// N.B. The detected bump is the minimum-possible SemVer bump in a new release.
// For crates at v0.1.0, the minimum possible bump is minor.
fn main() -> Result<()> {
    let baseline = Rustdoc::from_git_revision(".", "v0.1.0");
    let current = Rustdoc::from_root(".");

    let mut config = GlobalConfig::new();
    let mut check = Check::new(current);
    let check = check.set_baseline(baseline);

    let report = check.check_release(&mut config)?;

    if report.success() {
        println!("No semver bump required");
        return Ok(());
    }

    let Some((_crate_name, crate_report)) = report.crate_reports().iter().next() else {
        return Err(anyhow!("Crate reports not found"));
    };

    let Some(required_bump) = crate_report.required_bump() else {
        return Err(anyhow!("Required bump not found"));
    };

    let level = format!("{required_bump:?}").to_lowercase();
    println!("level={level}");

    Ok(())
}
