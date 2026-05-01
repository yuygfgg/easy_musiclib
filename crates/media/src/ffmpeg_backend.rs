use crate::formats::{CueRenderQuality, CueTrackRenderer, FFMPEG_CUE_RENDERER};
use crate::render::{PlaybackTranscodeFormat, RenderTags, TranscodedAudio};
use anyhow::{Context, Result, anyhow};
use ffmpeg::{codec, filter, format, frame, media};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::Rescale;
use once_cell::sync::OnceCell;
use std::fs;
#[cfg(unix)]
use std::os::fd::{FromRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::Path;

pub(crate) static CUE_RENDERER: FfmpegCueRenderer = FfmpegCueRenderer;

static FFMPEG_INIT: OnceCell<()> = OnceCell::new();

pub(crate) struct FfmpegCueRenderer;

impl CueTrackRenderer for FfmpegCueRenderer {
    fn id(&self) -> &'static str {
        FFMPEG_CUE_RENDERER
    }

    fn priority(&self, format_id: &str) -> Option<i32> {
        (!format_id.eq_ignore_ascii_case("cue")).then_some(50)
    }

    fn output_mime(&self) -> &'static str {
        "audio/flac"
    }

    fn output_extension(&self) -> &'static str {
        "flac"
    }

    fn quality(&self) -> CueRenderQuality {
        CueRenderQuality::Lossless
    }

    fn render(
        &self,
        path: &Path,
        start_sample: i64,
        end_sample: Option<i64>,
        _tags: &RenderTags,
    ) -> Result<Vec<u8>> {
        transcode_file_to_bytes(
            path,
            FfmpegOutput::FlacSourceRate,
            Some(FilterRange::Samples(SampleRange {
                start: start_sample.max(0),
                end: end_sample,
            })),
        )
        .map(|audio| audio.bytes)
    }
}

pub fn transcode_file_for_browser(
    path: &Path,
    format: PlaybackTranscodeFormat,
) -> Result<TranscodedAudio> {
    transcode_file_to_bytes(path, FfmpegOutput::Playback(format), None)
}

#[cfg(unix)]
pub fn transcode_file_range_for_browser_to_fd(
    path: &Path,
    format: PlaybackTranscodeFormat,
    start_ms: i64,
    end_ms: Option<i64>,
    output_fd: RawFd,
) -> Result<()> {
    transcode_file_to_fd(
        path,
        output_fd,
        FfmpegOutput::Playback(format),
        Some(FilterRange::Time(TimeRange {
            start_ms: start_ms.max(0),
            end_ms,
        })),
    )
}

pub fn transcode_bytes_for_browser(
    bytes: Vec<u8>,
    input_extension: &str,
    format: PlaybackTranscodeFormat,
) -> Result<TranscodedAudio> {
    let dir = tempfile::tempdir().context("creating ffmpeg input tempdir")?;
    let input_path = dir
        .path()
        .join(format!("input.{}", input_extension.trim_start_matches('.')));
    fs::write(&input_path, bytes).with_context(|| format!("writing {}", input_path.display()))?;
    transcode_file_to_bytes(&input_path, FfmpegOutput::Playback(format), None)
}

#[derive(Debug, Clone, Copy)]
struct SampleRange {
    start: i64,
    end: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
struct TimeRange {
    start_ms: i64,
    end_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
enum FilterRange {
    Samples(SampleRange),
    Time(TimeRange),
}

#[derive(Debug, Clone, Copy)]
enum FfmpegOutput {
    FlacSourceRate,
    Playback(PlaybackTranscodeFormat),
}

impl FfmpegOutput {
    fn mime(self) -> &'static str {
        match self {
            Self::FlacSourceRate => "audio/flac",
            Self::Playback(format) => format.mime(),
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::FlacSourceRate => "flac",
            Self::Playback(format) => format.extension(),
        }
    }

    fn container_format(self) -> &'static str {
        match self {
            Self::FlacSourceRate | Self::Playback(PlaybackTranscodeFormat::Flac48k) => "flac",
            Self::Playback(PlaybackTranscodeFormat::Opus256k) => "ogg",
        }
    }

    fn quality(self) -> CueRenderQuality {
        match self {
            Self::FlacSourceRate => CueRenderQuality::Lossless,
            Self::Playback(PlaybackTranscodeFormat::Opus256k) => CueRenderQuality::Lossy,
            Self::Playback(PlaybackTranscodeFormat::Flac48k) => CueRenderQuality::Lossless,
        }
    }

