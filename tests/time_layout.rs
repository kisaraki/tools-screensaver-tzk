use tools_screensaver_tzk::{
    layout::{buffer_bytes, offset_range, Drift, Layout},
    model::*,
};

#[test]
fn gregorian_centuries_and_monday_first_cells() {
    for (year, days) in [(2024, 29), (1900, 28), (2000, 29)] {
        assert_eq!(days_in_month(year, 2), days);
    }
    let feb = Calendar::new(2021, 2).unwrap();
    let oct = Calendar::new(2023, 10).unwrap();
    assert_eq!(feb.monday_offset, 0);
    assert_eq!(oct.monday_offset, 6);
    assert_eq!(feb.cell(28), Some((3, 6)));
    assert_eq!(oct.cell(31), Some((5, 1)));
    assert_eq!(Calendar::new(2023, 12).unwrap().cell(31), Some((4, 6)));
    assert!(Calendar::new(0, 1).is_none());
    assert!(Calendar::new(2024, 13).is_none());
    assert!(feb.cell(0).is_none());
    assert!(feb.cell(29).is_none());
    let layout = Layout::new(800, 369, DisplayMode::TimeDate).unwrap();
    assert_eq!(
        layout.calendar.h * 0.75 / 6.0 * 6.0,
        layout.calendar.h * 0.75
    );
}

#[test]
fn clock_angles_are_continuous_in_minutes_and_seconds() {
    for (hour, minute, expected) in [
        (0, 0, [0.0, 0.0, 0.0]),
        (3, 0, [90.0, 0.0, 0.0]),
        (12, 30, [15.0, 180.0, 0.0]),
    ] {
        assert_eq!(
            LocalTime {
                hour,
                minute,
                second: 0,
                ..LocalTime::FIXTURE
            }
            .hand_angles(),
            expected
        );
    }
    let angles = LocalTime::FIXTURE.hand_angles();
    assert!((angles[0] - 7.833333333).abs() < 0.000001);
    assert_eq!(angles[1], 94.0);
    assert_eq!(angles[2], 240.0);
}

#[test]
fn duration_fields_reject_bad_input_and_round_trip_extremes() {
    for (h, m, s, error) in [
        ("00", "00", "00", InputError::Zero),
        ("", "00", "01", InputError::Hours),
        ("1a", "00", "01", InputError::Hours),
        ("１", "00", "01", InputError::Hours),
        ("00", "60", "01", InputError::Minutes),
        ("00", "00", "60", InputError::Seconds),
        ("100", "00", "01", InputError::Hours),
        (" 1", "00", "01", InputError::Hours),
    ] {
        assert_eq!(parse_duration(h, m, s), Err(error));
    }
    for (h, m, s, total) in [
        ("00", "00", "01", 1),
        ("00", "59", "59", 3599),
        ("99", "59", "59", 359999),
    ] {
        assert_eq!(parse_duration(h, m, s), Ok(total));
        let [h, m, s] = hms(total);
        assert_eq!(h * 3600 + m * 60 + s, total);
    }
}

#[test]
fn countdown_ceil_ratio_and_deadline_overflow() {
    let countdown = Countdown::new(5, 100).unwrap();
    for (elapsed, seconds) in [(0, 5), (1, 5), (1000, 4), (4999, 1), (5000, 0), (9000, 0)] {
        let frame = countdown.frame(100 + elapsed);
        assert_eq!(frame.display_seconds, seconds);
        assert!((0.0..=1.0).contains(&frame.ratio));
    }
    assert_eq!(countdown.frame(0).ratio, 1.0);
    assert_eq!(Countdown::new(0, 0).unwrap_err(), InputError::Zero);
    assert_eq!(
        Countdown::new(359999, u64::MAX).unwrap_err(),
        InputError::Overflow
    );
    assert_eq!(Countdown::new(360000, 0).unwrap_err(), InputError::Overflow);
}

#[test]
fn last_ten_seconds_and_exactly_four_dim_half_cycles() {
    let countdown = Countdown::new(20, 0).unwrap();
    for (remaining, expected) in [(10001, false), (10000, true), (1, true), (0, false)] {
        assert_eq!(countdown.frame(20000 - remaining).final_ten, expected);
    }
    for (elapsed, dim, animating) in [
        (419, false, true),
        (420, true, true),
        (3359, true, true),
        (3360, false, false),
    ] {
        let frame = countdown.frame(20000 + elapsed);
        assert_eq!(frame.dim, dim);
        assert_eq!(frame.animating, animating);
    }
    assert_eq!(
        (0..8)
            .filter(|phase| countdown.frame(20000 + phase * 420).dim)
            .count(),
        4
    );
}

#[test]
fn shared_generation_clock_adjustment_and_resume_do_not_change_deadline() {
    let mut timeline = Timeline::new(DisplayMode::Countdown, 30, 0).unwrap();
    let a = timeline.sample(LocalTime::FIXTURE, 10000);
    let b = timeline.sample(
        LocalTime {
            day: 30,
            ..LocalTime::FIXTURE
        },
        10000,
    );
    assert_eq!(a.countdown, b.countdown);
    assert_ne!(a.local.day, b.local.day);
    let monitor_copies = [b; 3];
    assert!(monitor_copies
        .iter()
        .all(|copy| copy.generation == b.generation && copy.countdown == b.countdown));
    assert_eq!(timeline.interval(b), 100);
    let resumed = timeline.sample(LocalTime::FIXTURE, 86400000);
    assert_eq!(resumed.countdown.display_seconds, 0);
    assert!(!resumed.countdown.dim);
    assert_eq!(timeline.interval(resumed), 1000);
    let time = Timeline::new(DisplayMode::TimeDate, 300, 0).unwrap();
    assert_eq!(time.interval(a), 1000);
}

