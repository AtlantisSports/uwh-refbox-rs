use super::*;
use uwh_common::{bundles::BlackWhiteBundle, game_snapshot::GameSnapshotNoHeap};

fn empty_data() -> TransmittedData {
    TransmittedData {
        snapshot: GameSnapshotNoHeap {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: 0,
            timeout: None,
            scores: BlackWhiteBundle { black: 0, white: 0 },
            penalties: Default::default(),
            is_old_game: false,
        },
        white_on_right: false,
        flash: false,
        beep_test: false,
        brightness: Brightness::Low,
    }
}

const EMPTY_STATE: DisplayState = DisplayState {
    left_score_ones: Digit::ZERO,
    left_score_tens: Digit::EMPTY,
    right_score_ones: Digit::ZERO,
    right_score_tens: Digit::EMPTY,
    time_m_ones: Digit::ZERO,
    time_m_tens: Digit::EMPTY,
    time_s_ones: Digit::ZERO,
    time_s_tens: Digit::ZERO,
    white_on_left: true,
    white_on_right: false,
    left_to_ind: false,
    right_to_ind: false,
    ref_to_ind: false,
    one: false,
    slash: false,
    two: false,
    overtime: false,
    sdn_dth: false,
    colon: true,
};

#[test]
fn test_digit_from_num() {
    assert_eq!(
        Digit::from_num(0),
        Digit {
            a: true,
            b: true,
            c: true,
            d: true,
            e: true,
            f: true,
            g: false,
        }
    );
    assert_eq!(
        Digit::from_num(1),
        Digit {
            a: false,
            b: true,
            c: true,
            d: false,
            e: false,
            f: false,
            g: false,
        }
    );
    assert_eq!(
        Digit::from_num(2),
        Digit {
            a: true,
            b: true,
            c: false,
            d: true,
            e: true,
            f: false,
            g: true,
        }
    );
    assert_eq!(
        Digit::from_num(3),
        Digit {
            a: true,
            b: true,
            c: true,
            d: true,
            e: false,
            f: false,
            g: true,
        }
    );
    assert_eq!(
        Digit::from_num(4),
        Digit {
            a: false,
            b: true,
            c: true,
            d: false,
            e: false,
            f: true,
            g: true,
        }
    );
    assert_eq!(
        Digit::from_num(5),
        Digit {
            a: true,
            b: false,
            c: true,
            d: true,
            e: false,
            f: true,
            g: true,
        }
    );
    assert_eq!(
        Digit::from_num(6),
        Digit {
            a: true,
            b: false,
            c: true,
            d: true,
            e: true,
            f: true,
            g: true,
        }
    );
    assert_eq!(
        Digit::from_num(7),
        Digit {
            a: true,
            b: true,
            c: true,
            d: false,
            e: false,
            f: false,
            g: false,
        }
    );
    assert_eq!(
        Digit::from_num(8),
        Digit {
            a: true,
            b: true,
            c: true,
            d: true,
            e: true,
            f: true,
            g: true,
        }
    );
    assert_eq!(
        Digit::from_num(9),
        Digit {
            a: true,
            b: true,
            c: true,
            d: true,
            e: false,
            f: true,
            g: true,
        }
    );
    assert_eq!(
        Digit::from_num(10),
        Digit {
            a: false,
            b: false,
            c: false,
            d: false,
            e: false,
            f: false,
            g: true,
        }
    );
}

#[test]
fn test_digit_pair_from_num() {
    assert_eq!(Digit::pair_from_num(0, false), (Digit::EMPTY, Digit::ZERO,));
    assert_eq!(Digit::pair_from_num(0, true), (Digit::ZERO, Digit::ZERO,));
    assert_eq!(Digit::pair_from_num(1, false), (Digit::EMPTY, Digit::ONE,));
    assert_eq!(Digit::pair_from_num(12, false), (Digit::ONE, Digit::TWO,));
    assert_eq!(Digit::pair_from_num(99, false), (Digit::NINE, Digit::NINE,));
    assert_eq!(
        Digit::pair_from_num(100, false),
        (Digit::ERROR, Digit::ERROR,)
    );
}

#[test]
fn test_sores() {
    let mut data = empty_data();

    let (state, brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(state, EMPTY_STATE);
    assert_eq!(brightness, Brightness::Low);

    data.snapshot.scores.black = 1;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            right_score_ones: Digit::ONE,
            right_score_tens: Digit::EMPTY,
            ..EMPTY_STATE
        }
    );

    data.snapshot.scores.black = 10;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            right_score_ones: Digit::ZERO,
            right_score_tens: Digit::ONE,
            ..EMPTY_STATE
        }
    );

    data.snapshot.scores.white = 99;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            right_score_ones: Digit::ZERO,
            right_score_tens: Digit::ONE,
            left_score_ones: Digit::NINE,
            left_score_tens: Digit::NINE,
            ..EMPTY_STATE
        }
    );
}

