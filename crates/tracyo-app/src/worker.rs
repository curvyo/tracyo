//! The worker mode of the binary: reads requests from a stream, traces, writes replies.

use std::io::{ErrorKind, Read, Write};

use tracyo_core::job::Session;
use tracyo_core::protocol::{HEADER_LEN, Reply, Request, parse_header};
use vectrace::ocr::{OcrError, OcrModel};

/// The OCR model's three files, embedded (ADR 0006); loaded only for a `ReadText` request.
fn embedded_model() -> Result<OcrModel, OcrError> {
    OcrModel::load(
        include_bytes!("../../../assets/models/paddleocr-latin/det.onnx"),
        include_bytes!("../../../assets/models/paddleocr-latin/rec.onnx"),
        include_str!("../../../assets/models/paddleocr-latin/dict.txt"),
    )
}

/// Serves requests from `input` until it closes; every reply goes to `output`.
///
/// # Errors
/// The I/O error of a broken stream (the app is gone).
pub fn serve(input: impl Read, output: impl Write) -> std::io::Result<()> {
    serve_with(input, output, &embedded_model)
}

/// [`serve`] with the model's loader given (a test hands in one that fails).
///
/// # Errors
/// The I/O error of a broken stream (the app is gone).
pub fn serve_with(
    mut input: impl Read,
    mut output: impl Write,
    load_model: &dyn Fn() -> Result<OcrModel, OcrError>,
) -> std::io::Result<()> {
    let mut session: Option<Session> = None;
    let mut model: Option<OcrModel> = None;
    loop {
        let mut header = [0_u8; HEADER_LEN];
        match input.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        }
        let (tag, len) = parse_header(header);
        let mut payload = vec![0_u8; len];
        input.read_exact(&mut payload)?;
        match Request::decode(tag, &payload) {
            Ok(Request::Picture(picture)) => session = Some(Session::new(picture)),
            Ok(Request::ReadText) => {
                let Some(session) = session.as_ref() else {
                    send(&mut output, &Reply::Error("no picture".into()))?;
                    continue;
                };
                if model.is_none() {
                    model = load_model().ok();
                }
                let Some(model) = model.as_ref() else {
                    send(&mut output, &Reply::TextFailed)?;
                    continue;
                };
                let mut broken = None;
                let read = session.read_text(model, &mut |percent| {
                    if broken.is_none()
                        && let Err(e) = send(&mut output, &Reply::Percent(percent))
                    {
                        broken = Some(e);
                    }
                });
                if let Some(e) = broken {
                    return Err(e);
                }
                match read {
                    Ok(lines) => send(&mut output, &Reply::Lines(lines))?,
                    Err(_) => send(&mut output, &Reply::TextFailed)?,
                }
            }
            Ok(Request::Trace(request)) => {
                let Some(session) = session.as_mut() else {
                    send(&mut output, &Reply::Error("no picture".into()))?;
                    continue;
                };
                let mut broken = None;
                let wire = session.run(&request, &mut |percent| {
                    if broken.is_none()
                        && let Err(e) = send(&mut output, &Reply::Percent(percent))
                    {
                        broken = Some(e);
                    }
                });
                if let Some(e) = broken {
                    return Err(e);
                }
                send(&mut output, &Reply::Wire(wire))?;
            }
            Ok(Request::TraceParts(parts)) => {
                let Some(session) = session.as_mut() else {
                    send(&mut output, &Reply::Error("no picture".into()))?;
                    continue;
                };
                // Both callbacks write to the same stream; the first failure ends the worker.
                let sink = std::cell::RefCell::new((&mut output, None::<std::io::Error>));
                let say = |reply: &Reply| {
                    let mut guard = sink.borrow_mut();
                    let (out, broken) = &mut *guard;
                    if broken.is_none()
                        && let Err(e) = send(out, reply)
                    {
                        *broken = Some(e);
                    }
                };
                let wire = session.run_parts(
                    &parts,
                    &mut |part, wire| {
                        say(&Reply::PartWire {
                            part,
                            wire: wire.to_vec(),
                        });
                    },
                    &mut |percent| say(&Reply::Percent(percent)),
                );
                if let Some(e) = sink.into_inner().1 {
                    return Err(e);
                }
                send(&mut output, &Reply::Wire(wire))?;
            }
            Err(e) => send(&mut output, &Reply::Error(e.to_string()))?,
        }
    }
}

