//! The state of the blue button: idle, running (the 400 ms and 300 ms rule), stopped, failed,
//! and the reasons a result cannot be saved.

use std::time::Duration;

use vectrace::wire::{Output, Status};

/// A running job shows its percentage only after this long.
pub const SHOW_AFTER: Duration = Duration::from_millis(400);
/// Once shown, the running look stays at least this long.
pub const HOLD_FOR: Duration = Duration::from_millis(300);

/// What a job is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Phase {
    Idle,
    Running,
    Stopped,
    Failed(String),
}

/// What the blue button looks like.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Button {
    /// "Save as…", or dimmed with a reason.
    Rest,
    /// The fill, the percentage (`None` before the first one) and the x.
    Running(Option<u8>),
    /// "Stopped" and Retry.
    Stopped,
    /// "Failed" and Retry, with the reason for the tooltip.
    Failed(String),
}

/// The progress of the current job over time.
#[derive(Debug, Clone)]
pub struct Progress {
    phase: Phase,
    percent: Option<u8>,
    started: Duration,
    shown_at: Option<Duration>,
    held_until: Option<Duration>,
}

impl Default for Progress {
    fn default() -> Self {
        Self { phase: Phase::Idle, percent: None, started: Duration::ZERO, shown_at: None, held_until: None }
    }
}

impl Progress {
    /// A job starts at `now`; `immediate` shows it at once (a full trace before saving). A job
    /// that replaces one already shown keeps the running look.
    pub fn start(&mut self, now: Duration, immediate: bool) {
        let kept = (self.phase == Phase::Running).then_some(self.shown_at).flatten();
        self.phase = Phase::Running;
        self.percent = None;
        self.started = now;
        self.held_until = None;
        self.shown_at = if immediate { Some(kept.unwrap_or(now)) } else { kept };
    }

    /// The job reached `percent`.
    pub fn set_percent(&mut self, percent: u8) {
        if self.phase == Phase::Running {
            self.percent = Some(percent.min(100));
        }
    }

    /// The job ended with a result.
    pub fn finish(&mut self, now: Duration) {
        self.settle(now);
        self.held_until = self.shown_at.map(|at| at + HOLD_FOR);
        self.phase = Phase::Idle;
        self.percent = None;
    }

    /// The maker stopped the job.
    pub fn stop(&mut self) {
        self.phase = Phase::Stopped;
        self.reset_look();
    }

    /// The job failed for `reason` ("out of memory at 16 Mpx").
    pub fn fail(&mut self, reason: impl Into<String>) {
        self.phase = Phase::Failed(reason.into());
        self.reset_look();
    }

    /// Back to idle (a new picture, a new value).
    pub fn clear(&mut self) {
        self.phase = Phase::Idle;
        self.reset_look();
    }

    fn reset_look(&mut self) {
        self.percent = None;
        self.shown_at = None;
        self.held_until = None;
    }

    /// Records the moment a running job becomes visible.
    fn settle(&mut self, now: Duration) {
        if self.phase == Phase::Running && self.shown_at.is_none() && now >= self.started + SHOW_AFTER {
            self.shown_at = Some(self.started + SHOW_AFTER);
        }
    }

    /// Whether a job is running (visible or not).
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.phase == Phase::Running
    }

    /// Whether the button is Stopped or Failed (Retry is offered).
    #[must_use]
    pub fn is_stopped_or_failed(&self) -> bool {
        matches!(self.phase, Phase::Stopped | Phase::Failed(_))
    }

    /// The button at `now`.
    pub fn button(&mut self, now: Duration) -> Button {
        self.settle(now);
        match &self.phase {
            Phase::Running if self.shown_at.is_some() => Button::Running(self.percent),
            Phase::Idle if self.held_until.is_some_and(|until| now < until) => Button::Running(self.percent),
            Phase::Stopped => Button::Stopped,
            Phase::Failed(reason) => Button::Failed(reason.clone()),
            _ => Button::Rest,
        }
    }

    /// How long until the button changes by itself, if it will.
    #[must_use]
    pub fn wake_in(&self, now: Duration) -> Option<Duration> {
        match (&self.phase, self.shown_at, self.held_until) {
            (Phase::Running, None, _) => Some((self.started + SHOW_AFTER).saturating_sub(now)),
            (Phase::Idle, _, Some(until)) if now < until => Some(until.saturating_sub(now)),
            _ => None,
        }
    }
}

/// What Esc does, in order: stop a trace, disarm the pipette, close a menu or entry, then
/// return the keyboard to the canvas. It never closes the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Esc {
    /// Stop the running trace.
    StopTrace,
    /// Disarm an armed pipette.
    DisarmPipette,
    /// Close an open menu.
    CloseMenu,
    /// Restore and close a number entry.
    CloseEntry,
    /// Give the keyboard back to the canvas.
    FocusCanvas,
}

/// What is going on when Esc is pressed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // four independent facts of the window
pub struct EscState {
    /// A trace is running.
    pub running: bool,
    /// A pipette is armed.
    pub pipette_armed: bool,
    /// A menu is open.
    pub menu_open: bool,
    /// A number entry is open.
    pub entry_open: bool,
}

/// The first thing Esc does for this state.
#[must_use]
pub fn esc(state: EscState) -> Esc {
    if state.running {
        Esc::StopTrace
    } else if state.pipette_armed {
        Esc::DisarmPipette
    } else if state.menu_open {
        Esc::CloseMenu
    } else if state.entry_open {
        Esc::CloseEntry
    } else {
        Esc::FocusCanvas
    }
}