    fn encoder_name(self) -> Option<&'static str> {
        match self {
            Self::FlacSourceRate | Self::Playback(PlaybackTranscodeFormat::Flac48k) => Some("flac"),
            Self::Playback(PlaybackTranscodeFormat::Opus256k) => Some("libopus"),
        }
    }

    fn target_rate(self, source_rate: u32) -> u32 {
        match self {
            Self::FlacSourceRate => source_rate,
            Self::Playback(_) => 48_000,
        }
    }

    fn bit_rate(self) -> Option<usize> {
        match self {
            Self::Playback(PlaybackTranscodeFormat::Opus256k) => Some(256_000),
            _ => None,
        }
    }

    fn is_flac(self) -> bool {
        matches!(
            self,
            Self::FlacSourceRate | Self::Playback(PlaybackTranscodeFormat::Flac48k)
        )
    }
}

fn transcode_file_to_bytes(
    input: &Path,
    output: FfmpegOutput,
    range: Option<FilterRange>,
) -> Result<TranscodedAudio> {
    init_ffmpeg()?;
    let dir = tempfile::tempdir().context("creating ffmpeg output tempdir")?;
    let output_path = dir.path().join(format!("output.{}", output.extension()));
    let stats = transcode_file_to_path(input, &output_path, output, range)?;
    let mut bytes =
        fs::read(&output_path).with_context(|| format!("reading {}", output_path.display()))?;
    if output.is_flac() {
        patch_flac_total_samples(&mut bytes, stats.samples)?;
    }
    Ok(TranscodedAudio {
        bytes,
        mime: output.mime(),
        extension: output.extension(),
        quality: output.quality(),
    })
}

fn init_ffmpeg() -> Result<()> {
    FFMPEG_INIT
        .get_or_try_init(|| {
            ffmpeg::init().context("initializing ffmpeg")?;
            ffmpeg::log::set_level(ffmpeg::log::Level::Warning);
            Ok(())
        })
        .copied()
}

fn transcode_file_to_path(
    input: &Path,
    output_path: &Path,
    output: FfmpegOutput,
    range: Option<FilterRange>,
) -> Result<TranscodeStats> {
    init_ffmpeg()?;
    let mut ictx = format::input(input)
        .with_context(|| format!("opening ffmpeg input {}", input.display()))?;
    let mut octx = format::output(output_path)
        .with_context(|| format!("opening ffmpeg output {}", output_path.display()))?;
    transcode_contexts(&mut ictx, &mut octx, output_path, output, range)
}

#[cfg(unix)]
fn transcode_file_to_fd(
    input: &Path,
    output_fd: RawFd,
    output: FfmpegOutput,
    range: Option<FilterRange>,
) -> Result<()> {
    init_ffmpeg()?;
    let _fd_guard = RawFdGuard::new(output_fd);
    let mut ictx = format::input(input)
        .with_context(|| format!("opening ffmpeg input {}", input.display()))?;
    let output_url = format!("pipe:{output_fd}");
    let mut octx = format::output_as(&output_url, output.container_format())
        .context("opening ffmpeg pipe output")?;
    transcode_contexts(&mut ictx, &mut octx, Path::new(&output_url), output, range)?;
    Ok(())
}

#[cfg(unix)]
struct RawFdGuard {
    fd: Option<RawFd>,
}

#[cfg(unix)]
impl RawFdGuard {
    fn new(fd: RawFd) -> Self {
        Self { fd: Some(fd) }
    }
}

#[cfg(unix)]
impl Drop for RawFdGuard {
    fn drop(&mut self) {
        if let Some(fd) = self.fd.take() {
            unsafe {
                drop(UnixStream::from_raw_fd(fd));
            }
        }
    }
}

