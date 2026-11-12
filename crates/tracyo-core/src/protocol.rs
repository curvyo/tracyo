//! The worker's request and reply frames as bytes: a tag, a little-endian length, the payload.

use vectrace::ocr::OcrLine;
use vectrace::{Rgb8, Selection, Swatches};

use crate::clip::Clip;
use crate::palette::ColourSpec;
use crate::picture::Picture;
use crate::request::TraceRequest;
use crate::settings::Settings;

/// Bytes of a frame header: one tag byte and a `u32` length.
pub const HEADER_LEN: usize = 5;

/// A frame that cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProtocolError {
    /// The tag is not one the protocol knows.
    #[error("unknown frame tag {0}")]
    UnknownTag(u8),
    /// The payload is shorter or longer than its tag says.
    #[error("malformed frame payload")]
    Malformed,
}

/// One part of a parts job.
#[derive(Debug, Clone, PartialEq)]
pub struct PartJob {
    /// The part's number, 0 for part 1.
    pub part: u8,
    /// How to trace it, its clip inside.
    pub request: TraceRequest,
    /// Its result from an earlier job (vectrace's numbers): not traced again.
    pub cached: Option<Vec<f64>>,
}

/// A job with all the parts that add something, in stacking order, bottom first.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceParts {
    /// The parts.
    pub jobs: Vec<PartJob>,
    /// The index in `jobs` of the part whose palette the joined result keeps.
    pub palette_of: u8,
}

/// What the app asks the worker.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Keep this picture for the traces that follow.
    Picture(Picture),
    /// Trace the kept picture.
    Trace(TraceRequest),
    /// Read the text of the kept picture (the full picture, whatever is traced from it).
    ReadText,
    /// Trace several parts and join them.
    TraceParts(TraceParts),
}

/// What the worker answers.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// The job reached this whole percent.
    Percent(u8),
    /// The finished result as vectrace's numbers.
    Wire(Vec<f64>),
    /// The job could not run.
    Error(String),
    /// The text lines of a `ReadText` request, in the pixels of the full picture.
    Lines(Vec<OcrLine>),
    /// The text reader could not be used; the picture is traced without text.
    TextFailed,
    /// One part of a parts job is finished (vectrace's numbers); the join follows as `Wire`.
    PartWire {
        /// The part's number.
        part: u8,
        /// Its result.
        wire: Vec<f64>,
    },
}

fn frame(tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.push(tag);
    // invariant: payloads are far below 4 GiB (a 12 Mpx picture is 48 MB).
    out.extend(
        u32::try_from(payload.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    out.extend(payload);
    out
}

/// The tag and payload length of a frame header.
#[must_use]
pub fn parse_header(header: [u8; HEADER_LEN]) -> (u8, usize) {
    let len = u32::from_le_bytes([header[1], header[2], header[3], header[4]]);
    (header[0], len as usize)
}

/// The frame of a picture request, made from a borrowed picture.
#[must_use]
pub fn encode_picture(p: &Picture) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8 + p.rgba.len());
    payload.extend(p.width.to_le_bytes());
    payload.extend(p.height.to_le_bytes());
    payload.extend(&p.rgba);
    frame(1, &payload)
}

/// A cursor over a payload.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProtocolError> {
        if self.0.len() < n {
            return Err(ProtocolError::Malformed);
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }
    fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, ProtocolError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f64(&mut self) -> Result<f64, ProtocolError> {
        let b = self.take(8)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    fn done(&self) -> Result<(), ProtocolError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ProtocolError::Malformed)
        }
    }
}

/// A colour as three bytes, with a leading 1; `None` is a single 0.
fn encode_colour(colour: Option<Rgb8>, out: &mut Vec<u8>) {
    match colour {
        Some(c) => out.extend([1, c.r, c.g, c.b]),
        None => out.push(0),
    }
}

fn decode_colour(r: &mut Reader<'_>) -> Result<Option<Rgb8>, ProtocolError> {
    match r.u8()? {
        0 => Ok(None),
        1 => Ok(Some(Rgb8::new(r.u8()?, r.u8()?, r.u8()?))),
        _ => Err(ProtocolError::Malformed),
    }
}

