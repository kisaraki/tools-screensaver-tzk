use my_datetime_screensaver::cli::{parse, RunMode};

#[test]
fn no_arguments_opens_configuration() {
    assert_eq!(parse(&[]), Ok(RunMode::Configure(None)));
}

#[test]
fn install_helper_accepts_only_its_exact_single_flag() {
    assert_eq!(
        parse(&["--install-set-current"]),
        Ok(RunMode::InstallSetCurrent)
    );
    for args in [
        vec!["--INSTALL-SET-CURRENT"],
        vec!["-install-set-current"],
        vec!["--install-set-current", "/s"],
        vec!["--install-set-current=other"],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
}

#[test]
fn accepts_case_prefix_and_both_handle_separators() {
    for prefix in ["/", "-"] {
        for letter in ["s", "S"] {
            assert_eq!(
                parse(&[&format!("{prefix}{letter}")]),
                Ok(RunMode::Fullscreen)
            );
        }
        for letter in ["p", "P", "c", "C"] {
            let expected = if letter.eq_ignore_ascii_case("p") {
                RunMode::Preview(42)
            } else {
                RunMode::Configure(Some(42))
            };
            assert_eq!(parse(&[&format!("{prefix}{letter}:00042")]), Ok(expected));
            assert_eq!(parse(&[&format!("{prefix}{letter}"), "42"]), Ok(expected));
        }
    }
}

#[test]
fn zero_owner_is_allowed_but_zero_preview_is_not() {
    assert_eq!(parse(&["/c"]), Ok(RunMode::Configure(None)));
    assert_eq!(parse(&["/c:0"]), Ok(RunMode::Configure(None)));
    assert_eq!(parse(&["/c", "000"]), Ok(RunMode::Configure(None)));
    assert!(parse(&["/p", "0"]).is_err());
}

#[test]
fn keeps_all_64_handle_bits_and_rejects_overflow() {
    assert_eq!(parse(&["/p:4294967296"]), Ok(RunMode::Preview(4294967296)));
    assert_eq!(
        parse(&["/p", &usize::MAX.to_string()]),
        Ok(RunMode::Preview(usize::MAX))
    );
    assert!(parse(&["/p", "18446744073709551616"]).is_err());
}

#[test]
fn rejects_malformed_handles_for_both_modes() {
    for mode in ["/p", "/c"] {
        for handle in [
            "", " ", "-1", "+1", "0x20", "1.0", "1e3", "12x", "１２", "1 ", "\t1",
        ] {
            assert!(parse(&[mode, handle]).is_err(), "{mode} {handle:?}");
        }
    }
}

#[test]
fn rejects_missing_unknown_duplicate_and_debug_arguments() {
    for args in [
        vec!["/p"],
        vec!["/p:"],
        vec!["/c:"],
        vec!["/s:123"],
        vec!["/s", "/c"],
        vec!["/p:42", "43"],
        vec!["/c", "42", "extra"],
        vec!["s"],
        vec!["--s"],
        vec!["/x"],
        vec!["/"],
        vec!["--dev-render=unknown"],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
}

#[test]
fn developer_modes_are_compiled_only_into_debug() {
    for mode in ["time-date", "countdown", "japan-travel"] {
        let argument = format!("--dev-render={mode}");
        assert_eq!(parse(&[&argument]).is_ok(), cfg!(debug_assertions));
        assert!(parse(&[&argument, "/s"]).is_err());
    }
}
