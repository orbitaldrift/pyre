use std::fmt::{Display, Formatter};

use color_eyre::{eyre::eyre, Result};
use termion::color::{Fg, LightGreen, LightMagenta, Reset};
use vergen_git2::{BuildBuilder, Emitter, Git2Builder, RustcBuilder};

/// Emit build information to env vars.
///
/// # Errors
/// This function will return an error if any of the following fail:
/// - Building the build, rustc or git information
/// - Emitting the instructions for each
pub fn emit_build_info() -> Result<()> {
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let rustc = RustcBuilder::default()
        .host_triple(true)
        .semver(true)
        .build()?;

    let git2 = Git2Builder::all_git().map_err(|e| eyre!(e))?;

    Emitter::default()
        .add_instructions(&build)
        .map_err(|e| eyre!(e))?
        .add_instructions(&rustc)
        .map_err(|e| eyre!(e))?
        .add_instructions(&git2)
        .map_err(|e| eyre!(e))?
        .emit()
        .map_err(|e| eyre!(e))?;

    Ok(())
}

#[macro_export]
macro_rules! build_info {
    () => {
        const BUILD_INFO: $crate::BuildInfo = $crate::BuildInfo {
            crate_version: env!("CARGO_PKG_VERSION"),
            crate_name: env!("CARGO_PKG_NAME"),
            triple: env!("VERGEN_RUSTC_HOST_TRIPLE"),
            commit: env!("VERGEN_GIT_SHA"),
            rust_version: env!("VERGEN_RUSTC_SEMVER"),
            date: env!("VERGEN_BUILD_TIMESTAMP"),
        };

        println!("{}", BUILD_INFO)
    };
}

pub struct BuildInfo {
    pub crate_version: &'static str,
    pub crate_name: &'static str,
    pub triple: &'static str,
    pub commit: &'static str,
    pub rust_version: &'static str,
    pub date: &'static str,
}

impl Display for BuildInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n{}\t\t\t\t\t  {} {}{}\n{}\t\t{} . sha {} . rust {} . {}{}\n\n",
            Fg(LightGreen),
            self.crate_name,
            self.crate_version,
            Fg(Reset),
            Fg(LightMagenta),
            self.triple,
            &self.commit[..7],
            &self.rust_version,
            self.date,
            Fg(Reset),
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_build_info() {
        use super::BuildInfo;

        let build_info = BuildInfo {
            crate_version: "0.1.0",
            crate_name: "pyre-build",
            triple: "x86_64-unknown-linux-gnu",
            commit: "abcdef1",
            rust_version: "1.85.0",
            date: "2023-10-01T00:00:00Z",
        };

        println!("{build_info}");
    }
}
