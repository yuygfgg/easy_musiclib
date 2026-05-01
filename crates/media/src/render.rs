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

pub const FLAC_HLS_PLAYLIST_FILE: &str = "playlist.m3u8";
pub const FLAC_HLS_INIT_FILE: &str = "init.mp4";
pub const FLAC_HLS_SEGMENT_PATTERN: &str = "segment_%05d.m4s";
pub const FLAC_HLS_PLAYLIST_MIME: &str = "application/vnd.apple.mpegurl";
pub const FLAC_HLS_MEDIA_MIME: &str = "audio/mp4";
pub const FLAC_HLS_SEGMENT_SECONDS: f64 = 2.0;

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
