//! The steps that drive the stage runner (`dev-tools`) in scratch repositories with stand-ins for the task
//! runner, the machine lock and the test runner.

pub mod budgets;
pub mod checkout;
pub mod frozen;
pub mod journal;
pub mod locks;
pub mod named;
pub mod needs;
pub mod scan_range;
pub mod scratch;