fn transcode_contexts(
    ictx: &mut format::context::Input,
    mut octx: &mut format::context::Output,
    output_path: &Path,
    output: FfmpegOutput,
    range: Option<FilterRange>,
) -> Result<TranscodeStats> {
    seek_input_for_range(ictx, range)?;
    let mut transcoder = Transcoder::new(ictx, octx, output_path, output, range)?;

    octx.set_metadata(ictx.metadata().to_owned());
    octx.write_header()
        .context("writing ffmpeg output header")?;

    for (stream, mut packet) in ictx.packets() {
        if stream.index() == transcoder.stream {
            if packet_is_after_range(&packet, stream.time_base(), range) {
                break;
            }
            packet.rescale_ts(stream.time_base(), transcoder.in_time_base);
            transcoder.send_packet_to_decoder(&packet)?;
            transcoder.receive_and_process_decoded_frames(&mut octx)?;
            if transcoder.filter_finished {
                break;
            }
        }
    }

    if !transcoder.filter_finished {
        transcoder.send_eof_to_decoder()?;
        transcoder.receive_and_process_decoded_frames(&mut octx)?;
    }
    transcoder.flush_filter()?;
    transcoder.get_and_process_filtered_frames(&mut octx)?;
    transcoder.send_eof_to_encoder()?;
    transcoder.receive_and_process_encoded_packets(&mut octx)?;
    octx.write_trailer()
        .context("writing ffmpeg output trailer")?;
    Ok(TranscodeStats {
        samples: transcoder.encoded_duration,
    })
}

fn seek_input_for_range(
    ictx: &mut format::context::Input,
    range: Option<FilterRange>,
) -> Result<()> {
    let Some(FilterRange::Time(range)) = range else {
        return Ok(());
    };
    if range.start_ms <= 0 {
        return Ok(());
    }
    let seek_ms = range.start_ms.saturating_sub(1_000);
    let position = seek_ms.rescale((1, 1000), ffmpeg::rescale::TIME_BASE);
    ictx.seek(position, ..position)
        .context("seeking ffmpeg input")?;
    Ok(())
}

fn packet_is_after_range(
    packet: &ffmpeg::Packet,
    time_base: ffmpeg::Rational,
    range: Option<FilterRange>,
) -> bool {
    let Some(FilterRange::Time(TimeRange {
        end_ms: Some(end_ms),
        ..
    })) = range
    else {
        return false;
    };
    packet
        .pts()
        .or_else(|| packet.dts())
        .map(|ts| ts.rescale(time_base, (1, 1000)) > end_ms.saturating_add(1_000))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy)]
struct TranscodeStats {
    samples: i64,
}

fn build_filter(
    spec: &str,
    decoder: &codec::decoder::Audio,
    encoder: &codec::encoder::Audio,
) -> Result<filter::Graph> {
    let mut graph = filter::Graph::new();
    let args = format!(
        "time_base={}:sample_rate={}:sample_fmt={}:channel_layout=0x{:x}",
        decoder.time_base(),
        decoder.rate(),
        decoder.format().name(),
        input_channel_layout(decoder).bits()
    );

    graph
        .add(
            &filter::find("abuffer").context("ffmpeg abuffer filter missing")?,
            "in",
            &args,
        )
        .context("adding ffmpeg abuffer")?;
    graph
        .add(
            &filter::find("abuffersink").context("ffmpeg abuffersink filter missing")?,
            "out",
            "",
        )
        .context("adding ffmpeg abuffersink")?;

    graph.output("in", 0)?.input("out", 0)?.parse(spec)?;
    graph.validate().context("validating ffmpeg filter graph")?;

    if let Some(codec) = encoder.codec() {
        if !codec
            .capabilities()
            .contains(ffmpeg::codec::capabilities::Capabilities::VARIABLE_FRAME_SIZE)
        {
            graph
                .get("out")
                .context("ffmpeg filter out node missing")?
                .sink()
                .set_frame_size(encoder.frame_size());
        }
    }

    Ok(graph)
}

fn patch_flac_total_samples(bytes: &mut [u8], samples: i64) -> Result<()> {
    if samples < 0 || bytes.len() < 42 || &bytes[0..4] != b"fLaC" {
        return Ok(());
    }
    let block_type = bytes[4] & 0x7f;
    let block_len =
        ((usize::from(bytes[5])) << 16) | ((usize::from(bytes[6])) << 8) | usize::from(bytes[7]);
    if block_type != 0 || block_len < 34 || bytes.len() < 8 + block_len {
        return Ok(());
    }
    const TOTAL_SAMPLES_MASK: u64 = (1_u64 << 36) - 1;
    let samples = (samples as u64) & TOTAL_SAMPLES_MASK;
    let offset = 8 + 10;
    let mut packed = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
    packed = (packed & !TOTAL_SAMPLES_MASK) | samples;
    bytes[offset..offset + 8].copy_from_slice(&packed.to_be_bytes());
    Ok(())
}

