use ansi_term::{Color, Style};
use anyhow::{Error, Result};

use std::fmt;
use std::path::PathBuf;

use crate::utils::ansi_link;

const DOC_SITE_HOST: &str = "https://ast-grep.github.io";
const PATTERN_GUIDE: Option<&str> = Some("/guide/pattern-syntax.html");
const CONFIG_GUIDE: Option<&str> = Some("/guide/rule-config.html");
const CLI_USAGE: Option<&str> = Some("/reference/cli.html");
const TEST_GUIDE: Option<&str> = Some("/guide/test-rule.html");
const UTIL_GUIDE: Option<&str> = Some("/guide/rule-config/utility-rule.html");
const EDITOR_INTEGRATION: Option<&str> = Some("/guide/editor-integration.html");
const PLAYGROUND: Option<&str> = Some("/playground.html");

/// AppError stands for ast-grep command line usage.
/// It provides abstraction around exit code, context,
/// message, potential fix and reference link.
#[derive(Debug, Clone)]
pub enum ErrorContext {
  // Config
  ReadConfiguration,
  ParseConfiguration,
  WalkRuleDir(PathBuf),
  ReadRule(PathBuf),
  ParseRule(PathBuf),
  ParseTest(PathBuf),
  GlobPattern,
  // Run
  ParsePattern,
  // Scan
  DiagnosticError(usize),
  // LSP
  StartLanguageServer,
  // Edit
  OpenEditor,
  WriteFile(PathBuf),
  // Test
  TestFail(String),
  // New
  ProjectAlreadyExist,
  ProjectNotExist,
  FileAlreadyExist(PathBuf),
  NoTestDirConfigured,
  NoUtilDirConfigured,
  InsufficientCLIArgument(&'static str),
}

impl ErrorContext {
  fn exit_code(&self) -> i32 {
    use ErrorContext::*;
    // reference: https://mariadb.com/kb/en/operating-system-error-codes/
    match self {
      DiagnosticError(_) => 1,
      ProjectNotExist => 2,
      TestFail(_) => 3,
      NoTestDirConfigured | NoUtilDirConfigured => 4,
      ReadConfiguration | ReadRule(_) | WalkRuleDir(_) | WriteFile(_) => 5,
      ParseTest(_) | ParseRule(_) | ParseConfiguration | GlobPattern | ParsePattern => 8,
      ProjectAlreadyExist | FileAlreadyExist(_) => 17,
      InsufficientCLIArgument(_) => 22,
      OpenEditor | StartLanguageServer => 126,
    }
  }
}

impl fmt::Display for ErrorContext {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let err_msg = ErrorMessage::from_context(self);
    write!(f, "{}", err_msg.title)
  }
}

// guideline: https://twitter.com/mgechev/status/1405019854906834944
// * Use consistent terminology
// * Be clear and concise
// * Provide context
// * Suggest a fix
struct ErrorMessage {
  title: String,
  description: String,
  link: Option<&'static str>,
}

impl ErrorMessage {
  fn new<T: ToString, D: ToString>(title: T, description: D, link: Option<&'static str>) -> Self {
    Self {
      title: title.to_string(),
      description: description.to_string(),
      link,
    }
  }