/// The method index, its Colour tolerance, its background and its picked colours.
fn encode_selection(selection: &Selection, out: &mut Vec<u8>) {
    out.push(selection.index());
    match selection {
        Selection::Brightness => {}
        Selection::RemoveBackground {
            background,
            tolerance,
        } => {
            out.push(*tolerance);
            encode_colour(*background, out);
        }
        Selection::PickColours {
            swatches,
            background,
            tolerance,
        } => {
            out.push(*tolerance);
            encode_colour(*background, out);
            // invariant: at most MAX_SWATCHES (8) colours.
            out.push(u8::try_from(swatches.as_slice().len()).unwrap_or(u8::MAX));
            for c in swatches.as_slice() {
                out.extend([c.r, c.g, c.b]);
            }
        }
    }
}

/// Multicolour's palette: 0 for Single colour, else 1, the count (0 is Auto), the fixed colours and
/// the excluded colours.
fn encode_colours(spec: Option<&ColourSpec>, out: &mut Vec<u8>) {
    let Some(spec) = spec else {
        out.push(0);
        return;
    };
    out.extend([1, spec.count.unwrap_or(0)]);
    for list in [&spec.fixed, &spec.excluded] {
        // invariant: a palette holds at most 16 colours.
        out.push(u8::try_from(list.len()).unwrap_or(u8::MAX));
        for c in list {
            out.extend([c.r, c.g, c.b]);
        }
    }
}

fn decode_colour_list(r: &mut Reader<'_>) -> Result<Vec<Rgb8>, ProtocolError> {
    let n = usize::from(r.u8()?);
    let mut list = Vec::with_capacity(n);
    for _ in 0..n {
        list.push(Rgb8::new(r.u8()?, r.u8()?, r.u8()?));
    }
    Ok(list)
}

fn decode_colours(r: &mut Reader<'_>) -> Result<Option<ColourSpec>, ProtocolError> {
    match r.u8()? {
        0 => Ok(None),
        1 => {
            let count = r.u8()?;
            let fixed = decode_colour_list(r)?;
            let excluded = decode_colour_list(r)?;
            Ok(Some(ColourSpec {
                count: (count != 0).then_some(count),
                fixed,
                excluded,
            }))
        }
        _ => Err(ProtocolError::Malformed),
    }
}

fn decode_selection(r: &mut Reader<'_>) -> Result<Selection, ProtocolError> {
    match r.u8()? {
        0 => Ok(Selection::Brightness),
        1 => {
            let tolerance = r.u8()?;
            let background = decode_colour(r)?;
            Ok(Selection::RemoveBackground {
                background,
                tolerance,
            })
        }
        2 => {
            let tolerance = r.u8()?;
            let background = decode_colour(r)?;
            let count = usize::from(r.u8()?);
            let mut colours = Vec::with_capacity(count);
            for _ in 0..count {
                colours.push(Rgb8::new(r.u8()?, r.u8()?, r.u8()?));
            }
            Ok(Selection::PickColours {
                swatches: Swatches::from_slice(&colours),
                background,
                tolerance,
            })
        }
        _ => Err(ProtocolError::Malformed),
    }
}

