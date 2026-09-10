//! Detects gaps between the monotonic clock and the wall clock that are too large to be
//! routine timekeeping drift: the machine slept, or its system clock was corrected. This
//! module only measures the missing time — it does not decide what the caller should do
//! about it.

use log::warn;
use time::OffsetDateTime;
use tokio::time::{Duration, Instant};

/// How far the monotonic clock must fall behind the wall clock, tick to tick, before
/// [`JumpDetector::observe`] reports it. Below this, the gap is routine scheduling jitter
/// or a small time-server nudge, not a sleep or a clock correction.
pub(crate) const TIME_JUMP_THRESHOLD: Duration = Duration::from_secs(10);

/// Watches the monotonic clock and the wall clock tick to tick and reports when the
/// monotonic clock has fallen behind by more than [`TIME_JUMP_THRESHOLD`] — evidence the
/// machine slept or its system clock was corrected. A corrected system clock is treated
/// exactly like a sleep: in both cases real time has moved on and the caller needs to
/// catch up to it.
#[derive(Debug, Default)]
pub(crate) struct JumpDetector {
    last: Option<(Instant, OffsetDateTime)>,
}

impl JumpDetector {
    /// Records this tick's monotonic and wall clock readings and reports the time the
    /// monotonic clock lost since the previous call, if it is over
    /// [`TIME_JUMP_THRESHOLD`].
    ///
    /// Returns `None` on the first call (there is nothing yet to compare against), when
    /// the two clocks moved together, and when the wall clock moved backwards (a
    /// backwards step is still recorded, so the next call measures its gap from there,
    /// not from further back).
    pub(crate) fn observe(&mut self, mono: Instant, wall: OffsetDateTime) -> Option<Duration> {
        // Rebase unconditionally before either guard below can return early. A backwards
        // step that left `last` pointing at the reading from before it would have the
        // next call measure its gap against that stale reference instead of the rewound
        // one.
        let (last_mono, last_wall) = self.last.replace((mono, wall))?;

        let wall_delta = wall - last_wall;
        if wall_delta.is_negative() {
            warn!("Wall clock moved backwards by {wall_delta}; not reporting a time jump");
            return None;
        }
        // `time::Duration` is signed and `Duration` here is not; this only fails for a
        // value too large to represent as unsigned, which the negative check above has
        // already ruled out.
        let wall_delta: Duration = wall_delta.try_into().ok()?;

        let mono_delta = mono.saturating_duration_since(last_mono);
        let lost = wall_delta.saturating_sub(mono_delta);

        (lost >= TIME_JUMP_THRESHOLD).then_some(lost)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// A fixed wall-clock reading offset from the Unix epoch by `offset_secs`, so tests
    /// never depend on the real wall clock.
    fn wall(offset_secs: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(offset_secs)
    }

    #[test]
    fn first_observation_reports_nothing() {
        let mut detector = JumpDetector::default();
        assert_eq!(detector.observe(Instant::now(), wall(0)), None);
    }

    #[test]
    fn a_two_hour_sleep_reports_two_hours() {
        let mut detector = JumpDetector::default();
        let mono = Instant::now();
        assert_eq!(detector.observe(mono, wall(0)), None);

        // The machine slept for 2 hours: the wall clock jumped forward by 7200s, but
        // the monotonic clock only ticked forward by the 20ms it was actually running.
        let lost = detector
            .observe(mono + Duration::from_millis(20), wall(7200))
            .expect(
                "a 2 hour wall clock jump with only 20ms of monotonic progress must be reported",
            );

        let expected = Duration::from_secs(7200);
        let tolerance = Duration::from_secs(1);
        assert!(
            lost + tolerance >= expected && lost <= expected + tolerance,
            "expected ~{expected:?}, got {lost:?}"
        );
    }

    #[test]
    fn nothing_is_reported_when_there_is_no_real_jump() {
        struct Case {
            name: &'static str,
            mono_delta: Duration,
            wall_delta: time::Duration,
        }

        let cases = [
            Case {
                name: "clocks moving together",
                mono_delta: Duration::from_millis(20),
                wall_delta: time::Duration::milliseconds(20),
            },
            Case {
                name: "a gap one second below the threshold",
                mono_delta: Duration::ZERO,
                wall_delta: time::Duration::seconds(9),
            },
            Case {
                name: "a one-hour backwards step",
                mono_delta: Duration::from_millis(20),
                wall_delta: time::Duration::seconds(-3600),
            },
        ];

        for case in cases {
            let mut detector = JumpDetector::default();
            let mono0 = Instant::now();
            assert_eq!(
                detector.observe(mono0, wall(0)),
                None,
                "case: {}",
                case.name
            );

            let result = detector.observe(mono0 + case.mono_delta, wall(0) + case.wall_delta);
            assert_eq!(result, None, "case: {}", case.name);
        }
    }

    #[test]
    fn a_backwards_step_still_rebases_so_the_next_gap_is_measured_from_it() {
        let mut detector = JumpDetector::default();
        let mono0 = Instant::now();

        // Establish a baseline.
        assert_eq!(detector.observe(mono0, wall(0)), None);

        // The wall clock is corrected 5 minutes backwards. Nothing is reported for the
        // step itself, but it must still become the new `last` reading.
        let mono1 = mono0 + Duration::from_millis(20);
        assert_eq!(
            detector.observe(mono1, wall(0) + time::Duration::seconds(-300)),
            None
        );

        // A genuine 2-hour sleep follows immediately. If the backwards step above had
        // NOT rebased `last`, this gap would be measured against the pre-correction
        // wall reading (`wall(0)`) instead of the corrected one, and would come out
        // short by the size of that correction (300s) instead of matching the full 2
        // hours.
        let mono2 = mono1 + Duration::from_millis(20);
        let lost = detector
            .observe(mono2, wall(0) + time::Duration::seconds(-300 + 7200))
            .expect("a 2 hour jump following the correction must be reported");

        let expected = Duration::from_secs(7200);
        let tolerance = Duration::from_secs(1);
        assert!(
            lost + tolerance >= expected && lost <= expected + tolerance,
            "expected ~{expected:?} (measured from the rebased point), got {lost:?}"
        );
    }
}
