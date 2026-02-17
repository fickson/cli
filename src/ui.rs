use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::time::Duration;

pub fn spinner_enabled(format: Option<&str>) -> bool {
    if let Some(fmt) = format {
        if fmt.eq_ignore_ascii_case("json") {
            return false;
        }
    }
    std::io::stdout().is_terminal()
}

pub struct SpinnerGuard {
    pb: Option<ProgressBar>,
}

impl SpinnerGuard {
    pub fn new(enabled: bool, msg: &str) -> Self {
        if !enabled {
            return Self { pb: None };
        }

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(Duration::from_millis(100));
        Self { pb: Some(pb) }
    }
}

impl Drop for SpinnerGuard {
    fn drop(&mut self) {
        if let Some(pb) = &self.pb {
            pb.finish_and_clear();
        }
    }
}

pub async fn with_spinner_if<T, F>(enabled: bool, msg: &str, fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let _guard = SpinnerGuard::new(enabled, msg);
    fut.await
}
