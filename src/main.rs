#![windows_subsystem = "windows"]

use std::process::ExitCode;

use tools_screensaver_tzk::app;

fn main() -> ExitCode {
    match app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if cfg!(debug_assertions) {
                app::report_diagnostic(&error.to_string());
            }
            ExitCode::from(error.exit_code())
        }
    }
}
