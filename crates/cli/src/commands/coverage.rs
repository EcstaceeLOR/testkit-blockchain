use std::process::Command;

use clap::{Args, ValueEnum};

use super::output::{Output, Verbosity};
use super::CliError;

/// Output format for `soroban-testkit coverage`.
#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    /// Per-file coverage stats printed to the terminal.
    Text,
    /// An `lcov.info` file, for IDE integrations (e.g. Coverage Gutters).
    Lcov,
    /// A browsable HTML report.
    Html,
}

/// Arguments for `soroban-testkit coverage`.
#[derive(Args)]
#[command(after_help = "EXAMPLES:\n    \
    soroban-testkit coverage --format html --open\n    \
    soroban-testkit coverage --format lcov\n    \
    soroban-testkit coverage --fail-under 80")]
pub struct CoverageArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Fail (non-zero exit) if line coverage is below this percentage.
    #[arg(long, value_name = "PCT")]
    fail_under: Option<f64>,
    /// Open the HTML report after generating it (implies --format html).
    #[arg(long)]
    open: bool,
    /// Restrict coverage to one or more workspace packages.
    #[arg(short = 'p', long = "package", value_name = "SPEC")]
    packages: Vec<String>,
    /// Restrict coverage to named integration test targets.
    #[arg(long = "test", value_name = "NAME")]
    tests: Vec<String>,
    /// Arguments forwarded to the test binary (after `--`).
    #[arg(last = true, num_args = 0.., allow_hyphen_values = true)]
    test_args: Vec<String>,
}

/// Wraps `cargo llvm-cov test`, which handles Soroban's coverage needs
/// correctly out of the box — soroban_sdk's own test harness runs contract
/// logic natively (not through the WASM VM), so ordinary LLVM
/// source-based coverage instrumentation applies without any
/// Soroban-specific flags. This command's job is knowing *that*, and
/// knowing the right output-format flags, not reimplementing coverage
/// collection.
///
/// Requires `cargo-llvm-cov` to be installed
/// (`cargo install cargo-llvm-cov`).
pub fn run(args: CoverageArgs, out: Output) -> Result<(), CliError> {
    check_llvm_cov_version()?;
    let cargo_args = cargo_args(&args, out);
    out.detail(format!("running: cargo {}", cargo_args.join(" ")));

    let mut cmd = Command::new("cargo");
    cmd.args(&cargo_args);
    if !out.color {
        cmd.env("CARGO_TERM_COLOR", "never").env("NO_COLOR", "1");
    }

    let status = cmd.status().map_err(|err| {
        CliError(format!(
            "failed to run `cargo llvm-cov` ({err}); is cargo-llvm-cov installed? \
             try `cargo install cargo-llvm-cov`"
        ))
    })?;

    if !status.success() {
        return Err(CliError(format!(
            "coverage run failed (cargo llvm-cov exited with {status})"
        )));
    }

    if args.open {
        open_report(&std::path::PathBuf::from("target/llvm-cov/html/index.html"))?;
    }

    Ok(())
}

/// The arguments passed to `cargo` for a coverage run. `--fail-under PCT`
/// becomes cargo-llvm-cov's `--fail-under-lines PCT`, so the threshold is
/// enforced by cargo-llvm-cov itself and a shortfall surfaces as this
/// command's non-zero exit.
fn cargo_args(args: &CoverageArgs, out: Output) -> Vec<String> {
    let mut cargo_args = vec!["llvm-cov".to_string(), "test".to_string()];
    let mut push = |arg: &str| cargo_args.push(arg.to_string());

    match args.format {
        Format::Text => {}
        Format::Lcov => {
            push("--lcov");
            push("--output-path");
            push("lcov.info");
        }
        Format::Html => push("--html"),
    }

    if args.open {
        if !matches!(args.format, Format::Html) {
            push("--html");
        }
    }

    for package in &args.packages {
        push("--package");
        push(package);
    }
    for test in &args.tests {
        push("--test");
        push(test);
    }

    if let Some(pct) = args.fail_under {
        push("--fail-under-lines");
        push(&pct.to_string());
    }

    if !out.color {
        push("--color");
        push("never");
    }
    match out.verbosity {
        Verbosity::Quiet => push("--quiet"),
        Verbosity::Normal => {}
        Verbosity::Verbose => push("--verbose"),
    }

    if !args.test_args.is_empty() {
        push("--");
        for arg in &args.test_args {
            push(arg);
        }
    }

    cargo_args
}


const MINIMUM_LLVM_COV_VERSION: (u64, u64, u64) = (0, 6, 0);

/// Parse the numeric prefix of cargo-llvm-cov's version output, accepting
/// optional prerelease/build suffixes while rejecting unrelated output.
fn parse_version(output: &str) -> Option<(u64, u64, u64)> {
    let version = output.split_whitespace().find(|part| {
        part.chars().next().is_some_and(|c| c.is_ascii_digit())
    })?;
    let numbers: Vec<_> = version.split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?.split('.').collect();
    if numbers.len() < 3 {
        return None;
    }
    Some((
        numbers[0].parse().ok()?,
        numbers[1].parse().ok()?,
        numbers[2].parse().ok()?,
    ))
}