/// Why a Brightness result cannot be saved, `None` when it can.
#[must_use]
pub fn nothing_to_save(output: &Output, picture_pixels: u64) -> Option<String> {
    match output.status {
        Status::TooDetailed => Some("Too much detail: raise Ignore below.".into()),
        Status::Invalid => Some("This image can't be traced.".into()),
        Status::Traced if output.paths == 0 => {
            let direction = if output.foreground_pixels * 2 > picture_pixels { "too high" } else { "too low" };
            Some(format!("0 paths at this Threshold: {direction}."))
        }
        Status::Traced => None,
    }
}

/// The reason a worker that ended without a reply failed: its stderr and the size of the picture.
#[must_use]
pub fn failure_reason(stderr: &str, pixels: u64, killed_by_oom: bool) -> String {
    if killed_by_oom || stderr.contains("memory allocation") || stderr.contains("allocat") {
        let mpx = (pixels + 500_000) / 1_000_000;
        format!("out of memory at {mpx} Mpx")
    } else {
        "internal error".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_short_job_never_shows() {
        let mut p = Progress::default();
        p.start(ms(0), false);
        assert_eq!(p.button(ms(399)), Button::Rest);
        p.finish(ms(399));
        assert_eq!(p.button(ms(400)), Button::Rest);
        assert_eq!(p.wake_in(ms(400)), None);
    }

    #[test]
    fn the_percentage_shows_after_400_ms() {
        let mut p = Progress::default();
        p.start(ms(0), false);
        assert_eq!(p.wake_in(ms(100)), Some(ms(300)));
        p.set_percent(42);
        assert_eq!(p.button(ms(399)), Button::Rest);
        assert_eq!(p.button(ms(400)), Button::Running(Some(42)));
    }

    #[test]
    fn before_the_first_percent_the_running_button_has_none() {
        let mut p = Progress::default();
        p.start(ms(0), false);
        assert_eq!(p.button(ms(500)), Button::Running(None));
    }

    #[test]
    fn once_shown_it_stays_300_ms() {
        let mut p = Progress::default();
        p.start(ms(0), false);
        p.set_percent(90);
        assert_eq!(p.button(ms(450)), Button::Running(None.or(Some(90))));
        p.finish(ms(450));
        assert_eq!(p.button(ms(699)), Button::Running(None));
        assert_eq!(p.wake_in(ms(500)), Some(ms(200)));
        assert_eq!(p.button(ms(700)), Button::Rest);
    }

    #[test]
    fn a_full_trace_before_saving_shows_at_once() {
        let mut p = Progress::default();
        p.start(ms(1000), true);
        assert_eq!(p.button(ms(1000)), Button::Running(None));
    }

    #[test]
    fn a_job_replacing_a_shown_one_keeps_the_running_look() {
        let mut p = Progress::default();
        p.start(ms(0), false);
        assert!(matches!(p.button(ms(500)), Button::Running(_)));
        p.start(ms(500), false);
        assert_eq!(p.button(ms(501)), Button::Running(None));
    }

    #[test]
    fn stopped_and_failed_replace_the_running_look() {
        let mut p = Progress::default();
        p.start(ms(0), false);
        p.stop();
        assert_eq!(p.button(ms(600)), Button::Stopped);
        assert!(p.is_stopped_or_failed());
        p.start(ms(700), false);
        p.fail("out of memory at 16 Mpx");
        assert_eq!(p.button(ms(800)), Button::Failed("out of memory at 16 Mpx".into()));
        p.clear();
        assert_eq!(p.button(ms(900)), Button::Rest);
    }

    #[test]
    fn a_late_percent_after_a_stop_is_ignored() {
        let mut p = Progress::default();
        p.start(ms(0), false);
        p.stop();
        p.set_percent(10);
        assert_eq!(p.button(ms(500)), Button::Stopped);
    }

    #[test]
    fn esc_stops_then_disarms_then_closes_then_returns() {
        let all = EscState { running: true, pipette_armed: true, menu_open: true, entry_open: true };
        assert_eq!(esc(all), Esc::StopTrace);
        let all = EscState { running: false, ..all };
        assert_eq!(esc(all), Esc::DisarmPipette);
        let all = EscState { pipette_armed: false, ..all };
        assert_eq!(esc(all), Esc::CloseMenu);
        let all = EscState { menu_open: false, ..all };
        assert_eq!(esc(all), Esc::CloseEntry);
        assert_eq!(esc(EscState::default()), Esc::FocusCanvas);
    }

    fn output(status: Status, paths: usize, foreground: u64) -> Output {
        Output {
            status,
            paths,
            nodes: 0,
            specks_removed: 0,
            foreground_pixels: foreground,
            covers_whole_image: false,
            outlines: Vec::new(),
            palette: Vec::new(),
            layers: Vec::new(),
            joined: 0,
            texts: vectrace::wire::Texts::default(),
            shapes: Vec::new(),
            unclipped: 0,
        }
    }

    #[test]
    fn nothing_to_save_names_its_reason() {
        assert_eq!(nothing_to_save(&output(Status::Traced, 0, 10), 100).as_deref(), Some("0 paths at this Threshold: too low."));
        assert_eq!(nothing_to_save(&output(Status::Traced, 0, 90), 100).as_deref(), Some("0 paths at this Threshold: too high."));
        assert_eq!(nothing_to_save(&output(Status::TooDetailed, 0, 0), 100).as_deref(), Some("Too much detail: raise Ignore below."));
        assert_eq!(nothing_to_save(&output(Status::Traced, 3, 10), 100), None);
    }

    #[test]
    fn failure_reasons_follow_curvyo() {
        assert_eq!(failure_reason("memory allocation of 9 bytes failed", 16_000_000, false), "out of memory at 16 Mpx");
        assert_eq!(failure_reason("", 12_400_000, true), "out of memory at 12 Mpx");
        assert_eq!(failure_reason("thread panicked", 1, false), "internal error");
    }
}
