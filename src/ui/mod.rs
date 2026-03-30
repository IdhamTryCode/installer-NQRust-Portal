mod ascii_art;
mod error;
mod home;
mod installing;
mod keycloak_form;
mod portal_form;
mod registry;
mod success;

pub use ascii_art::{ASCII_HEADER, get_orange_accent, get_orange_color};
pub use error::{ErrorView, render_error};
pub use home::render_home;
pub use installing::{InstallingView, render_installing};
pub use keycloak_form::render_keycloak_form;
pub use portal_form::render_portal_form;
pub use registry::{RegistrySetupView, render_registry_setup};
pub use success::{SuccessView, render_success};
