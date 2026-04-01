#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    RegistrySetup,
    Home,
    IdentityForm,
    InstallingIdentity,
    IdentitySuccess,
    PortalForm,
    InstallingPortal,
    PortalSuccess,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HomeMenuItem {
    InstallIdentity,
    InstallPortal,
    UpdateToken,
    Exit,
}
