//! The worker mode of the binary: reads requests from a stream, traces, writes replies.

use std::io::{ErrorKind, Read, Write};

use tracyo_core::job::Session;
use tracyo_core::protocol::{HEADER_LEN, Reply, Request, parse_header};

/// Serves requests from `input` until it closes; every reply goes to `output`.
///
/// # Errors
/// The I/O error of a broken stream (the app is gone).
pub fn serve(mut input: impl Read, mut output: impl Write) -> std::io::Result<()> {
    let mut session: Option<Session> = None;
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
                reduced: false,
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
            reduced: false,
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
}