#[test]
fn segment_mapping_and_seeded_random_sequence() {
    assert_eq!(
        SEGMENTS,
        [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f]
    );
    let mut random = Xorshift32::new(1);
    assert_eq!(
        [
            random.next_value(),
            random.next_value(),
            random.next_value()
        ],
        [270369, 67634689, 2647435461]
    );
    let mut zero = Xorshift32::new(0);
    for _ in 0..50 {
        assert_ne!(zero.next_value(), 0);
    }
}

#[test]
fn layouts_fit_all_aspects_and_small_previews_without_double_dpi_scaling() {
    for mode in [
        DisplayMode::TimeDate,
        DisplayMode::Countdown,
        DisplayMode::JapanTravel,
    ] {
        for (width, height) in [
            (1920, 1080),
            (1920, 1200),
            (1024, 768),
            (1080, 1920),
            (3440, 1440),
            (1349, 1000),
            (1350, 1000),
            (320, 180),
            (120, 80),
            (1, 1),
        ] {
            for _dpi in [96, 144, 192, 288] {
                let layout = Layout::new(width, height, mode).unwrap();
                assert_eq!(
                    layout.horizontal,
                    f64::from(width) / f64::from(height) >= 1.35
                );
                for rect in [
                    layout.clock,
                    layout.calendar,
                    layout.panel,
                    layout.inner,
                    layout.hourglass,
                ] {
                    if rect.w == 0.0 {
                        continue;
                    }
                    assert!(
                        rect.x + 1e-8 >= layout.group.x && rect.y + 1e-8 >= layout.group.y,
                        "{mode:?} {width}x{height}: {rect:?} starts outside {:?}",
                        layout.group
                    );
                    assert!(
                        rect.right() <= layout.group.right() + 1e-8
                            && rect.bottom() <= layout.group.bottom() + 1e-8,
                        "{mode:?} {width}x{height}: {rect:?} ends outside {:?}",
                        layout.group
                    );
                    assert!(rect.w > 0.0 && rect.h > 0.0);
                }
            }
        }
        assert!(Layout::new(0, 0, mode).is_none());
        assert!(Layout::new(-1, 80, mode).is_none());
    }
    assert_eq!(buffer_bytes(0, 0), Some(0));
    assert_eq!(buffer_bytes(3840, 2160), Some(33177600));
    assert!(buffer_bytes(i32::MAX, i32::MAX).is_none());
    assert!(buffer_bytes(-1, 20).is_none());
}

#[test]
fn clock_and_countdown_use_a_centered_friendly_desktop_stage() {
    for mode in [DisplayMode::TimeDate, DisplayMode::Countdown] {
        for (width, height) in [(800, 369), (1920, 1080), (3840, 2160), (1080, 1920)] {
            let layout = Layout::new(width, height, mode).unwrap();
            let (w, h) = (f64::from(width), f64::from(height));
            assert!((layout.group.x - w * 0.18).abs() < 1e-8);
            assert!((layout.group.y - h * 0.20).abs() < 1e-8);
            assert!((layout.group.w - w * 0.64).abs() < 1e-8);
            assert!((layout.group.h - h * 0.60).abs() < 1e-8);
            assert!((layout.group.cx() - w / 2.0).abs() < 1e-8);
            assert!((layout.group.cy() - h / 2.0).abs() < 1e-8);
        }

        let preview = Layout::new(320, 180, mode).unwrap();
        assert!((preview.group.w - 320.0 * 0.88).abs() < 1e-8);
        assert!((preview.group.h - 180.0 * 0.82).abs() < 1e-8);
    }

    let travel = Layout::new(1920, 1080, DisplayMode::JapanTravel).unwrap();
    assert!((travel.group.w - 1920.0 * 0.88).abs() < 1e-8);
    assert!((travel.group.h - 1080.0 * 0.82).abs() < 1e-8);
}

#[test]
fn drift_stays_inside_safety_bounds_and_skips_missed_intervals() {
    assert_eq!(offset_range(800, 0.0, 800.0), (0, 0));
    assert_eq!(offset_range(1, 0.0, 1.0), (0, 0));
    let layout = Layout::new(800, 369, DisplayMode::TimeDate).unwrap();
    let mut a = Drift::new(42, 0);
    let mut b = a;
    a.update(59999, 800, 369, layout.group);
    assert_eq!(a.offset, (0, 0));
    a.update(600000, 800, 369, layout.group);
    b.update(60000, 800, 369, layout.group);
    assert_eq!(
        a.offset, b.offset,
        "missed intervals must consume only one anchor"
    );
    for i in 1..100 {
        a.update(600000 + i * 60000, 800, 369, layout.group);
        assert!(f64::from(a.offset.0).abs() <= 40.0 && f64::from(a.offset.1).abs() <= 18.45);
        assert!(layout.group.x + f64::from(a.offset.0) >= 16.0);
        assert!(layout.group.right() + f64::from(a.offset.0) <= 784.0);
        assert!(layout.group.y + f64::from(a.offset.1) >= 369.0 * 0.02);
        assert!(layout.group.bottom() + f64::from(a.offset.1) <= 369.0 * 0.98);
    }
}
