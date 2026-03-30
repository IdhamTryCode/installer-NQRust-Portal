#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    RegistrySetup,
    Home,
    KeycloakForm,
    InstallingKeycloak,
    KeycloakSuccess,
    PortalForm,
    InstallingPortal,
    PortalSuccess,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HomeMenuItem {
    InstallKeycloak,
    InstallPortal,
    UpdateToken,
    Exit,
}
