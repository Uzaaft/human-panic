//! Panic messages for humans
//!
//! Handles panics by calling
//! [`std::panic::set_hook`](https://doc.rust-lang.org/std/panic/fn.set_hook.html)
//! to make errors nice for humans.
//!
//! ## Why?
//! When you're building a CLI, polish is super important. Even though Rust is
//! pretty great at safety, it's not unheard of to access the wrong index in a
//! vector or have an assert fail somewhere.
//!
//! When an error eventually occurs, you probably will want to know about it. So
//! instead of just providing an error message on the command line, we can create a
//! call to action for people to submit a report.
//!
//! This should empower people to engage in communication, lowering the chances
//! people might get frustrated. And making it easier to figure out what might be
//! causing bugs.
//!
//! ### Default Output
//!
//! ```txt
//! thread 'main' panicked at 'oops', examples/main.rs:2:3
//! note: Run with `RUST_BACKTRACE=1` for a backtrace.
//! ```
//!
//! ### Human-Panic Output
//!
//! ```txt
//! Well, this is embarrassing.
//!
//! human-panic had a problem and crashed. To help us diagnose the problem you can send us a crash report.
//!
//! We have generated a report file at "/var/folders/zw/bpfvmq390lv2c6gn_6byyv0w0000gn/T/report-8351cad6-d2b5-4fe8-accd-1fcbf4538792.toml". Submit an issue or email with the subject of "human-panic Crash Report" and include the report as an attachment.
//!
//! - Homepage: https://github.com/yoshuawuyts/human-panic
//! - Authors: Yoshua Wuyts <yoshuawuyts@gmail.com>
//!
//! We take privacy seriously, and do not perform any automated error collection. In order to improve the software, we rely on people to submit reports.
//!
//! Thank you kindly!

#![cfg_attr(feature = "nightly", deny(missing_docs))]
#![cfg_attr(feature = "nightly", feature(external_doc))]
#![cfg_attr(feature = "nightly", feature(panic_info_message))]

pub mod report;
use report::{Method, Report};

use native_dialog::{MessageDialog, MessageType};
use std::borrow::Cow;
use std::fmt::Write as _;
use std::io::Result as IoResult;
use std::panic::PanicInfo;
use std::path::{Path, PathBuf}; // import without risk of name clashing

/// A convenient metadata struct that describes a crate
pub struct Metadata {
  /// The crate version
  pub version: Cow<'static, str>,
  /// The crate name
  pub name: Cow<'static, str>,
  /// The list of authors of the crate
  pub authors: Cow<'static, str>,
  /// The URL of the crate's website
  pub homepage: Cow<'static, str>,
}

/// `human-panic` initialisation macro
///
/// You can either call this macro with no arguments `setup_panic!()` or
/// with a Metadata struct, if you don't want the error message to display
/// the values used in your `Cargo.toml` file.
///
/// The Metadata struct can't implement `Default` because of orphan rules, which
/// means you need to provide all fields for initialisation.
///
/// ```
/// use human_panic::setup_panic;
///
/// setup_panic!(Metadata {
///     name: env!("CARGO_PKG_NAME").into(),
///     version: env!("CARGO_PKG_VERSION").into(),
///     authors: "My Company Support <support@mycompany.com>".into(),
///     homepage: "support.mycompany.com".into(),
/// });
/// ```
#[macro_export]
macro_rules! setup_panic {
  ($meta:expr) => {
    #[allow(unused_imports)]
    use std::panic::{self, PanicInfo};
    #[allow(unused_imports)]
    use $crate::{handle_dump, print_msg, Metadata};

    #[cfg(not(debug_assertions))]
    match ::std::env::var("RUST_BACKTRACE") {
      Err(_) => {
        panic::set_hook(Box::new(move |info: &PanicInfo| {
          let file_path = handle_dump(&$meta, info);
          print_msg(file_path, &$meta)
            .expect("human-panic: printing error message to console failed");
        }));
      }
      Ok(_) => {}
    }
  };

  () => {
    #[allow(unused_imports)]
    use std::panic::{self, PanicInfo};
    #[allow(unused_imports)]
    use $crate::{handle_dump, print_msg, Metadata};

    #[cfg(not(debug_assertions))]
    match ::std::env::var("RUST_BACKTRACE") {
      Err(_) => {
        let meta = Metadata {
          version: env!("CARGO_PKG_VERSION").into(),
          name: env!("CARGO_PKG_NAME").into(),
          authors: env!("CARGO_PKG_AUTHORS").replace(":", ", ").into(),
          homepage: env!("CARGO_PKG_HOMEPAGE").into(),
        };

        panic::set_hook(Box::new(move |info: &PanicInfo| {
          let file_path = handle_dump(&meta, info);
          print_msg(file_path, &meta)
            .expect("human-panic: printing error message to console failed");
        }));
      }
      Ok(_) => {}
    }
  };
}

/// Utility function that prints a message to our human users
pub fn print_msg<P: AsRef<Path>>(
  file_path: Option<P>,
  meta: &Metadata,
) -> IoResult<()> {
  let (_version, name, authors, homepage) =
    (&meta.version, &meta.name, &meta.authors, &meta.homepage);

  let mut error_message = "Well, this is embarrassing.\n".to_string();
  let _ = writeln!(error_message, "{} had a problem and crashed.", name,);
  let _ = writeln!(
    error_message,
    "We have generated a report file at: \n
    {} \n
    Submit an \
    issue or email and include the \
    report as an attachment.",
    match file_path {
      Some(fp) => format!("{}", fp.as_ref().display()),
      None => "<Failed to store file to disk>".to_string(),
    },
    name
  );

  if !homepage.is_empty() {
    let _ = writeln!(error_message, "- Homepage: {}", homepage);
  }
  if !authors.is_empty() {
    let _ = writeln!(error_message, "- Authors: {}", authors);
  }

  MessageDialog::new()
    .set_type(MessageType::Info)
    .set_title("An error happened")
    .set_text(&error_message)
    .set_type(MessageType::Warning)
    .show_alert();

  Ok(())
}

/// Utility function which will handle dumping information to disk
pub fn handle_dump(meta: &Metadata, panic_info: &PanicInfo) -> Option<PathBuf> {
  let mut expl = String::new();

  #[cfg(feature = "nightly")]
  let message = panic_info.message().map(|m| format!("{}", m));

  #[cfg(not(feature = "nightly"))]
  let message = match (
    panic_info.payload().downcast_ref::<&str>(),
    panic_info.payload().downcast_ref::<String>(),
  ) {
    (Some(s), _) => Some(s.to_string()),
    (_, Some(s)) => Some(s.to_string()),
    (None, None) => None,
  };

  let cause = match message {
    Some(m) => m,
    None => "Unknown".into(),
  };

  match panic_info.location() {
    Some(location) => writeln!(
      expl,
      "Panic occurred in file '{}' at line {}",
      location.file(),
      location.line()
    )
    .ok()?,
    None => writeln!(expl, "Panic location unknown.\n").ok()?,
  }

  let report =
    Report::new(&meta.name, &meta.version, Method::Panic, expl, cause);

  match report.persist() {
    Ok(f) => Some(f),
    Err(_) => {
      eprintln!("{}", report.serialize().unwrap());
      None
    }
  }
}