fn validate_version(output: &str) -> Result<(), CliError> {
    let found = parse_version(output).ok_or_else(|| CliError(format!(
        "cannot parse cargo-llvm-cov version from {output:?}; update with `cargo install cargo-llvm-cov --locked --force`"
    )))?;
    if found < MINIMUM_LLVM_COV_VERSION {
        return Err(CliError(format!(
            "cargo-llvm-cov {}.{}.{} is unsupported (minimum 0.6.0); update with `cargo install cargo-llvm-cov --locked --force`",
            found.0, found.1, found.2
        )));
    }
    Ok(())
}

fn check_llvm_cov_version() -> Result<(), CliError> {
    let output = Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .output()
        .map_err(|err| CliError(format!(
            "failed to inspect cargo-llvm-cov version ({err}); install it with `cargo install cargo-llvm-cov`"
        )))?;
    if !output.status.success() {
        return Err(CliError(
            "cargo llvm-cov --version failed; install or update with `cargo install cargo-llvm-cov --locked --force`".into()
        ));
    }
    validate_version(&String::from_utf8_lossy(&output.stdout))
}

/// Keep browser opening outside cargo-llvm-cov so missing platform opener
/// errors are distinguishable from instrumentation/test failures.
fn opener_command(path: &std::path::Path) -> (Command, &'static str) {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]).arg(path);
        return (command, "cmd /C start");
    }
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        command.arg(path);
        return (command, "open");
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        (command, "xdg-open")
    }
}

fn open_report(path: &std::path::Path) -> Result<(), CliError> {
    let (mut command, opener) = opener_command(path);
    let status = command.status().map_err(|err| CliError(format!(
        "HTML coverage report generated, but cannot launch {opener}: {err}; open {} manually",
        path.display()
    )))?;
    if !status.success() {
        return Err(CliError(format!(
            "HTML coverage report generated, but {opener} exited with {status}; open {} manually",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(format: Format, fail_under: Option<f64>, open: bool) -> CoverageArgs {
        CoverageArgs {
            format,
            fail_under,
            open,
            packages: Vec::new(),
            tests: Vec::new(),
            test_args: Vec::new(),
        }
    }

    fn out(verbosity: Verbosity, color: bool) -> Output {
        Output { verbosity, color }
    }

    #[test]
    fn parses_versions_and_rejects_old_tools() {
        assert_eq!(parse_version("cargo-llvm-cov 0.6.24"), Some((0, 6, 24)));
        assert_eq!(parse_version("cargo-llvm-cov 0.6.0-beta.1"), Some((0, 6, 0)));
        assert!(validate_version("cargo-llvm-cov 0.5.99").is_err());
        assert!(validate_version("not a version").is_err());
        assert!(validate_version("cargo-llvm-cov 0.6.0").is_ok());
    }

    #[test]
    fn package_test_and_harness_flags_preserve_boundaries() {
        let mut args = args(Format::Text, None, false);
        args.packages = vec!["contract".into(), "sdk".into()];
        args.tests = vec!["integration".into()];
        args.test_args = vec!["name_filter".into(), "--nocapture".into()];
        assert_eq!(cargo_args(&args, out(Verbosity::Normal, true)), [
            "llvm-cov", "test", "--package", "contract", "--package", "sdk",
            "--test", "integration", "--", "name_filter", "--nocapture"
        ]);
    }

    #[test]
    fn opener_is_platform_selected() {
        let (_, name) = opener_command(std::path::Path::new("report.html"));
        #[cfg(target_os = "windows")]
        assert_eq!(name, "cmd /C start");
        #[cfg(target_os = "macos")]
        assert_eq!(name, "open");
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        assert_eq!(name, "xdg-open");
    }

    #[test]
    fn fail_under_is_forwarded_as_fail_under_lines() {
        let cargo = cargo_args(
            &args(Format::Text, Some(80.0), false),
            out(Verbosity::Normal, true),
        );
        assert_eq!(cargo, ["llvm-cov", "test", "--fail-under-lines", "80"]);

        let cargo = cargo_args(
            &args(Format::Lcov, Some(92.5), false),
            out(Verbosity::Normal, true),
        );
        assert!(
            cargo.ends_with(&["--fail-under-lines".to_string(), "92.5".to_string()]),
            "{cargo:?}"
        );
    }

    #[test]
    fn no_threshold_means_no_fail_under_flag() {
        let cargo = cargo_args(
            &args(Format::Html, None, true),
            out(Verbosity::Normal, true),
        );
        assert_eq!(cargo, ["llvm-cov", "test", "--html"]);
    }

    #[test]
    fn color_and_verbosity_are_forwarded_to_cargo_llvm_cov() {
        let quiet = cargo_args(
            &args(Format::Text, None, false),
            out(Verbosity::Quiet, false),
        );
        assert_eq!(quiet, ["llvm-cov", "test", "--color", "never", "--quiet"]);

        let verbose = cargo_args(
            &args(Format::Text, None, false),
            out(Verbosity::Verbose, true),
        );
        assert_eq!(verbose, ["llvm-cov", "test", "--verbose"]);
    }
}