/// A text line: its quad, its string, the confidence and position of each character.
fn encode_line(line: &OcrLine, out: &mut Vec<u8>) {
    for corner in line.quad {
        for v in corner {
            out.extend(v.to_le_bytes());
        }
    }
    // invariant: a line's text and its per-character values are far below 4 GiB.
    out.extend(
        u32::try_from(line.text.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    out.extend(line.text.as_bytes());
    for list in [&line.confidence, &line.positions] {
        out.extend(u32::try_from(list.len()).unwrap_or(u32::MAX).to_le_bytes());
        for v in list {
            out.extend(v.to_le_bytes());
        }
    }
}

fn encode_lines(lines: &[OcrLine], out: &mut Vec<u8>) {
    // invariant: a picture has far fewer than 4 billion text lines.
    out.extend(u32::try_from(lines.len()).unwrap_or(u32::MAX).to_le_bytes());
    for line in lines {
        encode_line(line, out);
    }
}

/// A clip: 1 for Outside, 0 for Inside, the count of grid numbers, the numbers.
fn encode_clip(clip: &Clip, out: &mut Vec<u8>) {
    out.push(u8::from(clip.outside()));
    let units = clip.to_units();
    // invariant: a clip has far fewer than 4 billion numbers.
    out.extend(u32::try_from(units.len()).unwrap_or(u32::MAX).to_le_bytes());
    for v in units {
        out.extend(v.to_le_bytes());
    }
}

fn decode_clip(r: &mut Reader<'_>) -> Result<Clip, ProtocolError> {
    let outside = match r.u8()? {
        0 => false,
        1 => true,
        _ => return Err(ProtocolError::Malformed),
    };
    let n = r.u32()? as usize;
    // The bytes must be there before anything is reserved for them.
    if r.0.len() / 8 < n {
        return Err(ProtocolError::Malformed);
    }
    let units: Vec<i64> = (0..n)
        .map(|_| {
            r.take(8)
                .map(|b| i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
        })
        .collect::<Result<_, _>>()?;
    Clip::from_units(&units, outside).ok_or(ProtocolError::Malformed)
}

fn decode_floats(r: &mut Reader<'_>) -> Result<Vec<f64>, ProtocolError> {
    let n = r.u32()? as usize;
    // The bytes must be there before anything is reserved for them.
    if r.0.len() / 8 < n {
        return Err(ProtocolError::Malformed);
    }
    (0..n).map(|_| r.f64()).collect()
}

fn decode_lines(r: &mut Reader<'_>) -> Result<Vec<OcrLine>, ProtocolError> {
    let n = r.u32()? as usize;
    let mut lines = Vec::new();
    for _ in 0..n {
        let mut quad = [[0.0; 2]; 4];
        for corner in &mut quad {
            *corner = [r.f64()?, r.f64()?];
        }
        let len = r.u32()? as usize;
        let text =
            String::from_utf8(r.take(len)?.to_vec()).map_err(|_| ProtocolError::Malformed)?;
        let confidence = decode_floats(r)?;
        let positions = decode_floats(r)?;
        lines.push(OcrLine {
            quad,
            text,
            confidence,
            positions,
        });
    }
    Ok(lines)
}

/// The payload of a trace frame: the values, the lines and, only when there is one, the clip, so
/// a request without a clip is byte for byte the one of the first version.
#[must_use]
pub fn encode_trace(t: &TraceRequest) -> Vec<u8> {
    let s = &t.settings;
    let mut payload = vec![t.threshold, s.smooth];
    payload.extend(s.tolerance.to_le_bytes());
    payload.extend(s.ignore_below.to_le_bytes());
    payload.extend([
        s.corners,
        u8::from(s.upscale_auto),
        s.denoise,
        u8::from(t.reduced),
        u8::from(t.invert),
        u8::from(t.drop_border),
        u8::from(t.shapes),
    ]);
    encode_colours(t.colours.as_ref(), &mut payload);
    encode_selection(&t.selection, &mut payload);
    encode_lines(&t.lines, &mut payload);
    if let Some(clip) = &t.clip {
        encode_clip(clip, &mut payload);
    }
    payload
}

/// The trace request of the payload of [`encode_trace`].
///
/// # Errors
/// [`ProtocolError::Malformed`] unless the payload is a trace request.
pub fn decode_trace(payload: &[u8]) -> Result<TraceRequest, ProtocolError> {
    let mut r = Reader(payload);
    let threshold = r.u8()?;
    let smooth = r.u8()?;
    let tolerance = r.f64()?;
    let ignore_below = r.f64()?;
    let (corners, upscale, denoise, reduced) = (r.u8()?, r.u8()?, r.u8()?, r.u8()?);
    let (invert, drop_border, shapes) = (r.u8()?, r.u8()?, r.u8()?);
    let colours = decode_colours(&mut r)?;
    let selection = decode_selection(&mut r)?;
    let lines = decode_lines(&mut r)?;
    let clip = if r.0.is_empty() {
        None
    } else {
        Some(decode_clip(&mut r)?)
    };
    r.done()?;
    let settings = Settings {
        smooth,
        tolerance,
        ignore_below,
        corners,
        upscale_auto: upscale != 0,
        denoise,
        ..Settings::default()
    };
    Ok(TraceRequest {
        threshold,
        settings,
        selection,
        invert: invert != 0,
        drop_border: drop_border != 0,
        colours,
        reduced: reduced != 0,
        shapes: shapes != 0,
        lines,
        clip,
    })
}

fn encode_floats(numbers: &[f64], out: &mut Vec<u8>) {
    // invariant: a wire has far fewer than 4 billion numbers.
    out.extend(
        u32::try_from(numbers.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    for n in numbers {
        out.extend(n.to_le_bytes());
    }
}

/// The payload of a parts frame: the part whose palette the join keeps, the count of jobs, then
/// per job its part, its cached wire (a flag, then the numbers) and its trace request.
fn encode_parts(parts: &TraceParts) -> Vec<u8> {
    let mut out = vec![parts.palette_of];
    // invariant: at most 8 parts.
    out.extend(
        u32::try_from(parts.jobs.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    for job in &parts.jobs {
        out.push(job.part);
        match &job.cached {
            Some(wire) => {
                out.push(1);
                encode_floats(wire, &mut out);
            }
            None => out.push(0),
        }
        let request = encode_trace(&job.request);
        out.extend(
            u32::try_from(request.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        out.extend(request);
    }
    out
}

fn decode_parts(payload: &[u8]) -> Result<TraceParts, ProtocolError> {
    let mut r = Reader(payload);
    let palette_of = r.u8()?;
    let count = r.u32()? as usize;
    let mut jobs = Vec::new();
    for _ in 0..count {
        let part = r.u8()?;
        let cached = match r.u8()? {
            0 => None,
            1 => Some(decode_floats(&mut r)?),
            _ => return Err(ProtocolError::Malformed),
        };
        let len = r.u32()? as usize;
        let request = decode_trace(r.take(len)?)?;
        jobs.push(PartJob {
            part,
            request,
            cached,
        });
    }
    r.done()?;
    Ok(TraceParts { jobs, palette_of })
}

impl Request {
    /// The whole frame, header included.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Picture(p) => encode_picture(p),
            Self::Trace(t) => frame(2, &encode_trace(t)),
            Self::TraceParts(parts) => frame(4, &encode_parts(parts)),
            Self::ReadText => frame(3, &[]),
        }
    }

    /// The request of a frame's tag and payload.
    ///
    /// # Errors
    /// [`ProtocolError`] for an unknown tag or a payload of the wrong size.
    pub fn decode(tag: u8, payload: &[u8]) -> Result<Self, ProtocolError> {
        let mut r = Reader(payload);
        match tag {
            1 => {
                let (width, height) = (r.u32()?, r.u32()?);
                let rgba = r.0.to_vec();
                let expected = u64::from(width) * u64::from(height) * 4;
                if rgba.len() as u64 != expected {
                    return Err(ProtocolError::Malformed);
                }
                Ok(Self::Picture(Picture {
                    width,
                    height,
                    rgba,
                }))
            }
            2 => Ok(Self::Trace(decode_trace(payload)?)),
            4 => Ok(Self::TraceParts(decode_parts(payload)?)),
            3 => {
                r.done()?;
                Ok(Self::ReadText)
            }
            other => Err(ProtocolError::UnknownTag(other)),
        }
    }
}

impl Reply {
    /// The whole frame, header included.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Percent(p) => frame(1, &[*p]),
            Self::Wire(numbers) => {
                let mut payload = Vec::with_capacity(numbers.len() * 8);
                for n in numbers {
                    payload.extend(n.to_le_bytes());
                }
                frame(2, &payload)
            }
            Self::Error(text) => frame(3, text.as_bytes()),
            Self::Lines(lines) => {
                let mut payload = Vec::new();
                encode_lines(lines, &mut payload);
                frame(4, &payload)
            }
            Self::TextFailed => frame(5, &[]),
            Self::PartWire { part, wire } => {
                let mut payload = vec![*part];
                payload.extend(wire.iter().flat_map(|n| n.to_le_bytes()));
                frame(6, &payload)
            }
        }
    }

    /// The reply of a frame's tag and payload.
    ///
    /// # Errors
    /// [`ProtocolError`] for an unknown tag or a payload of the wrong size.
    pub fn decode(tag: u8, payload: &[u8]) -> Result<Self, ProtocolError> {
        match tag {
            1 => {
                let mut r = Reader(payload);
                let p = r.u8()?;
                r.done()?;
                Ok(Self::Percent(p))
            }
            2 => {
                if !payload.len().is_multiple_of(8) {
                    return Err(ProtocolError::Malformed);
                }
                let mut r = Reader(payload);
                let mut numbers = Vec::with_capacity(payload.len() / 8);
                while !r.0.is_empty() {
                    numbers.push(r.f64()?);
                }
                Ok(Self::Wire(numbers))
            }
            3 => Ok(Self::Error(String::from_utf8_lossy(payload).into_owned())),
            4 => {
                let mut r = Reader(payload);
                let lines = decode_lines(&mut r)?;
                r.done()?;
                Ok(Self::Lines(lines))
            }
            5 if payload.is_empty() => Ok(Self::TextFailed),
            6 => {
                let (part, rest) = payload.split_first().ok_or(ProtocolError::Malformed)?;
                if !rest.len().is_multiple_of(8) {
                    return Err(ProtocolError::Malformed);
                }
                let wire = rest
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|b| f64::from_le_bytes(*b))
                    .collect();
                Ok(Self::PartWire { part: *part, wire })
            }
            other => Err(ProtocolError::UnknownTag(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(frame: &[u8]) -> (u8, &[u8]) {
        let mut header = [0; HEADER_LEN];
        header.copy_from_slice(&frame[..HEADER_LEN]);
        let (tag, len) = parse_header(header);
        assert_eq!(frame.len(), HEADER_LEN + len);
        (tag, &frame[HEADER_LEN..])
    }

    fn round_request(r: &Request) -> Request {
        let bytes = r.encode();
        let (tag, payload) = split(&bytes);
        Request::decode(tag, payload).unwrap()
    }

    fn round_reply(r: &Reply) -> Reply {
        let bytes = r.encode();
        let (tag, payload) = split(&bytes);
        Reply::decode(tag, payload).unwrap()
    }

    #[test]
    fn a_picture_request_round_trips() {
        let r = Request::Picture(Picture {
            width: 2,
            height: 1,
            rgba: vec![1, 2, 3, 4, 5, 6, 7, 8],
        });
        assert_eq!(round_request(&r), r);
    }

    #[test]
    fn a_trace_request_round_trips() {
        let settings = Settings {
            smooth: 33,
            tolerance: 1.25,
            ignore_below: 2.5,
            corners: 12,
            upscale_auto: true,
            denoise: 2,
            ..Settings::default()
        };
        let r = Request::Trace(TraceRequest {
            threshold: 158,
            settings,
            selection: Selection::Brightness,
            invert: true,
            drop_border: true,
            colours: None,
            reduced: true,
            shapes: false,
            lines: Vec::new(),
            clip: None,
        });
        assert_eq!(round_request(&r), r);
    }

    fn plain(threshold: u8) -> TraceRequest {
        TraceRequest {
            threshold,
            settings: Settings::default(),
            selection: Selection::Brightness,
            invert: false,
            drop_border: false,
            colours: None,
            reduced: false,
            shapes: false,
            lines: Vec::new(),
            clip: None,
        }
    }

    #[test]
    fn a_parts_request_with_clips_and_cached_wires_round_trips() {
        let clip = Clip::of_regions(
            &[crate::regions::Region::rect(
                crate::regions::PxRect::new(0, 0, 50, 80),
                false,
            )],
            crate::regions::MaskMode::Outside,
            vectrace::PixelMap::scale(1.0, 1.0),
        )
        .unwrap();
        let mut second = plain(70);
        second.clip = clip;
        let r = Request::TraceParts(TraceParts {
            jobs: vec![
                PartJob {
                    part: 0,
                    request: plain(40),
                    cached: Some(vec![0.0, 1.5, -2.0]),
                },
                PartJob {
                    part: 3,
                    request: second,
                    cached: None,
                },
            ],
            palette_of: 1,
        });
        assert_eq!(round_request(&r), r);
    }

    #[test]
    fn a_clip_of_lassos_with_slanted_sides_round_trips_through_the_worker_protocol() {
        use crate::regions::{MaskMode, PxPoint, PxRect, Region};
        let corners = [
            PxPoint::new(0, 0),
            PxPoint::new(600, 0),
            PxPoint::new(0, 1000),
        ];
        let regions = [
            Region::lasso(&corners, false).unwrap(),
            Region::rect(PxRect::new(100, 100, 200, 200), true),
        ];
        let clip = Clip::of_regions(
            &regions,
            MaskMode::Inside,
            vectrace::PixelMap::scale(0.26, 0.26),
        )
        .unwrap();
        let mut request = plain(70);
        request.clip = clip;
        assert!(request.clip.is_some());
        let r = Request::Trace(request);
        assert_eq!(round_request(&r), r);
        let parts = Request::TraceParts(TraceParts {
            jobs: vec![PartJob {
                part: 1,
                request: {
                    let mut p = plain(40);
                    p.clip = Clip::of_regions(
                        &regions,
                        MaskMode::Outside,
                        vectrace::PixelMap::scale(0.26, 0.26),
                    )
                    .unwrap();
                    p
                },
                cached: None,
            }],
            palette_of: 0,
        });
        assert_eq!(round_request(&parts), parts);
    }

    #[test]
    fn a_malformed_parts_request_is_refused() {
        let r = Request::TraceParts(TraceParts {
            jobs: vec![PartJob {
                part: 0,
                request: plain(40),
                cached: None,
            }],
            palette_of: 0,
        });
        let bytes = r.encode();
        let (tag, payload) = split(&bytes);
        for cut in [1, 6, payload.len() - 1] {
            assert_eq!(
                Request::decode(tag, &payload[..cut]),
                Err(ProtocolError::Malformed),
                "cut at {cut}"
            );
        }
    }

    #[test]
    fn a_finished_part_round_trips() {
        let r = Reply::PartWire {
            part: 2,
            wire: vec![0.0, 3.25, 1e-9],
        };
        assert_eq!(round_reply(&r), r);
        assert_eq!(
            Reply::decode(6, &[]),
            Err(ProtocolError::Malformed),
            "no part number"
        );
    }

    #[test]
    fn the_trace_frame_of_a_request_without_a_clip_is_the_one_of_the_first_version() {
        let bytes = Request::Trace(plain(9)).encode();
        let (tag, payload) = split(&bytes);
        assert_eq!(tag, 2);
        assert_eq!(payload, encode_trace(&plain(9)).as_slice());
        // threshold, smooth, tolerance, ignore below, six switches and values, no colours,
        // Brightness, no lines.
        assert_eq!(payload.len(), 2 + 8 + 8 + 7 + 1 + 1 + 4);
    }

    fn line(text: &str) -> OcrLine {
        OcrLine {
            quad: [[1.5, 2.0], [30.25, 2.0], [30.25, 9.0], [1.5, 9.0]],
            text: text.to_owned(),
            confidence: vec![0.9, 0.75],
            positions: vec![1.5, 12.0],
        }
    }

    #[test]
    fn a_trace_request_with_shapes_and_lines_round_trips() {
        let r = Request::Trace(TraceRequest {
            threshold: 9,
            settings: Settings::default(),
            selection: Selection::Brightness,
            invert: false,
            drop_border: false,
            colours: None,
            reduced: false,
            shapes: true,
            lines: vec![line("PR\u{c4}ZISION"), line("")],
            clip: None,
        });
        assert_eq!(round_request(&r), r);
    }

    #[test]
    fn the_text_requests_and_replies_round_trip() {
        assert_eq!(round_request(&Request::ReadText), Request::ReadText);
        let lines = Reply::Lines(vec![line("MIT SYSTEM")]);
        assert_eq!(round_reply(&lines), lines);
        assert_eq!(
            round_reply(&Reply::Lines(Vec::new())),
            Reply::Lines(Vec::new())
        );
        assert_eq!(round_reply(&Reply::TextFailed), Reply::TextFailed);
    }

    #[test]
    fn a_line_count_larger_than_its_bytes_is_refused_not_reserved() {
        let bytes = Reply::Lines(vec![line("A")]).encode();
        let (tag, payload) = split(&bytes);
        let mut bad = payload.to_vec();
        bad[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Reply::decode(tag, &bad), Err(ProtocolError::Malformed));
        let mut cut = payload.to_vec();
        cut.truncate(cut.len() - 3);
        assert_eq!(Reply::decode(tag, &cut), Err(ProtocolError::Malformed));
    }

    #[test]
    fn every_selection_round_trips() {
        let colours = [Rgb8::new(1, 2, 3), Rgb8::new(250, 251, 252)];
        for selection in [
            Selection::Brightness,
            Selection::RemoveBackground {
                background: Some(Rgb8::new(255, 255, 255)),
                tolerance: 40,
            },
            Selection::RemoveBackground {
                background: None,
                tolerance: 0,
            },
            Selection::PickColours {
                swatches: Swatches::from_slice(&colours),
                background: None,
                tolerance: 19,
            },
            Selection::PickColours {
                swatches: Swatches::from_slice(&[]),
                background: Some(Rgb8::new(9, 9, 9)),
                tolerance: 100,
            },
        ] {
            let r = Request::Trace(TraceRequest {
                threshold: 1,
                settings: Settings::default(),
                selection,
                invert: false,
                drop_border: false,
                colours: None,
                reduced: false,
                shapes: false,
                lines: Vec::new(),
                clip: None,
            });
            assert_eq!(round_request(&r), r);
        }
    }

    #[test]
    fn an_unknown_method_in_a_trace_is_refused() {
        let bytes = Request::Trace(TraceRequest {
            threshold: 1,
            settings: Settings::default(),
            selection: Selection::Brightness,
            invert: false,
            drop_border: false,
            colours: None,
            reduced: false,
            shapes: false,
            lines: Vec::new(),
            clip: None,
        })
        .encode();
        let (tag, payload) = split(&bytes);
        let mut bad = payload.to_vec();
        // The method index is followed by the four bytes of the line count.
        let at = bad.len() - 5;
        bad[at] = 7;
        assert_eq!(Request::decode(tag, &bad), Err(ProtocolError::Malformed));
    }

    #[test]
    fn a_palette_round_trips_with_auto_and_with_a_count() {
        let spec = |count| ColourSpec {
            count,
            fixed: vec![Rgb8::new(1, 2, 3), Rgb8::new(4, 5, 6)],
            excluded: vec![Rgb8::new(250, 251, 252)],
        };
        for colours in [
            Some(spec(None)),
            Some(spec(Some(16))),
            Some(ColourSpec {
                count: Some(2),
                fixed: Vec::new(),
                excluded: Vec::new(),
            }),
        ] {
            let r = Request::Trace(TraceRequest {
                threshold: 1,
                settings: Settings::default(),
                selection: Selection::Brightness,
                invert: false,
                drop_border: false,
                colours,
                reduced: false,
                shapes: false,
                lines: Vec::new(),
                clip: None,
            });
            assert_eq!(round_request(&r), r);
        }
    }

    #[test]
    fn replies_round_trip() {
        for r in [
            Reply::Percent(42),
            Reply::Wire(vec![0.0, 1.5, -2.25, f64::MAX]),
            Reply::Wire(Vec::new()),
            Reply::Error("RangeError: ü".into()),
        ] {
            assert_eq!(round_reply(&r), r);
        }
    }

    #[test]
    fn the_header_carries_tag_and_length() {
        let bytes = Reply::Percent(7).encode();
        assert_eq!(bytes, [1, 1, 0, 0, 0, 7]);
        assert_eq!(parse_header([2, 0, 1, 0, 0]), (2, 256));
    }

    #[test]
    fn truncated_or_padded_payloads_are_refused() {
        let bytes = Request::Trace(TraceRequest {
            threshold: 1,
            settings: Settings::default(),
            selection: Selection::Brightness,
            invert: false,
            drop_border: false,
            colours: None,
            reduced: false,
            shapes: false,
            lines: Vec::new(),
            clip: None,
        })
        .encode();
        let (tag, payload) = split(&bytes);
        assert_eq!(
            Request::decode(tag, &payload[..payload.len() - 1]),
            Err(ProtocolError::Malformed)
        );
        let mut long = payload.to_vec();
        long.push(0);
        assert_eq!(Request::decode(tag, &long), Err(ProtocolError::Malformed));
        assert_eq!(
            Request::decode(1, &[1, 0, 0, 0, 1, 0, 0, 0, 9]),
            Err(ProtocolError::Malformed)
        );
        assert_eq!(Reply::decode(2, &[0; 9]), Err(ProtocolError::Malformed));
        assert_eq!(Reply::decode(1, &[]), Err(ProtocolError::Malformed));
    }

    #[test]
    fn unknown_tags_are_refused() {
        assert_eq!(Request::decode(9, &[]), Err(ProtocolError::UnknownTag(9)));
        assert_eq!(Reply::decode(0, &[]), Err(ProtocolError::UnknownTag(0)));
    }

    fn clip(outside: bool) -> Clip {
        use crate::regions::{MaskMode, PxRect, Region};
        let regions = [
            Region::rect(PxRect::new(0, 0, 50, 40), false),
            Region::rect(PxRect::new(10, 10, 20, 20), true),
        ];
        let mode = if outside {
            MaskMode::Outside
        } else {
            MaskMode::Inside
        };
        Clip::of_regions(&regions, mode, vectrace::PixelMap::scale(0.26, 0.26))
            .unwrap()
            .unwrap()
    }

    fn plain_trace() -> TraceRequest {
        TraceRequest {
            threshold: 100,
            settings: Settings::default(),
            selection: Selection::Brightness,
            invert: false,
            drop_border: false,
            colours: None,
            reduced: false,
            shapes: false,
            lines: Vec::new(),
            clip: None,
        }
    }

    #[test]
    fn a_trace_with_a_clip_round_trips_inside_and_outside() {
        for outside in [false, true] {
            let r = Request::Trace(TraceRequest {
                clip: Some(clip(outside)),
                ..plain_trace()
            });
            assert_eq!(round_request(&r), r);
        }
    }

    #[test]
    fn a_trace_without_a_clip_is_the_frame_of_the_first_version() {
        let with = Request::Trace(TraceRequest {
            clip: Some(clip(false)),
            ..plain_trace()
        })
        .encode();
        let without = Request::Trace(plain_trace()).encode();
        assert!(with.len() > without.len());
        // The payload of a request without a clip ends with the (empty) line list.
        assert!(without.ends_with(&[0, 0, 0, 0]));
        assert_eq!(
            round_request(&Request::Trace(plain_trace())),
            Request::Trace(plain_trace())
        );
    }

    #[test]
    fn a_damaged_clip_is_refused() {
        let Request::Trace(t) = Request::Trace(TraceRequest {
            clip: Some(clip(false)),
            ..plain_trace()
        }) else {
            unreachable!()
        };
        let bytes = Request::Trace(t).encode();
        let (tag, payload) = split(&bytes);
        assert_eq!(
            Request::decode(tag, &payload[..payload.len() - 1]),
            Err(ProtocolError::Malformed)
        );
        let mut bad_mode = payload.to_vec();
        let mode_at = payload.len() - clip(false).to_units().len() * 8 - 4 - 1;
        bad_mode[mode_at] = 7;
        assert_eq!(
            Request::decode(tag, &bad_mode),
            Err(ProtocolError::Malformed)
        );
    }
}
