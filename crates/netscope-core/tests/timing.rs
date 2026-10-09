//! Regression checks for the timing distinctions in the M1 contract.
use netscope_core::{InvalidTiming, Timing};

#[test]
fn absence_unavailability_and_zero_remain_distinct() {
    assert_eq!(Timing::from_optional_har(None), Ok(Timing::Missing));
    assert_eq!(
        Timing::from_optional_har(Some(-1.0)),
        Ok(Timing::Unavailable)
    );
    assert_eq!(
        Timing::from_optional_har(Some(0.0)),
        Ok(Timing::Milliseconds(0.0))
    );
}

#[test]
fn preserves_valid_durations_without_rounding() {
    for duration in [0.001, 20.0, 30.125, f64::MAX] {
        assert_eq!(
            Timing::from_optional_har(Some(duration)),
            Ok(Timing::Milliseconds(duration))
        );
    }
}

#[test]
fn rejects_invalid_numbers_without_panicking() {
    for duration in [-2.0, -0.001, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            Timing::from_optional_har(Some(duration)),
            Err(InvalidTiming)
        );
    }
}