fn send(output: &mut impl Write, reply: &Reply) -> std::io::Result<()> {
    output.write_all(&reply.encode())?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracyo_core::picture::Picture;
    use tracyo_core::request::TraceRequest;
    use tracyo_core::settings::Settings;

    fn replies(mut bytes: &[u8]) -> Vec<Reply> {
        let mut out = Vec::new();
        while !bytes.is_empty() {
            let mut header = [0; HEADER_LEN];
            header.copy_from_slice(&bytes[..HEADER_LEN]);
            let (tag, len) = parse_header(header);
            out.push(Reply::decode(tag, &bytes[HEADER_LEN..HEADER_LEN + len]).unwrap());
            bytes = &bytes[HEADER_LEN + len..];
        }
        out
    }

    #[test]
    fn a_picture_and_a_trace_give_percents_then_the_wire() {
        let mut rgba = vec![255; 20 * 20 * 4];
        for y in 5..15 {
            for x in 5..15 {
                let i = (y * 20 + x) * 4;
                rgba[i..i + 3].fill(0);
            }
        }
        let mut input = Request::Picture(Picture {
            width: 20,
            height: 20,
            rgba,
        })
        .encode();
        input.extend(
            Request::Trace(TraceRequest {
                threshold: 128,
                settings: Settings::default(),
                selection: vectrace::Selection::Brightness,
                invert: false,
                drop_border: false,
                colours: None,
                reduced: false,
                shapes: false,
                lines: Vec::new(),
                clip: None,
            })
            .encode(),
        );
        let mut output = Vec::new();
        serve(&input[..], &mut output).unwrap();
        let got = replies(&output);
        assert!(
            matches!(got.last(), Some(Reply::Wire(w)) if vectrace::wire::read(w).is_some_and(|o| o.paths == 1))
        );
        assert!(
            got.iter()
                .filter(|r| matches!(r, Reply::Percent(_)))
                .count()
                >= 1
        );
    }

    #[test]
    fn a_trace_without_a_picture_is_an_error_reply() {
        let input = Request::Trace(TraceRequest {
            threshold: 1,
            settings: Settings::default(),
            selection: vectrace::Selection::Brightness,
            invert: false,
            drop_border: false,
            colours: None,
            reduced: false,
            shapes: false,
            lines: Vec::new(),
            clip: None,
        })
        .encode();
        let mut output = Vec::new();
        serve(&input[..], &mut output).unwrap();
        assert!(matches!(replies(&output)[0], Reply::Error(_)));
    }

    #[test]
    fn a_closed_input_ends_the_worker() {
        serve(&[][..], Vec::new()).unwrap();
    }

    fn blank_picture() -> Request {
        Request::Picture(Picture {
            width: 32,
            height: 32,
            rgba: vec![255; 32 * 32 * 4],
        })
    }

    #[test]
    fn read_text_answers_percents_then_the_lines() {
        let mut input = blank_picture().encode();
        input.extend(Request::ReadText.encode());
        let mut output = Vec::new();
        serve(&input[..], &mut output).unwrap();
        let got = replies(&output);
        assert!(matches!(got.last(), Some(Reply::Lines(lines)) if lines.is_empty()));
        assert!(got.iter().any(|r| matches!(r, Reply::Percent(_))));
    }

    #[test]
    fn a_model_that_cannot_be_used_answers_text_failed_and_is_asked_once_per_request() {
        let mut input = blank_picture().encode();
        input.extend(Request::ReadText.encode());
        let mut output = Vec::new();
        let loads = std::cell::Cell::new(0);
        serve_with(&input[..], &mut output, &|| {
            loads.set(loads.get() + 1);
            Err(OcrError::WrongModel)
        })
        .unwrap();
        assert_eq!(replies(&output), [Reply::TextFailed]);
        assert_eq!(loads.get(), 1);
    }

    #[test]
    fn the_model_is_not_loaded_for_a_trace() {
        let mut input = blank_picture().encode();
        input.extend(
            Request::Trace(TraceRequest {
                threshold: 128,
                settings: Settings::default(),
                selection: vectrace::Selection::Brightness,
                invert: false,
                drop_border: false,
                colours: None,
                reduced: false,
                shapes: true,
                lines: Vec::new(),
                clip: None,
            })
            .encode(),
        );
        let mut output = Vec::new();
        serve_with(&input[..], &mut output, &|| panic!("no text is read")).unwrap();
        assert!(matches!(replies(&output).last(), Some(Reply::Wire(_))));
    }

    #[test]
    fn read_text_without_a_picture_is_an_error_reply() {
        let input = Request::ReadText.encode();
        let mut output = Vec::new();
        serve(&input[..], &mut output).unwrap();
        assert!(matches!(replies(&output)[0], Reply::Error(_)));
    }
}
