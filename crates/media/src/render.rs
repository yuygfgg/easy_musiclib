use crate::formats::CueRenderQuality;

#[derive(Debug, Clone)]
pub struct RenderTags {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub track_no: Option<i64>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackTranscodeFormat {
    Opus256k,
    Flac48k,
}

impl PlaybackTranscodeFormat {
    pub fn mime(self) -> &'static str {
        match self {
            Self::Opus256k => "audio/ogg; codecs=opus",
            Self::Flac48k => "audio/flac",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Opus256k => "opus",
            Self::Flac48k => "flac",
        }
    }
}

#[derive(Debug)]
pub struct TranscodedAudio {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub extension: &'static str,
    pub quality: CueRenderQuality,
}
