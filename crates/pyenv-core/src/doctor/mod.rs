// ./crates/pyenv-core/src/doctor/mod.rs
//! Health and diagnostics reporting for common pyenv-native configuration issues.

mod checks;
mod fixes;
mod helpers;
mod report;
mod tests;
mod types;

pub use checks::{collect_checks, collect_checks_with_options};
pub use fixes::{
    apply_doctor_fixes, apply_doctor_fixes_with_options, doctor_fix_plan,
    doctor_fix_plan_with_options, install_powershell_7,
};
pub use report::cmd_doctor;
pub use types::{DoctorCheck, DoctorFix, DoctorFixOutcome, DoctorOptions, DoctorStatus};
