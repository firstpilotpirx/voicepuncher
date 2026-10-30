//! What both recognizers of the command spotting service share: its address and its program.

use voicepuncher_copy_pool::{ServiceAddress, COMMAND_SPOTTING_COPY_OFFSET};

use super::{Lookup, Missing, PathBuf, Service, ServiceKind, COMMAND_SPOTTING};

impl Lookup<'_> {
    /// The command spotting service at `url`, missing the first of its program and `models`
    /// not found; none when `url` names no address.
    pub(super) fn command_spotting_service(
        &self,
        url: &str,
        copies: u32,
        models: &[&Result<PathBuf, Missing>],
    ) -> Option<(ServiceAddress, Service)> {
        let address = ServiceAddress::parse(url).ok()?;
        let address = address.with_copy_offset(COMMAND_SPOTTING_COPY_OFFSET);
        let program = self.command_spotting_program();
        let problem = std::iter::once(&program)
            .chain(models.iter().copied())
            .find_map(|found| found.as_ref().err().cloned());
        let kind = ServiceKind::CommandSpotting;
        let service = Service { kind, binary: COMMAND_SPOTTING, program, problem, copies };
        Some((address, service))
    }
}
