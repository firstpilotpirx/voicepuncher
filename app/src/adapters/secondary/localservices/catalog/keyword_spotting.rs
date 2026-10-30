//! The command spotting service with the keyword spotter: its model folder and voice gate,
//! confirming with the speech recognition service for commands.

use super::{
    command_spotting_args, shown, Lookup, Missing, PathBuf, ServiceProbe, ServiceSpec,
    KWS_MODEL_DIR_PREFIX, KWS_VAD_MODEL,
};

impl Lookup<'_> {
    pub(super) fn command_spotting(
        &self,
        url: &str,
        commands_url: &str,
        copies: u32,
        probe: &dyn ServiceProbe,
    ) -> Vec<ServiceSpec> {
        let model_dir = self.kws_model_dir();
        let vad = self.model("VOICEPUNCHER_KWS_VAD_MODEL", KWS_VAD_MODEL);
        let Some((address, service)) =
            self.command_spotting_service(url, copies, &[&model_dir, &vad])
        else {
            return Vec::new();
        };
        self.copies(&service, &address, probe, |port, _| {
            let history = self.command_history.as_ref();
            command_spotting_args(
                port,
                &shown(&model_dir),
                &shown(&vad),
                commands_url,
                history,
            )
        })
    }

    /// The newest `sherpa-onnx-kws-*` folder, as the `justfile` picks it.
    fn kws_model_dir(&self) -> Result<PathBuf, Missing> {
        self.newest_dir("VOICEPUNCHER_KWS_MODEL_DIR", KWS_MODEL_DIR_PREFIX)
    }
}