struct Transcoder {
    stream: usize,
    filter: filter::Graph,
    decoder: codec::decoder::Audio,
    encoder: codec::encoder::Audio,
    in_time_base: ffmpeg::Rational,
    out_time_base: ffmpeg::Rational,
    encoded_duration: i64,
    filter_finished: bool,
}

impl Transcoder {
    fn new(
        ictx: &mut format::context::Input,
        octx: &mut format::context::Output,
        output_path: &Path,
        output_target: FfmpegOutput,
        range: Option<FilterRange>,
    ) -> Result<Self> {
        let input = ictx
            .streams()
            .best(media::Type::Audio)
            .ok_or_else(|| anyhow!("ffmpeg found no audio stream"))?;
        let context = ffmpeg::codec::context::Context::from_parameters(input.parameters())
            .context("creating ffmpeg decoder context")?;
        let mut decoder = context.decoder().audio().context("opening audio decoder")?;
        decoder
            .set_parameters(input.parameters())
            .context("setting decoder parameters")?;

        let codec = select_encoder(octx, output_path, output_target)?;
        let global = octx
            .format()
            .flags()
            .contains(ffmpeg::format::flag::Flags::GLOBAL_HEADER);
        let mut output_stream = octx
            .add_stream(codec)
            .context("adding audio output stream")?;
        let context = ffmpeg::codec::context::Context::from_parameters(output_stream.parameters())
            .context("creating ffmpeg encoder context")?;
        let mut encoder = context.encoder().audio().context("opening audio encoder")?;

        let channel_layout = codec
            .channel_layouts()
            .map(|layouts| layouts.best(decoder.channel_layout().channels()))
            .unwrap_or(ffmpeg::channel_layout::ChannelLayout::STEREO);
        let channel_layout_name = channel_layout_name(channel_layout);
        let target_rate = output_target.target_rate(decoder.rate());

        if global {
            encoder.set_flags(ffmpeg::codec::flag::Flags::GLOBAL_HEADER);
        }
        encoder.set_rate(target_rate as i32);
        encoder.set_channel_layout(channel_layout);
        if let Some(mut formats) = codec.formats() {
            if let Some(format) = formats.next() {
                encoder.set_format(format);
            }
        }
        if let Some(bit_rate) = output_target.bit_rate() {
            encoder.set_bit_rate(bit_rate);
            encoder.set_compression(Some(10));
        } else {
            encoder.set_bit_rate(decoder.bit_rate());
            encoder.set_max_bit_rate(decoder.max_bit_rate());
        }
        encoder.set_time_base((1, target_rate as i32));
        output_stream.set_time_base((1, target_rate as i32));

        let encoder = encoder.open_as(codec).context("opening selected encoder")?;
        output_stream.set_parameters(&encoder);
        let filter_spec = filter_spec(
            range,
            target_rate,
            encoder.format().name(),
            channel_layout_name,
        );
        let filter = build_filter(&filter_spec, &decoder, &encoder)?;
        let in_time_base = decoder.time_base();
        let out_time_base = output_stream.time_base();

        Ok(Self {
            stream: input.index(),
            filter,
            decoder,
            encoder,
            in_time_base,
            out_time_base,
            encoded_duration: 0,
            filter_finished: false,
        })
    }

    fn send_packet_to_decoder(&mut self, packet: &ffmpeg::Packet) -> Result<()> {
        self.decoder
            .send_packet(packet)
            .context("sending ffmpeg packet to decoder")
    }

    fn send_eof_to_decoder(&mut self) -> Result<()> {
        self.decoder
            .send_eof()
            .context("sending ffmpeg decoder eof")
    }

    fn receive_and_process_decoded_frames(
        &mut self,
        octx: &mut format::context::Output,
    ) -> Result<()> {
        let mut decoded = frame::Audio::empty();
        while self.decoder.receive_frame(&mut decoded).is_ok() {
            if self.filter_finished {
                break;
            }
            let timestamp = decoded.timestamp();
            decoded.set_pts(timestamp);
            if !self.add_frame_to_filter(&decoded)? {
                self.filter_finished = true;
                break;
            }
            self.get_and_process_filtered_frames(octx)?;
        }
        Ok(())
    }