  fn from_context(ctx: &ErrorContext) -> ErrorMessage {
    use ErrorContext::*;
    match ctx {
      ReadConfiguration => Self::new(
        "Cannot read configuration.",
        "Please add an sgconfig.yml configuration file in the project root to run the scan command.",
        CONFIG_GUIDE,
      ),
      ParseConfiguration => Self::new(
        "Cannot parse configuration",
        "The sgconfig.yml is not a valid configuration file. Please refer to doc and fix the error.",
        CONFIG_GUIDE,
      ),
      WalkRuleDir(dir) => Self::new(
        format!("Cannot read rule directory {}", dir.display()),
        "The rule directory cannot be read or traversed",
        None,
      ),
      ReadRule(file) => Self::new(
        format!("Cannot read rule {}", file.display()),
        "The rule file either does not exist or cannot be opened.",
        CONFIG_GUIDE,
      ),
      ParseRule(file) => Self::new(
        format!("Cannot parse rule {}", file.display()),
        "The file is not a valid ast-grep rule. Please refer to doc and fix the error.",
        CONFIG_GUIDE,
      ),
      GlobPattern => Self::new(
        "Cannot parse glob pattern in config",
        "The pattern in files/ignore is not a valid glob. Please refer to doc and fix the error.",
        CONFIG_GUIDE,
      ),
      ParseTest(file) => Self::new(
        format!("Cannot parse test case {}", file.display()),
        "The file is not a valid ast-grep test case. Please refer to doc and fix the error.",
        TEST_GUIDE,
      ),
      DiagnosticError(num) => Self::new(
        format!("{num} error(s) found in code."),
        "Scan succeeded and found error level diagnostics in the codebase.",
        None,
      ),
      ParsePattern => Self::new(
        "Cannot parse query as a valid pattern",
        "The pattern either fails to parse or contains error. Please refer to pattern syntax guide.",
        PATTERN_GUIDE,
      ),
      StartLanguageServer => Self::new(
        "Cannot start language server.",
        "Please see language server logging file.",
        EDITOR_INTEGRATION,
      ),
      OpenEditor => Self::new(
        "Cannot open file in editor.",
        "Please check if the editor is installed and the EDITOR environment variable is correctly set.",
        CLI_USAGE,
      ),
      WriteFile(file) => Self::new(
        format!("Cannot rewrite file {}", file.display()),
        "Fail to apply fix to the file. Skip to next file",
        None,
      ),
      TestFail(message) => Self::new(
        message,
        "You can use ast-grep playground to debug your rules and test cases.",
        PLAYGROUND,
      ),
      ProjectAlreadyExist => Self::new(
        "ast-grep project already exists.",
        "You are already inside a sub-folder of an ast-grep project. Try finding sgconfig.yml in ancestor directory?",
        CONFIG_GUIDE,
      ),
      ProjectNotExist => Self::new(
        "Fail to create the item because no project configuration is found.",
        "You need to create an ast-grep project before creating rule. Try `sg new` to create one.",
        CONFIG_GUIDE,
      ),
      FileAlreadyExist(path) => Self::new(
        format!("File `{}` already exists.", path.display()),
        "The item you want to create already exists. Try editing the existing file or create a new one with a different name?",
        None,
      ),
      NoTestDirConfigured => Self::new(
        "No test file directory is configured.",
        "Fail to create a test file because the project `sgconfig.yml` does not specify any test configuration.",
        TEST_GUIDE,
      ),
      NoUtilDirConfigured => Self::new(
        "No util file directory is configured.",
        "Fail to create a utility rule because the project `sgconfig.yml` does not specify any utils directory.",
        UTIL_GUIDE,
      ),
      InsufficientCLIArgument(name) => Self::new(
        "Insufficient command line argument provided to use `--yes` option.",
        format!("You need to provide `{name}` in command line to use non-interactive `new`."),
        None,
      ),
    }
  }
}

pub fn exit_with_error(error: Error) -> Result<()> {
  if let Some(e) = error.downcast_ref::<clap::Error>() {
    e.exit()
  }
  if let Some(e) = error.downcast_ref::<ErrorContext>() {
    let error_fmt = ErrorFormat {
      context: e,
      inner: &error,
    };
    eprintln!("{error_fmt}");
    std::process::exit(e.exit_code())
  }
  // use anyhow's default error reporting
  Err(error)
}

struct ErrorFormat<'a> {
  context: &'a ErrorContext,
  inner: &'a Error,
}

impl<'a> fmt::Display for ErrorFormat<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let ErrorMessage {
      title,
      description,
      link,
    } = ErrorMessage::from_context(self.context);
    let bold = Style::new().bold();
    let error = Color::Red.paint("Error:");
    let message = bold.paint(title);
    writeln!(f, "{error} {message}")?;
    let help = Color::Blue.paint("Help:");
    writeln!(f, "{help} {description}")?;
    if let Some(url) = link {
      let reference = Style::new().bold().dimmed().paint("See also:");
      let link = ansi_link(format!("{DOC_SITE_HOST}{url}"));
      writeln!(f, "{reference} {link}")?;
    }

    // skip root error
    let mut causes = self.inner.chain().skip(1).peekable();
    if causes.peek().is_none() {
      return Ok(());
    }
    writeln!(f)?;
    writeln!(f, "{} Caused by", Color::Red.paint("×"))?;
    for err in causes {
      let prefix = Color::Red.paint("╰▻");
      writeln!(f, "{prefix} {err}")?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_display_error() {
    let error = anyhow::anyhow!("test error").context(ErrorContext::ReadConfiguration);
    let error_fmt = ErrorFormat {
      context: &ErrorContext::ReadConfiguration,
      inner: &error,
    };
    let display = format!("{error_fmt}");
    assert_eq!(display.lines().count(), 6);
    assert!(display.contains("Cannot read configuration."));
    assert!(
      display.contains("Caused by"),
      "Should display the error chain"
    );
    assert!(display.contains("test error"));
  }

  #[test]
  fn test_bare_anyhow() {
    let error = anyhow::anyhow!(ErrorContext::ReadConfiguration);
    let error_fmt = ErrorFormat {
      context: &ErrorContext::ReadConfiguration,
      inner: &error,
    };
    let display = format!("{error_fmt}");
    assert_eq!(display.lines().count(), 3);
    assert!(display.contains("Cannot read configuration."));
    assert!(
      !display.contains("Caused by"),
      "Should not contain error chain"
    );
  }
}
