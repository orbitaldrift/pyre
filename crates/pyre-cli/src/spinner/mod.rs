use std::time::Duration;

use indicatif::ProgressBar;
use pyre_telemetry::suspendable::Suspendable;
use termion::color;

const TICK_STRINGS: [&str; 12] = ["π", "∫", "∑", "∆", "∇", "π", "∂", "∏", "∞", "√", "𝜆", "𝛾"];

#[derive(Clone)]
pub enum SpinnerTemplate {
    Default,
    Progress,
}

#[derive(Debug, Clone)]
pub struct Spinner {
    pub inner: ProgressBar,
    current_step: u8,
    total_steps: u8,
}

impl Spinner {
    const DEFAULT_TEMPLATE: &'static str =
        "{prefix} {spinner:.magenta} {elapsed:.yellow} {msg:.magenta}";
    const PROGRESS_TEMPLATE: &'static str =
        "{bar:.magenta/blue} {bytes}/{total_bytes} {eta:.yellow}";

    /// Create a new spinner with the given total steps and message.
    /// The spinner will be styled according to the given template.
    ///
    /// # Panics
    /// This function will panic if the template is invalid.
    #[must_use]
    pub fn new(total_steps: u8, message: &str, template: &SpinnerTemplate) -> Self {
        let tpl = match template {
            SpinnerTemplate::Default => Self::DEFAULT_TEMPLATE.to_string(),
            SpinnerTemplate::Progress => {
                format!("{} {}", Self::DEFAULT_TEMPLATE, Self::PROGRESS_TEMPLATE)
            }
        };

        let spinner = ProgressBar::new_spinner().with_prefix(format!("[{}/{}]", 1, total_steps));
        spinner.set_style(
            indicatif::ProgressStyle::with_template(tpl.as_str())
                .expect("progress bar template is invalid")
                .tick_strings(&TICK_STRINGS),
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(100));

        Spinner {
            inner: spinner,
            current_step: 1,
            total_steps,
        }
    }

    pub fn next_step(&mut self, message: &str) {
        // Leave the last step in the log.
        // indicatif sadly doesn't have anything nice to make that happen,
        // so we trick it into deleting some empty space instead of the last step
        let msglen = self.inner.message().len() + 1;
        let spaces = " ".repeat(msglen);
        println!("{spaces}");

        self.current_step += 1;

        self.inner
            .set_prefix(format!("[{}/{}]", self.current_step, self.total_steps));
        self.inner.set_message(message.to_string());
    }

    pub fn success(&self, message: &str) {
        self.inner.finish_with_message(format!(
            "{}✓ {}{}",
            color::Fg(color::Green),
            message,
            color::Fg(color::Reset)
        ));
    }

    pub fn fail(&self, message: &str) {
        self.inner.finish_with_message(format!(
            "{}✕ {}{}",
            color::Fg(color::Red),
            message,
            color::Fg(color::Reset)
        ));
    }
}

impl Suspendable for Spinner {
    fn suspend<F: FnOnce() -> R, R>(&self, f: F) -> R {
        self.inner.suspend(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner() {
        let mut spinner = Spinner::new(10, "Loading...", &SpinnerTemplate::Default);
        assert_eq!(spinner.current_step, 1);
        assert_eq!(spinner.total_steps, 10);

        spinner.next_step("Step 2");
        assert_eq!(spinner.current_step, 2);

        spinner.success("Done");
        assert!(spinner.inner.is_finished());

        let spinner = Spinner::new(10, "Loading...", &SpinnerTemplate::Default);
        spinner.fail("Failed");
        assert!(spinner.inner.is_finished());
    }
}
