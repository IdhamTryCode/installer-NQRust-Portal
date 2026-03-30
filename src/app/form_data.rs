// Hardcoded defaults for Keycloak — NOT user-editable
pub const KC_ADMIN_USERNAME: &str = "admin";
pub const KC_ADMIN_PASSWORD: &str = "identity";
pub const KC_DB_NAME: &str = "identity";
pub const KC_DB_USER: &str = "identity";
pub const KC_DB_PASSWORD: &str = "identity";

#[derive(Debug, Clone, PartialEq)]
pub enum FocusState {
    Field(usize),
    SaveButton,
    CancelButton,
}

// Phase 1: Keycloak installation form
#[derive(Debug)]
pub struct KeycloakFormData {
    pub hostname: String,
    pub keycloak_port: String,
    pub portal_port: String,
    pub focus_state: FocusState,
    pub editing: bool,
    pub error_message: String,
}

impl KeycloakFormData {
    pub fn new() -> Self {
        Self {
            hostname: String::new(),
            keycloak_port: "8082".to_string(),
            portal_port: "8080".to_string(),
            focus_state: FocusState::Field(0),
            editing: false,
            error_message: String::new(),
        }
    }

    pub fn get_total_fields(&self) -> usize {
        3
    }

    pub fn get_field_label(&self, idx: usize) -> &str {
        match idx {
            0 => "Hostname / IP",
            1 => "Keycloak Port",
            2 => "Portal Port",
            _ => "",
        }
    }

    pub fn get_field_value(&self, idx: usize) -> &str {
        match idx {
            0 => &self.hostname,
            1 => &self.keycloak_port,
            2 => &self.portal_port,
            _ => "",
        }
    }

    pub fn get_field_value_mut(&mut self, idx: usize) -> Option<&mut String> {
        match idx {
            0 => Some(&mut self.hostname),
            1 => Some(&mut self.keycloak_port),
            2 => Some(&mut self.portal_port),
            _ => None,
        }
    }

    pub fn validate(&mut self) -> bool {
        if self.hostname.trim().is_empty() {
            self.error_message = "Hostname / IP is required".to_string();
            return false;
        }
        if self.hostname.contains(' ') {
            self.error_message = "Hostname must not contain spaces".to_string();
            return false;
        }
        match self.keycloak_port.trim().parse::<u16>() {
            Ok(p) if p >= 1024 => {}
            _ => {
                self.error_message =
                    "Keycloak Port must be a valid port number (1024-65535)".to_string();
                return false;
            }
        }
        match self.portal_port.trim().parse::<u16>() {
            Ok(p) if p >= 1024 => {}
            _ => {
                self.error_message =
                    "Portal Port must be a valid port number (1024-65535)".to_string();
                return false;
            }
        }
        if self.keycloak_port.trim() == self.portal_port.trim() {
            self.error_message = "Keycloak Port and Portal Port must be different".to_string();
            return false;
        }
        self.error_message.clear();
        true
    }
}

impl Default for KeycloakFormData {
    fn default() -> Self {
        Self::new()
    }
}

// Phase 2: Portal installation form
#[derive(Debug)]
pub struct PortalFormData {
    pub realm_name: String,
    pub client_id: String,
    pub client_secret: String,
    pub focus_state: FocusState,
    pub editing: bool,
    pub error_message: String,
}

impl PortalFormData {
    pub fn new() -> Self {
        Self {
            realm_name: "myrealm".to_string(),
            client_id: "nqrust-portal".to_string(),
            client_secret: String::new(),
            focus_state: FocusState::Field(0),
            editing: false,
            error_message: String::new(),
        }
    }

    pub fn get_total_fields(&self) -> usize {
        3
    }

    pub fn get_field_label(&self, idx: usize) -> &str {
        match idx {
            0 => "Realm Name",
            1 => "Client ID",
            2 => "Client Secret",
            _ => "",
        }
    }

    pub fn get_field_value(&self, idx: usize) -> &str {
        match idx {
            0 => &self.realm_name,
            1 => &self.client_id,
            2 => &self.client_secret,
            _ => "",
        }
    }

    pub fn get_field_value_mut(&mut self, idx: usize) -> Option<&mut String> {
        match idx {
            0 => Some(&mut self.realm_name),
            1 => Some(&mut self.client_id),
            2 => Some(&mut self.client_secret),
            _ => None,
        }
    }

    pub fn is_masked(&self, idx: usize) -> bool {
        idx == 2
    }

    pub fn validate(&mut self) -> bool {
        if self.realm_name.trim().is_empty() {
            self.error_message = "Realm Name is required".to_string();
            return false;
        }
        if self.realm_name.contains(' ') {
            self.error_message = "Realm Name must not contain spaces".to_string();
            return false;
        }
        if self.client_id.trim().is_empty() {
            self.error_message = "Client ID is required".to_string();
            return false;
        }
        if self.client_id.contains(' ') {
            self.error_message = "Client ID must not contain spaces".to_string();
            return false;
        }
        if self.client_secret.trim().is_empty() {
            self.error_message = "Client Secret is required".to_string();
            return false;
        }
        self.error_message.clear();
        true
    }
}

impl Default for PortalFormData {
    fn default() -> Self {
        Self::new()
    }
}
