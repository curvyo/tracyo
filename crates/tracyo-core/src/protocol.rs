//! The worker's request and reply frames as bytes: a tag, a little-endian length, the payload.

use vectrace::{Rgb8, Selection, Swatches};

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

/// What the app asks the worker.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Keep this picture for the traces that follow.
    Picture(Picture),
    /// Trace the kept picture.
    Trace(TraceRequest),
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

impl Request {
    /// The whole frame, header included.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Picture(p) => encode_picture(p),
            Self::Trace(t) => {
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
                ]);
                encode_colours(t.colours.as_ref(), &mut payload);
                encode_selection(&t.selection, &mut payload);
                frame(2, &payload)
            }
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
            2 => {
                let threshold = r.u8()?;
                let smooth = r.u8()?;
                let tolerance = r.f64()?;
                let ignore_below = r.f64()?;
                let (corners, upscale, denoise, reduced) = (r.u8()?, r.u8()?, r.u8()?, r.u8()?);
                let (invert, drop_border) = (r.u8()?, r.u8()?);
                let colours = decode_colours(&mut r)?;
                let selection = decode_selection(&mut r)?;
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
                Ok(Self::Trace(TraceRequest {
                    threshold,
                    settings,
                    selection,
                    invert: invert != 0,
                    drop_border: drop_border != 0,
                    colours,
                    reduced: reduced != 0,
                }))
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
        });
        assert_eq!(round_request(&r), r);
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
        })
        .encode();
        let (tag, payload) = split(&bytes);
        let mut bad = payload.to_vec();
        *bad.last_mut().unwrap() = 7;
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
}
