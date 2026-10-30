//! The command spotting service with the streaming recognizers: its two model folders and its
//! voice gate, no speech recognition service for commands.

use super::{shown, streaming_args, Lookup, ServiceProbe, ServiceSpec, KWS_VAD_MODEL};

const STREAMING_EN_PREFIX: &str = "sherpa-onnx-nemotron-speech-streaming-en-";
const STREAMING_RU_PREFIX: &str = "sherpa-onnx-streaming-zipformer-small-ru-";

impl Lookup<'_> {
    pub(super) fn streaming(
        &self,
        url: &str,
        copies: u32,
        probe: &dyn ServiceProbe,
    ) -> Vec<ServiceSpec> {
        let en = self.newest_dir("VOICEPUNCHER_STREAMING_EN_MODEL_DIR", STREAMING_EN_PREFIX);
        let ru = self.newest_dir("VOICEPUNCHER_STREAMING_RU_MODEL_DIR", STREAMING_RU_PREFIX);
        let vad = self.model("VOICEPUNCHER_KWS_VAD_MODEL", KWS_VAD_MODEL);
        let Some((address, service)) =
            self.command_spotting_service(url, copies, &[&en, &ru, &vad])
        else {
            return Vec::new();
        };
        self.copies(&service, &address, probe, |port, _| {
            let models = [shown(&en), shown(&ru), shown(&vad)];
            streaming_args(port, &models, self.command_history.as_ref())
        })
    }
}