#[test]
fn test_time() {
    let mut data = empty_data();

    let (state, brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(state, EMPTY_STATE);
    assert_eq!(brightness, Brightness::Low);

    data.snapshot.secs_in_period = 1;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_s_ones: Digit::ONE,
            ..EMPTY_STATE
        }
    );

    data.snapshot.secs_in_period = 10;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_s_ones: Digit::ZERO,
            time_s_tens: Digit::ONE,
            ..EMPTY_STATE
        }
    );

    data.snapshot.secs_in_period = 60;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::ONE,
            ..EMPTY_STATE
        }
    );

    data.snapshot.secs_in_period = 99 * 60 + 59;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::NINE,
            time_m_tens: Digit::NINE,
            time_s_ones: Digit::NINE,
            time_s_tens: Digit::FIVE,
            ..EMPTY_STATE
        }
    );

    data.snapshot.secs_in_period = 100 * 60;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::ERROR,
            time_m_tens: Digit::ERROR,
            time_s_ones: Digit::ERROR,
            time_s_tens: Digit::ERROR,
            ..EMPTY_STATE
        }
    );
}

#[test]
fn test_timeouts() {
    let mut data = empty_data();

    let (state, brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(state, EMPTY_STATE);
    assert_eq!(brightness, Brightness::Low);

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(0));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(1));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_ones: Digit::ONE,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(15));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::ONE,
            time_s_ones: Digit::FIVE,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(16));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::ONE,
            time_s_ones: Digit::SIX,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(30));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::THREE,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(31));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::THREE,
            time_s_ones: Digit::ONE,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(45));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::FOUR,
            time_s_ones: Digit::FIVE,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(46));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::FOUR,
            time_s_ones: Digit::SIX,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(60));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::SIX,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(61));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::SIX,
            time_s_ones: Digit::ONE,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(99));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_ones: Digit::NINE,
            time_s_tens: Digit::NINE,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Black(100));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_ones: Digit::NINE,
            time_s_tens: Digit::NINE,
            right_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::White(10));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::ONE,
            left_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::White(22));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::TWO,
            time_s_ones: Digit::TWO,
            left_to_ind: true,
            ..EMPTY_STATE
        }
    );

    data.white_on_right = true;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            time_m_ones: Digit::EMPTY,
            time_s_tens: Digit::TWO,
            time_s_ones: Digit::TWO,
            left_to_ind: false,
            right_to_ind: true,
            white_on_left: false,
            white_on_right: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::Ref(24));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            ref_to_ind: true,
            white_on_left: false,
            white_on_right: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.timeout = Some(TimeoutSnapshot::PenaltyShot(78));
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            ref_to_ind: true,
            white_on_left: false,
            white_on_right: true,
            ..EMPTY_STATE
        }
    );
}

#[test]
fn test_game_periods() {
    let mut data = empty_data();

    let (state, brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(state, EMPTY_STATE);
    assert_eq!(brightness, Brightness::Low);

    data.snapshot.current_period = GamePeriod::FirstHalf;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            one: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.current_period = GamePeriod::HalfTime;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            one: true,
            slash: true,
            two: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.current_period = GamePeriod::SecondHalf;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            two: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.current_period = GamePeriod::PreOvertime;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            overtime: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.current_period = GamePeriod::OvertimeFirstHalf;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            one: true,
            overtime: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.current_period = GamePeriod::OvertimeHalfTime;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            one: true,
            slash: true,
            two: true,
            overtime: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.current_period = GamePeriod::OvertimeSecondHalf;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            two: true,
            overtime: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.current_period = GamePeriod::PreSuddenDeath;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            overtime: true,
            sdn_dth: true,
            ..EMPTY_STATE
        }
    );

    data.snapshot.current_period = GamePeriod::SuddenDeath;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            overtime: true,
            sdn_dth: true,
            ..EMPTY_STATE
        }
    );
}

#[test]
fn test_flash() {
    let mut data = empty_data();

    let (state, brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(state, EMPTY_STATE);
    assert_eq!(brightness, Brightness::Low);

    data.flash = true;
    let (state, _brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(
        state,
        DisplayState {
            left_score_ones: Digit::EIGHT,
            left_score_tens: Digit::EIGHT,
            right_score_ones: Digit::EIGHT,
            right_score_tens: Digit::EIGHT,
            time_m_ones: Digit::EIGHT,
            time_m_tens: Digit::EIGHT,
            time_s_ones: Digit::EIGHT,
            time_s_tens: Digit::EIGHT,
            white_on_left: true,
            white_on_right: true,
            left_to_ind: true,
            right_to_ind: true,
            ref_to_ind: true,
            one: true,
            slash: true,
            two: true,
            overtime: true,
            sdn_dth: true,
            colon: true,
        }
    );
}

#[test]
fn test_brightness() {
    let mut data = empty_data();

    let (state, brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(state, EMPTY_STATE);
    assert_eq!(brightness, Brightness::Low);

    data.brightness = Brightness::Medium;
    let (state, brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(state, EMPTY_STATE);
    assert_eq!(brightness, Brightness::Medium);

    data.brightness = Brightness::High;
    let (state, brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(state, EMPTY_STATE);
    assert_eq!(brightness, Brightness::High);

    data.brightness = Brightness::Outdoor;
    let (state, brightness) = DisplayState::from_transmitted_data(&data);
    assert_eq!(state, EMPTY_STATE);
    assert_eq!(brightness, Brightness::Outdoor);
}