    fn add_frame_to_filter(&mut self, frame: &ffmpeg::Frame) -> Result<bool> {
        match self
            .filter
            .get("in")
            .context("ffmpeg filter in node missing")?
            .source()
            .add(frame)
        {
            Ok(()) => Ok(true),
            Err(ffmpeg::Error::Eof) => Ok(false),
            Err(err) => Err(err).context("adding frame to ffmpeg filter"),
        }
    }

    fn flush_filter(&mut self) -> Result<()> {
        if self.filter_finished {
            return Ok(());
        }
        self.filter
            .get("in")
            .context("ffmpeg filter in node missing")?
            .source()
            .flush()
            .or_else(|err| {
                if err == ffmpeg::Error::Eof {
                    Ok(())
                } else {
                    Err(err)
                }
            })
            .context("flushing ffmpeg filter")
    }

    fn get_and_process_filtered_frames(
        &mut self,
        octx: &mut format::context::Output,
    ) -> Result<()> {
        let mut filtered = frame::Audio::empty();
        while self
            .filter
            .get("out")
            .context("ffmpeg filter out node missing")?
            .sink()
            .frame(&mut filtered)
            .is_ok()
        {
            self.send_frame_to_encoder(&filtered)?;
            self.receive_and_process_encoded_packets(octx)?;
        }
        Ok(())
    }

    fn send_frame_to_encoder(&mut self, frame: &ffmpeg::Frame) -> Result<()> {
        self.encoder
            .send_frame(frame)
            .context("sending frame to ffmpeg encoder")
    }

    fn send_eof_to_encoder(&mut self) -> Result<()> {
        self.encoder
            .send_eof()
            .context("sending ffmpeg encoder eof")
    }

    fn receive_and_process_encoded_packets(
        &mut self,
        octx: &mut format::context::Output,
    ) -> Result<()> {
        let mut encoded = ffmpeg::Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            if encoded.size() == 0 {
                continue;
            }
            encoded.set_stream(0);
            encoded.rescale_ts(self.encoder.time_base(), self.out_time_base);
            self.encoded_duration = encoded
                .pts()
                .unwrap_or(self.encoded_duration)
                .saturating_add(encoded.duration());
            encoded
                .write_interleaved(octx)
                .context("writing ffmpeg encoded packet")?;
        }
        Ok(())
    }
}

fn channel_layout_name(layout: ffmpeg::channel_layout::ChannelLayout) -> &'static str {
    if layout == ffmpeg::channel_layout::ChannelLayout::MONO {
        "mono"
    } else {
        "stereo"
    }
}

fn input_channel_layout(decoder: &codec::decoder::Audio) -> ffmpeg::channel_layout::ChannelLayout {
    let layout = decoder.channel_layout();
    if layout.is_empty() || layout.bits() == 0 {
        ffmpeg::channel_layout::ChannelLayout::default(i32::from(decoder.channels()))
    } else {
        layout
    }
}

fn select_encoder(
    octx: &format::context::Output,
    output_path: &Path,
    output: FfmpegOutput,
) -> Result<codec::Audio> {
    if let Some(codec) = output
        .encoder_name()
        .and_then(ffmpeg::encoder::find_by_name)
        .and_then(|codec| codec.audio().ok())
    {
        return Ok(codec);
    }
    ffmpeg::encoder::find(octx.format().codec(output_path, media::Type::Audio))
        .ok_or_else(|| anyhow!("ffmpeg encoder missing for {}", output_path.display()))?
        .audio()
        .context("selected ffmpeg encoder is not audio")
}

