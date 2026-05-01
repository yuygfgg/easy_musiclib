use crate::render::RenderTags;
use anyhow::{Context, Result, bail};
use libflac_sys::*;
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::fs::File;
use std::io::{BufReader, Cursor, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::Path;

const METADATA_STREAMINFO: u8 = 0;
const METADATA_SEEKTABLE: u8 = 3;
const METADATA_VORBIS_COMMENT: u8 = 4;
const METADATA_CUESHEET: u8 = 5;
const MIN_STREAMINFO_BLOCK_SIZE: u16 = 16;

pub(super) fn render_cue_track_exact(
    path: &Path,
    start_sample: i64,
    end_sample: Option<i64>,
    tags: &RenderTags,
) -> Result<Vec<u8>> {
    let metadata_blocks = read_metadata_blocks(path)?;
    let source_stream_info = get_stream_info(&metadata_blocks)
        .cloned()
        .context("flac streaminfo missing")?;
    let total_samples = source_stream_info.sample_count;

    let start = (start_sample.max(0) as u64).min(total_samples);
    let end = end_sample
        .map(|s| s.max(start_sample) as u64)
        .unwrap_or(total_samples)
        .min(total_samples);
    if start >= end {
        bail!("invalid flac cue range: {start}..{end}");
    }

    let frame_layout = scan_frame_layout(path, &source_stream_info)?;
    let (frame_bytes, stats) =
        render_frame_range(path, &source_stream_info, &frame_layout, start, end)?;

    if stats.samples != end - start {
        bail!(
            "flac slice rendered {} samples, expected {}",
            stats.samples,
            end - start
        );
    }

    let mut metadata_blocks = prepare_metadata_blocks(&metadata_blocks, tags);
    update_stream_info(&mut metadata_blocks, &source_stream_info, stats)?;

    let mut out = Vec::with_capacity(metadata_blocks.len() * 128 + frame_bytes.len() + 4);
    out.write_all(b"fLaC")?;
    for block in &metadata_blocks {
        block.write_to(&mut out)?;
    }
    out.write_all(&frame_bytes)?;
    Ok(out)
}

fn render_frame_range(
    path: &Path,
    stream_info: &StreamInfoBlock,
    frames: &[FrameLayout],
    start: u64,
    end: u64,
) -> Result<(Vec<u8>, FrameWriteStats)> {
    let mut out = Vec::new();
    let mut stats = FrameWriteStats::default();
    let mut full_frame_start = start;

    if let Some(frame) = frames.iter().find(|frame| frame.contains_sample(start)) {
        if start > frame.sample_pos {
            let wanted = end.min(frame.end_sample()) - start;
            let mut data = decode_frame_samples(path, stream_info, frame)
                .with_context(|| format!("decoding flac head frame {}", path.display()))?;
            trim_decoded_samples(&mut data, start - frame.sample_pos, wanted)?;
            let decoded = frame_sample_len(&data);
            let encoded = encode_samples(stream_info, &data).context("encoding flac head frame")?;
            write_encoded_frames(&encoded, stream_info, &mut out, &mut stats)?;
            full_frame_start = start + decoded;
        }
    }

    let mut reuse_until = end;
    let mut tail_frames = Vec::new();
    if full_frame_start < end {
        if let Some(frame) = frames
            .iter()
            .find(|frame| frame.sample_pos < end && end < frame.end_sample())
        {
            if frame.sample_pos >= full_frame_start {
                let wanted = end - frame.sample_pos;
                let mut data = decode_frame_samples(path, stream_info, frame)
                    .with_context(|| format!("decoding flac tail frame {}", path.display()))?;
                trim_decoded_samples(&mut data, 0, wanted)?;
                tail_frames =
                    encode_samples(stream_info, &data).context("encoding flac tail frame")?;
                reuse_until = frame.sample_pos;
            }
        }
    }

    let file = File::open(path).with_context(|| format!("opening flac {}", path.display()))?;
    let mut reader = BufReader::new(file);
    for frame in frames {
        if frame.sample_pos >= full_frame_start && frame.end_sample() <= reuse_until {
            reader.seek(SeekFrom::Start(frame.byte_offset))?;
            let flac_frame = FlacFrame::read_from(&mut reader, stream_info, frame.byte_size)?;
            write_frame(flac_frame, &mut out, &mut stats)?;
        }
    }

    write_encoded_frames(&tail_frames, stream_info, &mut out, &mut stats)?;

    if !stats.saw_frame {
        bail!("flac slice produced no frames");
    }
    Ok((out, stats))
}

fn write_encoded_frames(
    frames: &[Vec<u8>],
    stream_info: &StreamInfoBlock,
    out: &mut Vec<u8>,
    stats: &mut FrameWriteStats,
) -> Result<()> {
    for bytes in frames {
        let frame = FlacFrame::read_from(&bytes[..], stream_info, bytes.len() as u64)?;
        write_frame(frame, out, stats)?;
    }
    Ok(())
}

fn write_frame(mut frame: FlacFrame, out: &mut Vec<u8>, stats: &mut FrameWriteStats) -> Result<()> {
    frame.metadata.blocking_strategy = FlacBlockingStrategy::Variable;
    frame.metadata.position = FlacFramePosition::SampleCount(stats.samples);

    let frame_samples = frame.metadata.block_size.get_size();
    stats.push(frame_samples);
    out.write_all(&frame.into_bytes())?;
    Ok(())
}

fn frame_sample_len(data: &FlacFrameData) -> u64 {
    data.first()
        .map(|samples| samples.len() as u64)
        .unwrap_or(0)
}

fn decode_frame_samples(
    path: &Path,
    stream_info: &StreamInfoBlock,
    frame: &FrameLayout,
) -> Result<FlacFrameData> {
    let mut file = File::open(path).with_context(|| format!("opening flac {}", path.display()))?;
    file.seek(SeekFrom::Start(frame.byte_offset))?;
    let mut frame_bytes = vec![0u8; frame.byte_size as usize];
    file.read_exact(&mut frame_bytes)?;

    let mut decoder = FlacDecoder::new()?;
    decoder.init(
        Box::new(Cursor::new(single_frame_flac(stream_info, &frame_bytes))),
        path.display().to_string(),
    );
    decoder.process_metadata();
    let data = decoder
        .decode_frame()
        .context("libFLAC returned no decoded frame")?;
    decoder.finish();

    Ok(data)
}

fn single_frame_flac(stream_info: &StreamInfoBlock, frame_bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 4 + 34 + frame_bytes.len());
    out.extend(b"fLaC");
    out.push(0x80 | METADATA_STREAMINFO);
    out.extend([0, 0, 34]);
    out.extend(stream_info.to_bytes());
    out.extend(frame_bytes);
    out
}

