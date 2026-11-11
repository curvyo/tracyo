//! One percent from 0 to 100 over the text read and the trace of a job.

/// The parts of a job that report their own percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Reading the picture's text.
    Read,
    /// Tracing, after a read in the same job.
    TraceAfterRead,
    /// Tracing alone.
    Trace,
}

/// The weights of Curvyo's job percent (reading 40, tracing 15), as Tracyo's job has only these
/// two stages: reading takes 40 of the 55 parts, so 72.7 % of the bar.
const READ_WEIGHT: f64 = 40.0;
const TRACE_WEIGHT: f64 = 15.0;

/// The job's whole percent when `stage` is at its own `percent`. Never runs backwards from one
/// stage to the next, and ends at 100 with the trace.
#[must_use]
pub fn job_percent(stage: Stage, percent: u8) -> u8 {
    let own = f64::from(percent.min(100));
    let read_end = 100.0 * READ_WEIGHT / (READ_WEIGHT + TRACE_WEIGHT);
    let whole = match stage {
        Stage::Trace => own,
        Stage::Read => read_end * own / 100.0,
        Stage::TraceAfterRead => read_end + (100.0 - read_end) * own / 100.0,
    };
    // invariant: 0 to 100 plus a hair, a whole percent fits in u8.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        (whole + 1e-9).floor().clamp(0.0, 100.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trace_alone_reports_its_own_percent() {
        assert_eq!(job_percent(Stage::Trace, 0), 0);
        assert_eq!(job_percent(Stage::Trace, 42), 42);
        assert_eq!(job_percent(Stage::Trace, 100), 100);
    }

    #[test]
    fn reading_takes_the_first_part_and_tracing_the_rest() {
        assert_eq!(job_percent(Stage::Read, 0), 0);
        assert_eq!(job_percent(Stage::Read, 100), 72);
        assert_eq!(job_percent(Stage::TraceAfterRead, 0), 72);
        assert_eq!(job_percent(Stage::TraceAfterRead, 100), 100);
    }

    #[test]
    fn the_percent_never_runs_backwards_over_the_two_stages() {
        let mut last = 0;
        for stage in [Stage::Read, Stage::TraceAfterRead] {
            for p in 0..=100 {
                let now = job_percent(stage, p);
                assert!(now >= last, "{stage:?} {p}: {now} after {last}");
                last = now;
            }
        }
        assert_eq!(last, 100);
        assert_eq!(job_percent(Stage::Trace, 250), 100);
    }
}
