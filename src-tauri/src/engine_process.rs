use std::path::Path;

use tokio::process::Command;

#[cfg(windows)]
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

pub(crate) fn engine_command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    configure_engine_environment(&mut command);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

pub(crate) fn configure_engine_environment(command: &mut Command) {
    command.env_clear().env("LC_ALL", "C").env("LANG", "C");

    #[cfg(windows)]
    for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn engine_environment_discards_unrelated_parent_values() {
        let mut command = Command::new(OsStr::new("synthetic-engine"));
        command.env("MORFLO_SYNTHETIC_SECRET", "must-not-leak");

        configure_engine_environment(&mut command);

        let configured = command
            .as_std()
            .get_envs()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().into_owned(),
                    value.map(|value| value.to_string_lossy().into_owned()),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(configured.get("LC_ALL"), Some(&Some("C".to_owned())));
        assert_eq!(configured.get("LANG"), Some(&Some("C".to_owned())));
        assert!(!configured.contains_key("MORFLO_SYNTHETIC_SECRET"));
        assert!(!configured.contains_key("PATH"));
        assert!(!configured.contains_key("HOME"));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_engine_children_run_without_a_console_window() {
        const PROBE_ENVIRONMENT: &str = "MORFLO_TEST_ENGINE_CONSOLE_WINDOW";
        const ABSENT_MARKER: &str = "MORFLO_ENGINE_CONSOLE_WINDOW=absent";

        if std::env::var_os(PROBE_ENVIRONMENT).is_some() {
            use windows_sys::Win32::System::Console::GetConsoleWindow;

            // SAFETY: GetConsoleWindow takes no arguments and only reports the
            // console associated with the current process, if one exists.
            let has_console_window = !unsafe { GetConsoleWindow() }.is_null();
            println!(
                "MORFLO_ENGINE_CONSOLE_WINDOW={}",
                if has_console_window {
                    "present"
                } else {
                    "absent"
                }
            );
            return;
        }

        let executable = std::env::current_exe().expect("resolve the current test executable");
        let mut command = engine_command(&executable);
        command
            .arg("--exact")
            .arg("engine_process::tests::windows_engine_children_run_without_a_console_window")
            .arg("--nocapture")
            .env(PROBE_ENVIRONMENT, "1");

        let output = command
            .output()
            .await
            .expect("launch the test executable through the engine process boundary");
        assert!(
            output.status.success(),
            "console probe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(ABSENT_MARKER),
            "engine child unexpectedly acquired a console window: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}
