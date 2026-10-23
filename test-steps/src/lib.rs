//! What the test harnesses of the workspace share: the needs a test states ([`required_env`]) and the Gherkin
//! steps both the slice and component harness and the E2E harness use.

pub mod needs;
pub mod shared_steps;

pub use needs::required_env;
pub use shared_steps::SharedSteps;

#[cfg(test)]
mod test_checks;
