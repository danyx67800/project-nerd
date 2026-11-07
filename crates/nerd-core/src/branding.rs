//! Single source of truth for the product identity.
//!
//! `scripts/bundle.sh` and the GitHub workflow read the application name from
//! this file, so the product can be renamed in exactly one place.

/// Human readable product name. Used for the window title, the menu bar,
/// `Info.plist` and the Application Support folder.
pub const APP_NAME: &str = "Project Nerd";

/// Short, filesystem-safe identifier.
pub const APP_ID: &str = "project-nerd";

/// Bundle identifier used when no override is supplied to `scripts/bundle.sh`.
pub const DEFAULT_BUNDLE_ID: &str = "com.projectnerd.app";

/// Current on-disk project schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// File extension of a saved evening (without the leading dot).
pub const PROJECT_EXTENSION: &str = "nerdshow";

/// Minimum supported macOS version.
pub const MIN_MACOS_VERSION: &str = "13.0";

/// Version of this build, injected by Cargo.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