fn trim_decoded_samples(data: &mut FlacFrameData, skip: u64, wanted: u64) -> Result<()> {
    let skip = skip as usize;
    let wanted = wanted as usize;
    for channel in data {
        if skip > channel.len() {
            bail!("libFLAC decoded frame shorter than requested skip");
        }
        channel.drain(..skip);
        channel.truncate(wanted);
    }
    Ok(())
}

fn encode_samples(stream_info: &StreamInfoBlock, data: &FlacFrameData) -> Result<Vec<Vec<u8>>> {
    if data.is_empty() || data[0].is_empty() {
        return Ok(Vec::new());
    }

    let mut encoder = FlacEncoder::new()?;
    encoder.set_params(
        stream_info.channels,
        stream_info.bits_per_sample,
        stream_info.sample_rate,
        Some(data[0].len() as u64),
    );
    encoder.init_stream();
    if !encoder.queue_encode(data) {
        encoder.finish();
        bail!("libFLAC failed to encode samples");
    }
    let bytes = encoder
        .finish()
        .context("libFLAC failed to finish encoding samples")?;
    extract_frames(&bytes)
}

fn extract_frames(bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    let first_frame_offset = first_frame_offset_from_bytes(bytes)?;
    let mut decoder = FlacDecoder::new()?;
    decoder.init(Box::new(Cursor::new(bytes.to_vec())), String::new());
    let mut offsets = decoder.scan_frame_offsets();
    decoder.finish();

    if offsets.is_empty() {
        return Ok(Vec::new());
    }
    offsets.push(first_frame_offset);
    let len = bytes.len() as u64;
    if offsets.last().copied() != Some(len) {
        offsets.push(len);
    }
    offsets.sort_unstable();
    offsets.dedup();

    Ok(offsets
        .windows(2)
        .map(|window| bytes[window[0] as usize..window[1] as usize].to_vec())
        .collect())
}

#[derive(Clone, Debug)]
struct FrameLayout {
    byte_offset: u64,
    byte_size: u64,
    sample_pos: u64,
    block_size: u16,
}

impl FrameLayout {
    fn end_sample(&self) -> u64 {
        self.sample_pos + u64::from(self.block_size)
    }

    fn contains_sample(&self, sample: u64) -> bool {
        self.sample_pos <= sample && sample < self.end_sample()
    }
}