fn filter_spec(
    range: Option<FilterRange>,
    target_rate: u32,
    sample_format: &str,
    channel_layout: &'static str,
) -> String {
    let mut parts = Vec::new();
    if let Some(range) = range {
        match range {
            FilterRange::Samples(range) => {
                let mut atrim = format!("atrim=start_sample={}", range.start);
                if let Some(end) = range.end {
                    atrim.push_str(&format!(":end_sample={}", end.max(range.start)));
                }
                parts.push(atrim);
            }
            FilterRange::Time(range) => {
                let mut atrim = format!("atrim=start={:.6}", range.start_ms as f64 / 1000.0);
                if let Some(end_ms) = range.end_ms {
                    atrim.push_str(&format!(
                        ":end={:.6}",
                        end_ms.max(range.start_ms) as f64 / 1000.0
                    ));
                }
                parts.push(atrim);
            }
        }
        parts.push(String::from("asetpts=PTS-STARTPTS"));
    }
    parts.push(format!("aresample={target_rate}"));
    parts.push(format!(
        "aformat=sample_fmts={sample_format}:channel_layouts={channel_layout}"
    ));
    parts.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::PlaybackTranscodeFormat;
    use std::f32::consts::PI;
    use std::io::{Read, Write};
    #[cfg(unix)]
    use std::os::fd::IntoRawFd;
    #[cfg(unix)]
    use std::os::unix::net::UnixStream;

    #[test]
    fn browser_transcodes_preserve_full_duration() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.wav");
        write_test_wav(&source, 5, 44_100);

        let opus = transcode_file_for_browser(&source, PlaybackTranscodeFormat::Opus256k).unwrap();
        assert!(opus.bytes.len() > 1024);
        assert_duration_ms("opus", &dir.path().join("out.opus"), &opus.bytes, 4_500);

        let flac = transcode_file_for_browser(&source, PlaybackTranscodeFormat::Flac48k).unwrap();
        assert!(flac.bytes.len() > 1024);
        assert_duration_ms("flac", &dir.path().join("out.flac"), &flac.bytes, 4_500);
    }

    #[cfg(unix)]
    #[test]
    fn browser_transcode_stream_writes_requested_range_to_fd() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.wav");
        write_test_wav(&source, 5, 44_100);

        let (mut reader, writer) = UnixStream::pair().unwrap();
        let output_fd = writer.into_raw_fd();
        let source_for_thread = source.clone();
        let handle = std::thread::spawn(move || {
            transcode_file_range_for_browser_to_fd(
                &source_for_thread,
                PlaybackTranscodeFormat::Opus256k,
                2_000,
                Some(4_000),
                output_fd,
            )
        });

        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        handle.join().unwrap().unwrap();
        assert!(bytes.len() > 1024);
        assert_duration_ms(
            "streamed opus range",
            &dir.path().join("out.opus"),
            &bytes,
            1_500,
        );
    }

    fn assert_duration_ms(label: &str, path: &Path, bytes: &[u8], min_ms: i64) {
        fs::write(path, bytes).unwrap();
        init_ffmpeg().unwrap();
        let ictx = format::input(path).unwrap();
        assert!(
            ictx.duration() / 1000 >= min_ms,
            "{label} container duration is not available"
        );
        let duration_ms = decoded_duration_ms(ictx);
        assert!(
            duration_ms >= min_ms,
            "{label} duration {duration_ms}ms is shorter than {min_ms}ms"
        );
    }

    fn decoded_duration_ms(mut ictx: format::context::Input) -> i64 {
        let input = ictx.streams().best(media::Type::Audio).unwrap();
        let stream_index = input.index();
        let context = ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();
        let mut decoder = context.decoder().audio().unwrap();
        decoder.set_parameters(input.parameters()).unwrap();
        let mut samples = 0_i64;
        for (stream, packet) in ictx.packets() {
            if stream.index() == stream_index {
                decoder.send_packet(&packet).unwrap();
                samples += receive_decoded_samples(&mut decoder);
            }
        }
        decoder.send_eof().unwrap();
        samples += receive_decoded_samples(&mut decoder);
        samples * 1000 / i64::from(decoder.rate())
    }

    fn receive_decoded_samples(decoder: &mut codec::decoder::Audio) -> i64 {
        let mut frame = frame::Audio::empty();
        let mut samples = 0_i64;
        while decoder.receive_frame(&mut frame).is_ok() {
            samples += frame.samples() as i64;
        }
        samples
    }

    fn write_test_wav(path: &Path, seconds: u32, sample_rate: u32) {
        let channels = 1_u16;
        let bits_per_sample = 16_u16;
        let samples = seconds * sample_rate;
        let data_len = samples * u32::from(channels) * u32::from(bits_per_sample / 8);
        let mut file = fs::File::create(path).unwrap();

        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        file.write_all(b"WAVE").unwrap();
        file.write_all(b"fmt ").unwrap();
        file.write_all(&16_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&(sample_rate * u32::from(channels) * 2).to_le_bytes())
            .unwrap();
        file.write_all(&(channels * 2).to_le_bytes()).unwrap();
        file.write_all(&bits_per_sample.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();

        for i in 0..samples {
            let phase = i as f32 / sample_rate as f32 * 440.0 * 2.0 * PI;
            let sample = (phase.sin() * f32::from(i16::MAX) * 0.25) as i16;
            file.write_all(&sample.to_le_bytes()).unwrap();
        }
    }
}
