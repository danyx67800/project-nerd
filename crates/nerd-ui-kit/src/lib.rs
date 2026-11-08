//! Project Nerd design system.
//!
//! The crate owns every visual decision in the application: semantic colours,
//! typography, spacing, radii, motion, icons and custom egui components. Pages
//! must not hard-code colours, font sizes or spacing.

pub mod components;
pub mod gallery;
pub mod icon;
pub mod system;
pub mod theme;
pub mod tokens;

pub use components::*;
pub use icon::{icon, Icon};
pub use theme::install;
pub use tokens::*;

/// Re-export so callers do not need to depend on egui directly for the kit API.
pub use egui;