fn scan_frame_layout(path: &Path, stream_info: &StreamInfoBlock) -> Result<Vec<FrameLayout>> {
    let first_frame_offset = first_frame_offset_from_path(path)?;
    let file = File::open(path).with_context(|| format!("opening flac {}", path.display()))?;
    let file_len = file.metadata()?.len();
    let mut decoder = FlacDecoder::new()?;
    decoder.init(Box::new(BufReader::new(file)), path.display().to_string());
    let mut frame_offsets = decoder.scan_frame_offsets();
    decoder.finish();

    if frame_offsets.is_empty() {
        bail!("flac contains no frames");
    }
    frame_offsets.push(first_frame_offset);
    frame_offsets.sort_unstable();
    frame_offsets.dedup();
    if frame_offsets.last().copied() != Some(file_len) {
        frame_offsets.push(file_len);
    }

    let file = File::open(path).with_context(|| format!("opening flac {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut fixed_block_size = None;
    let mut frames = Vec::new();

    for window in frame_offsets.windows(2) {
        let byte_offset = window[0];
        let byte_size = window[1] - window[0];
        reader.seek(SeekFrom::Start(byte_offset))?;
        let frame = FlacFrame::read_from(&mut reader, stream_info, byte_size)?;
        let block_size = frame.metadata.block_size.get_size();
        let sample_pos = match frame.metadata.position {
            FlacFramePosition::SampleCount(samples) => samples,
            FlacFramePosition::FrameCount(frame_no) => {
                let block_size = *fixed_block_size.get_or_insert(block_size);
                u64::from(frame_no) * u64::from(block_size)
            }
        };
        frames.push(FrameLayout {
            byte_offset,
            byte_size,
            sample_pos,
            block_size,
        });
    }

    Ok(frames)
}

#[derive(Clone, Debug, Default)]
struct FlacMetadataBlock {
    is_last: bool,
    block_type: u8,
    content: FlacMetadataBlockContent,
}

impl FlacMetadataBlock {
    fn read_from(mut reader: impl Read) -> Result<Self> {
        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;
        let is_last = (header[0] & 0x80) != 0;
        let block_type = header[0] & 0x7f;
        let block_len = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;
        let mut bytes = vec![0u8; block_len];
        reader.read_exact(&mut bytes)?;
        let content = FlacMetadataBlockContent::parse(block_type, bytes)?;
        Ok(Self {
            is_last,
            block_type,
            content,
        })
    }

    fn write_to(&self, mut writer: impl Write) -> Result<()> {
        let content = self.content.to_bytes();
        if content.len() > 0x00ff_ffff {
            bail!("flac metadata block too large");
        }

        let mut header = (content.len() as u32).to_be_bytes();
        header[0] = self.block_type & 0x7f;
        if self.is_last {
            header[0] |= 0x80;
        }
        writer.write_all(&header)?;
        writer.write_all(&content)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
enum FlacMetadataBlockContent {
    StreamInfo(StreamInfoBlock),
    VorbisComment(VorbisCommentBlock),
    Raw(Vec<u8>),
}

impl Default for FlacMetadataBlockContent {
    fn default() -> Self {
        Self::Raw(Vec::new())
    }
}

impl FlacMetadataBlockContent {
    fn parse(block_type: u8, bytes: Vec<u8>) -> Result<Self> {
        match block_type {
            METADATA_STREAMINFO => Ok(Self::StreamInfo(StreamInfoBlock::from_bytes(&bytes)?)),
            METADATA_VORBIS_COMMENT => {
                Ok(Self::VorbisComment(VorbisCommentBlock::from_bytes(&bytes)?))
            }
            _ => Ok(Self::Raw(bytes)),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::StreamInfo(stream_info) => stream_info.to_bytes(),
            Self::VorbisComment(vorbis_comment) => vorbis_comment.to_bytes(),
            Self::Raw(bytes) => bytes.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct StreamInfoBlock {
    min_block_size: u16,
    max_block_size: u16,
    min_frame_size: u32,
    max_frame_size: u32,
    sample_rate: u32,
    channels: u8,
    bits_per_sample: u8,
    sample_count: u64,
    md5: [u8; 16],
}

impl StreamInfoBlock {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 34 {
            bail!("invalid flac streaminfo length {}", bytes.len());
        }

        let sample_rate =
            ((bytes[10] as u32) << 12) | ((bytes[11] as u32) << 4) | ((bytes[12] as u32) >> 4);
        let channels = ((bytes[12] & 0x0e) >> 1) + 1;
        let bits_per_sample = (((bytes[12] & 0x01) << 4) | (bytes[13] >> 4)) + 1;
        let sample_count =
            (u64::from(bytes[13] & 0x0f) << 32) | u64::from(read_u32_be(&bytes[14..18])?);
        let mut md5 = [0u8; 16];
        md5.copy_from_slice(&bytes[18..34]);

        Ok(Self {
            min_block_size: read_u16_be(&bytes[0..2])?,
            max_block_size: read_u16_be(&bytes[2..4])?,
            min_frame_size: read_u24_be(&bytes[4..7])?,
            max_frame_size: read_u24_be(&bytes[7..10])?,
            sample_rate,
            channels,
            bits_per_sample,
            sample_count,
            md5,
        })
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(34);
        out.extend(self.min_block_size.to_be_bytes());
        out.extend(self.max_block_size.to_be_bytes());
        out.extend(&self.min_frame_size.to_be_bytes()[1..4]);
        out.extend(&self.max_frame_size.to_be_bytes()[1..4]);

        let stored_channels = self.channels.saturating_sub(1) & 0x07;
        let stored_bits = self.bits_per_sample.saturating_sub(1) & 0x1f;
        let mut chunk = (self.sample_rate << 12).to_be_bytes();
        chunk[2] |= stored_channels << 1;
        chunk[2] |= stored_bits >> 4;
        chunk[3] = ((stored_bits & 0x0f) << 4) | ((self.sample_count >> 32) as u8 & 0x0f);
        out.extend(chunk);
        out.extend((self.sample_count as u32).to_be_bytes());
        out.extend(self.md5);
        out
    }
}

#[derive(Clone, Debug)]
struct VorbisCommentBlock {
    vendor: String,
    comments: HashMap<String, String>,
}

impl VorbisCommentBlock {
    fn new() -> Self {
        Self {
            vendor: "easy-musiclib".to_string(),
            comments: HashMap::new(),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut pointer = 0;
        let vendor_len = read_u32_le_at(bytes, &mut pointer)? as usize;
        let vendor = read_utf8_at(bytes, &mut pointer, vendor_len)?.to_string();
        let comment_count = read_u32_le_at(bytes, &mut pointer)? as usize;
        let mut comments = HashMap::new();

        for _ in 0..comment_count {
            let len = read_u32_le_at(bytes, &mut pointer)? as usize;
            let comment = read_utf8_at(bytes, &mut pointer, len)?;
            let Some((key, value)) = comment.split_once('=') else {
                bail!("invalid flac vorbis comment");
            };
            comments.insert(key.to_string(), value.to_string());
        }

        Ok(Self { vendor, comments })
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend((self.vendor.len() as u32).to_le_bytes());
        out.extend(self.vendor.as_bytes());
        out.extend((self.comments.len() as u32).to_le_bytes());
        for (key, value) in &self.comments {
            let comment = format!("{key}={value}");
            out.extend((comment.len() as u32).to_le_bytes());
            out.extend(comment.as_bytes());
        }
        out
    }

    fn remove_case_insensitive(&mut self, key: &str) {
        if let Some(existing) = self
            .comments
            .keys()
            .find(|existing| existing.eq_ignore_ascii_case(key))
            .cloned()
        {
            self.comments.remove(&existing);
        }
    }

    fn upsert(&mut self, key: &str, value: impl ToString) {
        let key = self
            .comments
            .keys()
            .find(|existing| existing.eq_ignore_ascii_case(key))
            .cloned()
            .unwrap_or_else(|| key.to_string());
        self.comments.insert(key, value.to_string());
    }
}

fn read_metadata_blocks(path: &Path) -> Result<Vec<FlacMetadataBlock>> {
    let file = File::open(path).with_context(|| format!("opening flac {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"fLaC" {
        bail!("not a native flac stream");
    }

    let mut blocks = Vec::new();
    loop {
        let block = FlacMetadataBlock::read_from(&mut reader)?;
        let is_last = block.is_last;
        blocks.push(block);
        if is_last {
            break;
        }
    }
    Ok(blocks)
}

fn first_frame_offset_from_path(path: &Path) -> Result<u64> {
    let file = File::open(path).with_context(|| format!("opening flac {}", path.display()))?;
    let mut reader = BufReader::new(file);
    first_frame_offset(&mut reader)
}

fn first_frame_offset_from_bytes(bytes: &[u8]) -> Result<u64> {
    let mut reader = Cursor::new(bytes);
    first_frame_offset(&mut reader)
}

fn first_frame_offset(reader: &mut (impl Read + Seek)) -> Result<u64> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"fLaC" {
        bail!("not a native flac stream");
    }

    loop {
        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;
        let is_last = (header[0] & 0x80) != 0;
        let block_len = u32::from_be_bytes([0, header[1], header[2], header[3]]) as i64;
        reader.seek(SeekFrom::Current(block_len))?;
        if is_last {
            return Ok(reader.stream_position()?);
        }
    }
}

fn get_stream_info(blocks: &[FlacMetadataBlock]) -> Option<&StreamInfoBlock> {
    blocks.iter().find_map(|block| match &block.content {
        FlacMetadataBlockContent::StreamInfo(stream_info) => Some(stream_info),
        _ => None,
    })
}

fn prepare_metadata_blocks(
    original: &[FlacMetadataBlock],
    tags: &RenderTags,
) -> Vec<FlacMetadataBlock> {
    let mut blocks = original
        .iter()
        .filter(|block| {
            block.block_type != METADATA_SEEKTABLE && block.block_type != METADATA_CUESHEET
        })
        .cloned()
        .collect::<Vec<_>>();

    if !blocks
        .iter()
        .any(|block| block.block_type == METADATA_VORBIS_COMMENT)
    {
        let insert_at = blocks
            .iter()
            .position(|block| block.block_type != METADATA_STREAMINFO)
            .unwrap_or(blocks.len());
        blocks.insert(
            insert_at,
            FlacMetadataBlock {
                is_last: false,
                block_type: METADATA_VORBIS_COMMENT,
                content: FlacMetadataBlockContent::VorbisComment(VorbisCommentBlock::new()),
            },
        );
    }

    for block in &mut blocks {
        if let FlacMetadataBlockContent::VorbisComment(vorbis_comment) = &mut block.content {
            apply_render_tags(vorbis_comment, tags);
        }
    }
    update_last_metadata_flags(&mut blocks);
    blocks
}

fn apply_render_tags(vorbis_comment: &mut VorbisCommentBlock, tags: &RenderTags) {
    vorbis_comment.remove_case_insensitive("CUESHEET");
    vorbis_comment.upsert("TITLE", &tags.title);
    vorbis_comment.upsert("ARTIST", &tags.artist);
    if let Some(album) = &tags.album {
        vorbis_comment.upsert("ALBUM", album);
    }
    if let Some(track_no) = tags.track_no {
        vorbis_comment.upsert("TRACKNUMBER", track_no);
    }
    if let Some(date) = &tags.date {
        vorbis_comment.upsert("DATE", date);
    }
}

fn update_stream_info(
    blocks: &mut [FlacMetadataBlock],
    source: &StreamInfoBlock,
    stats: FrameWriteStats,
) -> Result<()> {
    let Some(block) = blocks
        .iter_mut()
        .find(|block| block.block_type == METADATA_STREAMINFO)
    else {
        bail!("flac streaminfo missing");
    };

    let mut stream_info = source.clone();
    stream_info.min_block_size = stats.min_block_size.max(MIN_STREAMINFO_BLOCK_SIZE);
    stream_info.max_block_size = stats.max_block_size.max(MIN_STREAMINFO_BLOCK_SIZE);
    stream_info.min_frame_size = 0;
    stream_info.max_frame_size = 0;
    stream_info.sample_count = stats.samples;
    stream_info.md5 = [0; 16];
    block.content = FlacMetadataBlockContent::StreamInfo(stream_info);
    update_last_metadata_flags(blocks);
    Ok(())
}

fn update_last_metadata_flags(blocks: &mut [FlacMetadataBlock]) {
    let last_idx = blocks.len().saturating_sub(1);
    for (idx, block) in blocks.iter_mut().enumerate() {
        block.is_last = idx == last_idx;
    }
}

#[derive(Clone, Copy, Debug)]
struct FrameWriteStats {
    min_block_size: u16,
    max_block_size: u16,
    samples: u64,
    saw_frame: bool,
}

impl Default for FrameWriteStats {
    fn default() -> Self {
        Self {
            min_block_size: u16::MAX,
            max_block_size: 0,
            samples: 0,
            saw_frame: false,
        }
    }
}

impl FrameWriteStats {
    fn push(&mut self, block_size: u16) {
        self.saw_frame = true;
        self.min_block_size = self.min_block_size.min(block_size);
        self.max_block_size = self.max_block_size.max(block_size);
        self.samples += u64::from(block_size);
    }
}

#[derive(Clone, Debug)]
struct FlacFrame {
    metadata: FlacFrameMetadata,
    frame_data: Vec<u8>,
}

impl FlacFrame {
    fn read_from(reader: impl Read, stream_info: &StreamInfoBlock, size: u64) -> Result<Self> {
        let mut frame_data = Vec::with_capacity(size as usize);
        reader.take(size).read_to_end(&mut frame_data)?;
        if frame_data.len() < 7 {
            bail!("flac frame too short");
        }

        let (metadata, header_size) = FlacFrameMetadata::read(&frame_data, stream_info)?;
        let payload_start = header_size + 1;
        if payload_start > frame_data.len().saturating_sub(2) {
            bail!("invalid flac frame size");
        }
        let frame_data = frame_data[payload_start..frame_data.len() - 2].to_vec();

        Ok(Self {
            metadata,
            frame_data,
        })
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = self.metadata.to_bytes();
        bytes.extend(self.frame_data);
        let crc = crc16(&bytes);
        bytes.extend(crc.to_be_bytes());
        bytes
    }
}

#[derive(Clone, Debug)]
struct FlacFrameMetadata {
    blocking_strategy: FlacBlockingStrategy,
    block_size: FlacBlockSize,
    sample_rate: FlacSampleRate,
    channel_assignment: u8,
    sample_bits: u8,
    position: FlacFramePosition,
}

impl FlacFrameMetadata {
    fn read(bytes: &[u8], _stream_info: &StreamInfoBlock) -> Result<(Self, usize)> {
        if bytes.len() < 5 {
            bail!("flac frame header too short");
        }
        if bytes[0] != 0xff || (bytes[1] & 0xfe) != 0xf8 {
            bail!("invalid flac frame sync");
        }

        let mut pointer = 4;
        let blocking_strategy = match bytes[1] & 0x01 {
            0 => FlacBlockingStrategy::Fixed,
            1 => FlacBlockingStrategy::Variable,
            _ => unreachable!(),
        };

        let block_size_code = bytes[2] >> 4;
        if block_size_code == 0 {
            bail!("reserved flac block size");
        }
        let sample_rate_code = bytes[2] & 0x0f;
        if sample_rate_code == 0x0f {
            bail!("invalid flac sample rate code");
        }
        let channel_assignment = bytes[3] >> 4;
        if channel_assignment >= 0x0b {
            bail!("invalid flac channel assignment");
        }
        let sample_bits = (bytes[3] & 0x0e) >> 1;
        if sample_bits == 0x03 {
            bail!("reserved flac sample bits");
        }

        let (position, position_bytes) = read_utf8_number(&bytes[pointer..])?;
        pointer += position_bytes;
        let position = match blocking_strategy {
            FlacBlockingStrategy::Fixed => FlacFramePosition::FrameCount(position as u32),
            FlacBlockingStrategy::Variable => FlacFramePosition::SampleCount(position),
        };

        let block_size = match block_size_code {
            6 => {
                let raw = read_u8_at(bytes, &mut pointer)?;
                FlacBlockSize::Dynamic8(u16::from(raw) + 1)
            }
            7 => {
                let raw = read_u16_be_at(bytes, &mut pointer)?;
                FlacBlockSize::Dynamic16(raw + 1)
            }
            code => FlacBlockSize::Predefined(code),
        };

        let sample_rate = match sample_rate_code {
            0 => FlacSampleRate::FromStreamInfo,
            12 => FlacSampleRate::HeaderEndU8KHz(read_u8_at(bytes, &mut pointer)?),
            13 => FlacSampleRate::HeaderEndU16Hz(read_u16_be_at(bytes, &mut pointer)?),
            14 => FlacSampleRate::HeaderEndU16TenHz(read_u16_be_at(bytes, &mut pointer)?),
            code => FlacSampleRate::Predefined(code),
        };

        Ok((
            Self {
                blocking_strategy,
                block_size,
                sample_rate,
                channel_assignment,
                sample_bits,
                position,
            },
            pointer,
        ))
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(match self.blocking_strategy {
            FlacBlockingStrategy::Fixed => [0xff, 0xf8],
            FlacBlockingStrategy::Variable => [0xff, 0xf9],
        });
        out.push((self.block_size.code() << 4) | self.sample_rate.code());
        out.push((self.channel_assignment << 4) | (self.sample_bits << 1));
        out.extend(encode_utf8_number(self.position.to_u64()));

        match self.block_size {
            FlacBlockSize::Dynamic8(size) => out.push(size.saturating_sub(1) as u8),
            FlacBlockSize::Dynamic16(size) => out.extend(size.saturating_sub(1).to_be_bytes()),
            FlacBlockSize::Predefined(_) => {}
        }

        match self.sample_rate {
            FlacSampleRate::HeaderEndU8KHz(raw) => out.push(raw),
            FlacSampleRate::HeaderEndU16Hz(raw) | FlacSampleRate::HeaderEndU16TenHz(raw) => {
                out.extend(raw.to_be_bytes());
            }
            FlacSampleRate::FromStreamInfo | FlacSampleRate::Predefined(_) => {}
        }

        let crc = crc8(&out);
        out.push(crc);
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlacBlockingStrategy {
    Fixed,
    Variable,
}

#[derive(Clone, Copy, Debug)]
enum FlacBlockSize {
    Predefined(u8),
    Dynamic8(u16),
    Dynamic16(u16),
}

impl FlacBlockSize {
    fn code(&self) -> u8 {
        match self {
            Self::Predefined(code) => *code,
            Self::Dynamic8(_) => 6,
            Self::Dynamic16(_) => 7,
        }
    }

    fn get_size(&self) -> u16 {
        match self {
            Self::Predefined(1) => 192,
            Self::Predefined(2) => 576,
            Self::Predefined(3) => 1152,
            Self::Predefined(4) => 2304,
            Self::Predefined(5) => 4608,
            Self::Predefined(8) => 256,
            Self::Predefined(9) => 512,
            Self::Predefined(10) => 1024,
            Self::Predefined(11) => 2048,
            Self::Predefined(12) => 4096,
            Self::Predefined(13) => 8192,
            Self::Predefined(14) => 16384,
            Self::Predefined(15) => 32768,
            Self::Predefined(_) => 0,
            Self::Dynamic8(size) | Self::Dynamic16(size) => *size,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum FlacSampleRate {
    FromStreamInfo,
    Predefined(u8),
    HeaderEndU8KHz(u8),
    HeaderEndU16Hz(u16),
    HeaderEndU16TenHz(u16),
}

impl FlacSampleRate {
    fn code(&self) -> u8 {
        match self {
            Self::FromStreamInfo => 0,
            Self::Predefined(code) => *code,
            Self::HeaderEndU8KHz(_) => 12,
            Self::HeaderEndU16Hz(_) => 13,
            Self::HeaderEndU16TenHz(_) => 14,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum FlacFramePosition {
    SampleCount(u64),
    FrameCount(u32),
}

impl FlacFramePosition {
    fn to_u64(self) -> u64 {
        match self {
            Self::SampleCount(samples) => samples,
            Self::FrameCount(frames) => u64::from(frames),
        }
    }
}

fn read_utf8_number(bytes: &[u8]) -> Result<(u64, usize)> {
    let first = *bytes.first().context("flac frame position missing")?;
    let leading_ones = first.leading_ones() as usize;
    let extra_bytes = if leading_ones > 0 {
        leading_ones - 1
    } else {
        0
    };
    if extra_bytes > 6 || bytes.len() < 1 + extra_bytes {
        bail!("invalid flac frame position");
    }

    let bits = match extra_bytes {
        0 => 7,
        1 => 11,
        2 => 16,
        3 => 21,
        4 => 26,
        5 => 31,
        6 => 36,
        _ => unreachable!(),
    };

    let mut value =
        (u64::from(first) & (!(0xffu64 << (bits - extra_bytes * 6)))) << (extra_bytes * 6);
    for (idx, byte) in bytes[1..1 + extra_bytes].iter().enumerate() {
        if byte >> 6 != 0b10 {
            bail!("invalid flac frame position continuation byte");
        }
        value |= u64::from(byte & 0x3f) << ((extra_bytes - idx - 1) * 6);
    }
    Ok((value, 1 + extra_bytes))
}

fn encode_utf8_number(number: u64) -> Vec<u8> {
    let bits = u64::BITS - number.leading_zeros();
    let encoded_bytes = match bits {
        0..=7 => 1,
        8..=11 => 2,
        12..=16 => 3,
        17..=21 => 4,
        22..=26 => 5,
        27..=31 => 6,
        32..=36 => 7,
        _ => unreachable!(),
    };

    if encoded_bytes == 1 {
        return vec![number as u8];
    }

    let mut out = vec![0u8; encoded_bytes as usize];
    let mut number = number;
    for idx in (1..encoded_bytes as usize).rev() {
        out[idx] = 0b1000_0000 | (number as u8 & 0b0011_1111);
        number >>= 6;
    }
    out[0] = (0xffu8 << (8 - encoded_bytes)) | number as u8;
    out
}

fn crc8(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |mut crc, byte| {
        crc ^= *byte;
        for _ in 0..8 {
            crc = if (crc & 0x80) != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
        crc
    })
}

fn crc16(data: &[u8]) -> u16 {
    data.iter().fold(0u16, |mut crc, byte| {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if (crc & 0x8000) != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
        }
        crc
    })
}

fn read_u8_at(bytes: &[u8], pointer: &mut usize) -> Result<u8> {
    let byte = *bytes
        .get(*pointer)
        .context("early end while parsing flac")?;
    *pointer += 1;
    Ok(byte)
}

fn read_u16_be(bytes: &[u8]) -> Result<u16> {
    if bytes.len() != 2 {
        bail!("invalid u16 length");
    }
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u16_be_at(bytes: &[u8], pointer: &mut usize) -> Result<u16> {
    let value = read_u16_be(
        bytes
            .get(*pointer..*pointer + 2)
            .context("early end while parsing flac")?,
    )?;
    *pointer += 2;
    Ok(value)
}

fn read_u24_be(bytes: &[u8]) -> Result<u32> {
    if bytes.len() != 3 {
        bail!("invalid u24 length");
    }
    Ok(u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]))
}

fn read_u32_be(bytes: &[u8]) -> Result<u32> {
    if bytes.len() != 4 {
        bail!("invalid u32 length");
    }
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u32_le_at(bytes: &[u8], pointer: &mut usize) -> Result<u32> {
    let slice = bytes
        .get(*pointer..*pointer + 4)
        .context("early end while parsing flac")?;
    *pointer += 4;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_utf8_at<'a>(bytes: &'a [u8], pointer: &mut usize, len: usize) -> Result<&'a str> {
    let slice = bytes
        .get(*pointer..*pointer + len)
        .context("early end while parsing flac")?;
    *pointer += len;
    Ok(std::str::from_utf8(slice)?)
}

trait SeekableRead: Read + Seek {}
impl<T: Read + Seek> SeekableRead for T {}

type FlacFrameData = Vec<Vec<FLAC__int32>>;

struct DecoderClientData {
    reader: Box<dyn SeekableRead>,
    path: String,
    decoded: Option<FlacFrameData>,
}

struct FlacDecoder {
    inner: *mut FLAC__StreamDecoder,
    client_data: Option<DecoderClientData>,
}

unsafe impl Send for FlacDecoder {}
unsafe impl Sync for FlacDecoder {}

impl FlacDecoder {
    fn new() -> Result<Self> {
        let inner = unsafe {
            let decoder = FLAC__stream_decoder_new();
            if decoder.is_null() {
                bail!("libFLAC failed to create decoder");
            }
            FLAC__stream_decoder_set_metadata_ignore_all(decoder);
            FLAC__stream_decoder_set_md5_checking(decoder, false.into());
            decoder
        };

        Ok(Self {
            inner,
            client_data: None,
        })
    }

    fn init(&mut self, reader: Box<dyn SeekableRead>, path: String) {
        self.client_data = Some(DecoderClientData {
            reader,
            path,
            decoded: None,
        });

        unsafe {
            FLAC__stream_decoder_init_stream(
                self.inner,
                Some(decoder_read_cb),
                Some(decoder_seek_cb),
                Some(decoder_tell_cb),
                Some(decoder_length_cb),
                Some(decoder_eof_cb),
                Some(decoder_write_cb),
                None,
                Some(decoder_error_cb),
                &mut self.client_data as *mut _ as *mut c_void,
            );
        }
    }

    fn finish(&mut self) -> Option<Box<dyn SeekableRead>> {
        unsafe {
            FLAC__stream_decoder_finish(self.inner);
        }
        self.client_data.take().map(|data| data.reader)
    }

    fn scan_frame_offsets(&mut self) -> Vec<u64> {
        let mut frame_offsets = Vec::new();
        unsafe {
            FLAC__stream_decoder_process_until_end_of_metadata(self.inner);
            let mut position = 0;
            FLAC__stream_decoder_get_decode_position(self.inner, &mut position);
            frame_offsets.push(position);

            loop {
                FLAC__stream_decoder_skip_single_frame(self.inner);
                if FLAC__stream_decoder_get_state(self.inner) == FLAC__STREAM_DECODER_END_OF_STREAM
                {
                    break;
                }
                FLAC__stream_decoder_get_decode_position(self.inner, &mut position);
                frame_offsets.push(position);
            }
        }
        frame_offsets
    }

    fn process_metadata(&mut self) {
        unsafe {
            FLAC__stream_decoder_process_until_end_of_metadata(self.inner);
        }
    }

    fn decode_frame(&mut self) -> Option<FlacFrameData> {
        unsafe {
            let success = FLAC__stream_decoder_process_single(self.inner);
            if success != 0 {
                self.client_data.as_mut()?.decoded.take()
            } else {
                None
            }
        }
    }
}

impl Drop for FlacDecoder {
    fn drop(&mut self) {
        unsafe {
            FLAC__stream_decoder_delete(self.inner);
        }
    }
}

unsafe extern "C" fn decoder_read_cb(
    _decoder: *const FLAC__StreamDecoder,
    buffer: *mut FLAC__byte,
    bytes: *mut usize,
    client_data: *mut c_void,
) -> FLAC__StreamDecoderReadStatus {
    let client_data = client_data as *mut DecoderClientData;

    unsafe {
        let buffer = std::slice::from_raw_parts_mut(buffer, *bytes);
        match (*client_data).reader.read(buffer) {
            Ok(bytes_read) => {
                *bytes = bytes_read;
                if !buffer.is_empty() && bytes_read == 0 {
                    FLAC__STREAM_DECODER_READ_STATUS_END_OF_STREAM
                } else {
                    FLAC__STREAM_DECODER_READ_STATUS_CONTINUE
                }
            }
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                *bytes = 0;
                FLAC__STREAM_DECODER_READ_STATUS_END_OF_STREAM
            }
            Err(_) => {
                *bytes = 0;
                FLAC__STREAM_DECODER_READ_STATUS_ABORT
            }
        }
    }
}

unsafe extern "C" fn decoder_seek_cb(
    _decoder: *const FLAC__StreamDecoder,
    absolute_byte_offset: FLAC__uint64,
    client_data: *mut c_void,
) -> FLAC__StreamDecoderSeekStatus {
    let client_data = client_data as *mut DecoderClientData;

    unsafe {
        match (*client_data)
            .reader
            .seek(SeekFrom::Start(absolute_byte_offset))
        {
            Ok(_) => FLAC__STREAM_DECODER_SEEK_STATUS_OK,
            Err(_) => FLAC__STREAM_DECODER_SEEK_STATUS_ERROR,
        }
    }
}

unsafe extern "C" fn decoder_tell_cb(
    _decoder: *const FLAC__StreamDecoder,
    absolute_byte_offset: *mut FLAC__uint64,
    client_data: *mut c_void,
) -> FLAC__StreamDecoderTellStatus {
    let client_data = client_data as *mut DecoderClientData;

    unsafe {
        match (*client_data).reader.stream_position() {
            Ok(pos) => {
                *absolute_byte_offset = pos;
                FLAC__STREAM_DECODER_TELL_STATUS_OK
            }
            Err(_) => FLAC__STREAM_DECODER_TELL_STATUS_ERROR,
        }
    }
}

unsafe extern "C" fn decoder_length_cb(
    _decoder: *const FLAC__StreamDecoder,
    stream_length: *mut FLAC__uint64,
    client_data: *mut c_void,
) -> FLAC__StreamDecoderLengthStatus {
    let client_data = client_data as *mut DecoderClientData;

    unsafe {
        match stream_len(&mut *(*client_data).reader) {
            Ok(len) => {
                *stream_length = len;
                FLAC__STREAM_DECODER_LENGTH_STATUS_OK
            }
            Err(_) => FLAC__STREAM_DECODER_LENGTH_STATUS_ERROR,
        }
    }
}

unsafe extern "C" fn decoder_eof_cb(
    _decoder: *const FLAC__StreamDecoder,
    _client_data: *mut c_void,
) -> FLAC__bool {
    false.into()
}

unsafe extern "C" fn decoder_write_cb(
    _decoder: *const FLAC__StreamDecoder,
    frame: *const FLAC__Frame,
    buffer: *const *const FLAC__int32,
    client_data: *mut c_void,
) -> FLAC__StreamDecoderWriteStatus {
    unsafe {
        let channels = (*frame).header.channels as usize;
        let samples = (*frame).header.blocksize as usize;

        let buffers = std::slice::from_raw_parts(buffer, channels);
        let decoded = buffers
            .iter()
            .map(|&buf| std::slice::from_raw_parts(buf, samples).to_vec())
            .collect();

        let client_data = client_data as *mut DecoderClientData;
        (*client_data).decoded = Some(decoded);
    }

    FLAC__STREAM_DECODER_WRITE_STATUS_CONTINUE
}

unsafe extern "C" fn decoder_error_cb(
    decoder: *const FLAC__StreamDecoder,
    status: FLAC__StreamDecoderErrorStatus,
    client_data: *mut c_void,
) {
    unsafe {
        let client_data = client_data as *mut DecoderClientData;
        let path = (*client_data).path.as_str();
        let status_strings = &FLAC__StreamDecoderErrorStatusString as *const *const c_char;
        let status_str = CStr::from_ptr(*status_strings.add(status as usize))
            .to_str()
            .unwrap_or("unknown");
        let mut byte_offset = 0;
        FLAC__stream_decoder_get_decode_position(decoder, &mut byte_offset);
        eprintln!("error while decoding FLAC file {path}: {status_str} at 0x{byte_offset:X}");
    }
}

struct EncoderClientData {
    buffer: Vec<u8>,
}

struct FlacEncoder {
    inner: *mut FLAC__StreamEncoder,
    client_data: EncoderClientData,
}

unsafe impl Send for FlacEncoder {}
unsafe impl Sync for FlacEncoder {}

impl FlacEncoder {
    fn new() -> Result<Self> {
        let inner = unsafe { FLAC__stream_encoder_new() };
        if inner.is_null() {
            bail!("libFLAC failed to create encoder");
        }
        Ok(Self {
            inner,
            client_data: EncoderClientData { buffer: Vec::new() },
        })
    }

    fn set_params(
        &mut self,
        channels: u8,
        bits_per_sample: u8,
        sample_rate: u32,
        total_samples: Option<u64>,
    ) {
        unsafe {
            FLAC__stream_encoder_set_channels(self.inner, u32::from(channels));
            FLAC__stream_encoder_set_bits_per_sample(self.inner, u32::from(bits_per_sample));
            FLAC__stream_encoder_set_sample_rate(self.inner, sample_rate);
            if let Some(total_samples) = total_samples {
                FLAC__stream_encoder_set_total_samples_estimate(self.inner, total_samples);
            }
        }
    }

    fn init_stream(&mut self) {
        self.client_data.buffer.clear();
        unsafe {
            FLAC__stream_encoder_init_stream(
                self.inner,
                Some(encoder_write_cb),
                None,
                None,
                None,
                &mut self.client_data as *mut _ as *mut c_void,
            );
        }
    }

    fn queue_encode(&mut self, data: &FlacFrameData) -> bool {
        unsafe {
            let samples = data[0].len();
            let data = data
                .iter()
                .map(|channel_data| channel_data.as_ptr())
                .collect::<Vec<_>>();
            FLAC__stream_encoder_process(self.inner, data.as_ptr(), samples as u32) != 0
        }
    }

    fn finish(&mut self) -> Option<Vec<u8>> {
        let ok = unsafe { FLAC__stream_encoder_finish(self.inner) != 0 };
        if ok {
            Some(std::mem::take(&mut self.client_data.buffer))
        } else {
            self.client_data.buffer.clear();
            None
        }
    }
}

impl Drop for FlacEncoder {
    fn drop(&mut self) {
        unsafe {
            FLAC__stream_encoder_delete(self.inner);
        }
    }
}

unsafe extern "C" fn encoder_write_cb(
    _encoder: *const FLAC__StreamEncoder,
    buffer: *const FLAC__byte,
    bytes: usize,
    _samples: u32,
    _current_frame: u32,
    client_data: *mut c_void,
) -> FLAC__StreamEncoderWriteStatus {
    unsafe {
        let buffer = std::slice::from_raw_parts(buffer, bytes);
        let client_data = client_data as *mut EncoderClientData;
        (*client_data).buffer.extend(buffer);
    }

    FLAC__STREAM_DECODER_WRITE_STATUS_CONTINUE
}

fn stream_len(reader: &mut dyn SeekableRead) -> std::io::Result<u64> {
    let pos = reader.stream_position()?;
    let len = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(pos))?;
    Ok(len)
}
