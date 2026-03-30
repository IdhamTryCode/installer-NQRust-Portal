use color_eyre::{Result, eyre::eyre};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, Frame};
use reqwest::Client;
use serde::Deserialize;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::{env, fs};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::ui::{ErrorView, InstallingView, RegistrySetupView, SuccessView};
use crate::utils;

pub mod form_data;
pub mod registry_form;
pub mod state;

pub use form_data::{KeycloakFormData, PortalFormData};
use registry_form::RegistryForm;
pub use state::{AppState, HomeMenuItem};

enum RegistryAction {
    Submit,
    Skip,
}

#[derive(Debug)]
pub struct App {
    running: bool,
    pub state: AppState,
    pub logs: Vec<String>,
    pub progress: f64,
    pub current_service: String,
    pub total_services: usize,
    pub completed_services: usize,
    pub keycloak_form: KeycloakFormData,
    pub portal_form: PortalFormData,
    pub registry_form: RegistryForm,
    pub registry_status: Option<String>,
    pub ghcr_token: Option<String>,
    pub menu_selection: usize,
    pub keycloak_running: bool,
    pub portal_running: bool,
    pub install_phase: String,
}

impl App {
    pub fn new() -> Self {
        let token_from_env = env::var("GHCR_TOKEN")
            .or_else(|_| env::var("GITHUB_TOKEN"))
            .or_else(|_| env::var("GH_TOKEN"))
            .ok();
        let token_from_disk = Self::load_token_from_disk();
        let initial_token = token_from_env.or(token_from_disk);

        let mut registry_form = RegistryForm::new();
        if let Some(token) = initial_token.clone() {
            registry_form.token = token;
        }

        let initial_state = if initial_token.is_some() {
            AppState::Home
        } else {
            AppState::RegistrySetup
        };

        let (keycloak_running, portal_running) = Self::detect_containers_sync();

        Self {
            running: true,
            state: initial_state,
            logs: Vec::new(),
            progress: 0.0,
            current_service: String::new(),
            total_services: 3,
            completed_services: 0,
            keycloak_form: KeycloakFormData::new(),
            portal_form: PortalFormData::new(),
            registry_form,
            registry_status: None,
            ghcr_token: initial_token,
            menu_selection: 0,
            keycloak_running,
            portal_running,
            install_phase: String::new(),
        }
    }

    fn detect_containers_sync() -> (bool, bool) {
        let root = utils::project_root();
        let output = std::process::Command::new("docker")
            .args(["compose", "ps", "--services", "--filter", "status=running"])
            .current_dir(&root)
            .output();

        match output {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
                let keycloak = text
                    .lines()
                    .any(|l| l.contains("keycloak") && !l.contains("db"));
                let portal = text.lines().any(|l| l.contains("portal"));
                (keycloak, portal)
            }
            Err(_) => (false, false),
        }
    }

    pub fn get_menu_items(&self) -> Vec<HomeMenuItem> {
        let mut items = vec![HomeMenuItem::InstallKeycloak, HomeMenuItem::InstallPortal];
        if self.ghcr_token.is_some() {
            items.push(HomeMenuItem::UpdateToken);
        }
        items.push(HomeMenuItem::Exit);
        items
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;

            match &self.state.clone() {
                AppState::RegistrySetup => {
                    if let Some(action) = self.handle_registry_events()? {
                        match action {
                            RegistryAction::Submit => match self.try_registry_login().await {
                                Ok(true) => {
                                    self.state = AppState::Home;
                                }
                                Ok(false) => {}
                                Err(e) => {
                                    self.registry_status =
                                        Some(format!("Failed to run docker login: {}", e));
                                }
                            },
                            RegistryAction::Skip => {
                                self.registry_status = Some(
                                    "Skipped GHCR login. You can authenticate later from the menu."
                                        .to_string(),
                                );
                                self.state = AppState::Home;
                            }
                        }
                    }
                }
                AppState::Home => {
                    self.handle_home_events()?;
                }
                AppState::KeycloakForm => {
                    self.handle_keycloak_form_events()?;
                }
                AppState::InstallingKeycloak => {
                    self.install_phase = "Keycloak".to_string();
                    self.logs
                        .push("🚀 Starting Keycloak installation...".to_string());
                    let result = self
                        .run_docker_compose_services(
                            &["traefik", "keycloak-db", "keycloak"],
                            &mut terminal,
                        )
                        .await;
                    match result {
                        Ok(_) => {
                            self.progress = 100.0;
                            self.state = AppState::KeycloakSuccess;
                        }
                        Err(e) => {
                            self.state =
                                AppState::Error(format!("Keycloak installation failed: {}", e));
                        }
                    }
                }
                AppState::KeycloakSuccess => {
                    self.handle_keycloak_success_events()?;
                }
                AppState::PortalForm => {
                    self.handle_portal_form_events()?;
                }
                AppState::InstallingPortal => {
                    self.install_phase = "Portal".to_string();
                    self.logs
                        .push("🚀 Starting Portal installation...".to_string());
                    let result = self
                        .run_docker_compose_services(&["portal"], &mut terminal)
                        .await;
                    match result {
                        Ok(_) => {
                            self.progress = 100.0;
                            self.state = AppState::PortalSuccess;
                        }
                        Err(e) => {
                            self.state =
                                AppState::Error(format!("Portal installation failed: {}", e));
                        }
                    }
                }
                AppState::PortalSuccess => {
                    if event::poll(std::time::Duration::from_millis(100))? {
                        if let Event::Key(key) = event::read()? {
                            if key.kind == KeyEventKind::Press {
                                match key.code {
                                    KeyCode::Char('c')
                                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                    {
                                        self.running = false;
                                    }
                                    KeyCode::Char('q') => self.running = false,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                AppState::Error(_) => {
                    if event::poll(std::time::Duration::from_millis(100))? {
                        if let Event::Key(key) = event::read()? {
                            if key.kind == KeyEventKind::Press {
                                if let KeyCode::Char('c') = key.code {
                                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                                        self.running = false;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_home_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let items = self.get_menu_items();
                    match key.code {
                        KeyCode::Up => {
                            if self.menu_selection > 0 {
                                self.menu_selection -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if self.menu_selection + 1 < items.len() {
                                self.menu_selection += 1;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(item) = items.get(self.menu_selection) {
                                match item {
                                    HomeMenuItem::InstallKeycloak => {
                                        self.keycloak_form = KeycloakFormData::new();
                                        self.state = AppState::KeycloakForm;
                                    }
                                    HomeMenuItem::InstallPortal => {
                                        self.portal_form = PortalFormData::new();
                                        self.state = AppState::PortalForm;
                                    }
                                    HomeMenuItem::UpdateToken => {
                                        self.registry_status = Some(
                                            "Update token and submit (Ctrl+S). Esc to cancel."
                                                .to_string(),
                                        );
                                        self.registry_form.focus_state =
                                            registry_form::FocusState::Field(0);
                                        self.registry_form.error_message.clear();
                                        self.registry_form.token =
                                            self.ghcr_token.clone().unwrap_or_default();
                                        self.state = AppState::RegistrySetup;
                                    }
                                    HomeMenuItem::Exit => {
                                        self.running = false;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.running = false;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_keycloak_form_events(&mut self) -> Result<()> {
        use form_data::FocusState;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if self.keycloak_form.editing {
                        match key.code {
                            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if let FocusState::Field(i) = self.keycloak_form.focus_state.clone()
                                {
                                    if let Some(val) = self.keycloak_form.get_field_value_mut(i) {
                                        val.push(c);
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if let FocusState::Field(i) = self.keycloak_form.focus_state.clone()
                                {
                                    if let Some(val) = self.keycloak_form.get_field_value_mut(i) {
                                        val.pop();
                                    }
                                }
                            }
                            KeyCode::Enter | KeyCode::Esc => {
                                self.keycloak_form.editing = false;
                            }
                            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.keycloak_form.editing = false;
                                self.try_save_keycloak_form();
                            }
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.running = false;
                            }
                            _ => {}
                        }
                        return Ok(());
                    }

                    match key.code {
                        KeyCode::Up | KeyCode::BackTab => {
                            self.keycloak_form.focus_state = match &self.keycloak_form.focus_state {
                                FocusState::Field(0) => FocusState::CancelButton,
                                FocusState::Field(i) => FocusState::Field(i - 1),
                                FocusState::SaveButton => {
                                    FocusState::Field(self.keycloak_form.get_total_fields() - 1)
                                }
                                FocusState::CancelButton => FocusState::SaveButton,
                            };
                        }
                        KeyCode::Down | KeyCode::Tab => {
                            let total = self.keycloak_form.get_total_fields();
                            self.keycloak_form.focus_state = match &self.keycloak_form.focus_state {
                                FocusState::Field(i) if *i + 1 < total => FocusState::Field(i + 1),
                                FocusState::Field(_) => FocusState::SaveButton,
                                FocusState::SaveButton => FocusState::CancelButton,
                                FocusState::CancelButton => FocusState::Field(0),
                            };
                        }
                        KeyCode::Enter => match &self.keycloak_form.focus_state {
                            FocusState::Field(_) => {
                                self.keycloak_form.editing = true;
                            }
                            FocusState::SaveButton => {
                                self.try_save_keycloak_form();
                            }
                            FocusState::CancelButton => {
                                self.state = AppState::Home;
                            }
                        },
                        KeyCode::Esc => {
                            self.state = AppState::Home;
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.try_save_keycloak_form();
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.running = false;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn try_save_keycloak_form(&mut self) {
        if self.keycloak_form.validate() {
            if let Err(e) = self.generate_env() {
                self.state = AppState::Error(format!("Failed to generate .env: {}", e));
                return;
            }
            self.progress = 0.0;
            self.completed_services = 0;
            self.total_services = 3;
            self.logs.clear();
            self.state = AppState::InstallingKeycloak;
        }
    }

    fn handle_portal_form_events(&mut self) -> Result<()> {
        use form_data::FocusState;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if self.portal_form.editing {
                        match key.code {
                            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if let FocusState::Field(i) = self.portal_form.focus_state.clone() {
                                    if let Some(val) = self.portal_form.get_field_value_mut(i) {
                                        val.push(c);
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if let FocusState::Field(i) = self.portal_form.focus_state.clone() {
                                    if let Some(val) = self.portal_form.get_field_value_mut(i) {
                                        val.pop();
                                    }
                                }
                            }
                            KeyCode::Enter | KeyCode::Esc => {
                                self.portal_form.editing = false;
                            }
                            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.portal_form.editing = false;
                                self.try_save_portal_form();
                            }
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.running = false;
                            }
                            _ => {}
                        }
                        return Ok(());
                    }

                    match key.code {
                        KeyCode::Up | KeyCode::BackTab => {
                            self.portal_form.focus_state = match &self.portal_form.focus_state {
                                FocusState::Field(0) => FocusState::CancelButton,
                                FocusState::Field(i) => FocusState::Field(i - 1),
                                FocusState::SaveButton => {
                                    FocusState::Field(self.portal_form.get_total_fields() - 1)
                                }
                                FocusState::CancelButton => FocusState::SaveButton,
                            };
                        }
                        KeyCode::Down | KeyCode::Tab => {
                            let total = self.portal_form.get_total_fields();
                            self.portal_form.focus_state = match &self.portal_form.focus_state {
                                FocusState::Field(i) if *i + 1 < total => FocusState::Field(i + 1),
                                FocusState::Field(_) => FocusState::SaveButton,
                                FocusState::SaveButton => FocusState::CancelButton,
                                FocusState::CancelButton => FocusState::Field(0),
                            };
                        }
                        KeyCode::Enter => match &self.portal_form.focus_state {
                            FocusState::Field(_) => {
                                self.portal_form.editing = true;
                            }
                            FocusState::SaveButton => {
                                self.try_save_portal_form();
                            }
                            FocusState::CancelButton => {
                                self.state = AppState::Home;
                            }
                        },
                        KeyCode::Esc => {
                            self.state = AppState::Home;
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.try_save_portal_form();
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.running = false;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn try_save_portal_form(&mut self) {
        if self.portal_form.validate() {
            if let Err(e) = self.update_env_for_portal() {
                self.state = AppState::Error(format!("Failed to update .env: {}", e));
                return;
            }
            self.progress = 0.0;
            self.completed_services = 0;
            self.total_services = 1;
            self.logs.clear();
            self.state = AppState::InstallingPortal;
        }
    }

    fn handle_keycloak_success_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('i') | KeyCode::Char('I') => {
                            self.portal_form = PortalFormData::new();
                            self.state = AppState::PortalForm;
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            self.running = false;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.running = false;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_registry_events(&mut self) -> Result<Option<RegistryAction>> {
        use registry_form::FocusState;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Up => {
                            self.registry_form.focus_state = match &self.registry_form.focus_state {
                                FocusState::Field(_) => FocusState::Field(0),
                                FocusState::SaveButton => FocusState::Field(0),
                                FocusState::CancelButton => FocusState::SaveButton,
                            };
                        }
                        KeyCode::Down => {
                            self.registry_form.focus_state = match &self.registry_form.focus_state {
                                FocusState::Field(_) => FocusState::SaveButton,
                                FocusState::SaveButton => FocusState::CancelButton,
                                FocusState::CancelButton => FocusState::CancelButton,
                            };
                        }
                        KeyCode::Tab => {
                            self.registry_form.focus_state = match &self.registry_form.focus_state {
                                FocusState::Field(_) => FocusState::SaveButton,
                                FocusState::SaveButton => FocusState::CancelButton,
                                FocusState::CancelButton => FocusState::Field(0),
                            };
                        }
                        KeyCode::BackTab => {
                            self.registry_form.focus_state = match &self.registry_form.focus_state {
                                FocusState::Field(_) => FocusState::CancelButton,
                                FocusState::SaveButton => FocusState::Field(0),
                                FocusState::CancelButton => FocusState::SaveButton,
                            };
                        }
                        KeyCode::Enter => match &self.registry_form.focus_state {
                            FocusState::SaveButton => return Ok(Some(RegistryAction::Submit)),
                            FocusState::CancelButton => return Ok(Some(RegistryAction::Skip)),
                            _ => {}
                        },
                        KeyCode::Esc => return Ok(Some(RegistryAction::Skip)),
                        KeyCode::Char(c) => {
                            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                                if matches!(&self.registry_form.focus_state, FocusState::Field(_)) {
                                    self.registry_form.get_current_value_mut().push(c);
                                }
                            } else if c == 'c' {
                                self.running = false;
                            } else if c == 's' {
                                return Ok(Some(RegistryAction::Submit));
                            }
                        }
                        KeyCode::Backspace => {
                            if matches!(&self.registry_form.focus_state, FocusState::Field(_)) {
                                self.registry_form.get_current_value_mut().pop();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(None)
    }

    async fn try_registry_login(&mut self) -> Result<bool> {
        if !self.registry_form.validate() {
            self.registry_status = Some(self.registry_form.error_message.clone());
            return Ok(false);
        }

        let token = self.registry_form.token.trim().to_string();
        if token.is_empty() {
            self.registry_status = Some("Token is required".to_string());
            return Ok(false);
        }

        self.registry_status = Some("Resolving GitHub username from token...".to_string());

        let username = match self.fetch_github_username(&token).await {
            Ok(name) => name,
            Err(e) => {
                self.registry_status = Some(format!("Failed to resolve username: {}", e));
                return Ok(false);
            }
        };

        self.registry_status = Some(format!("Logging in to ghcr.io as {}...", username));

        let mut child = Command::new("docker")
            .args(["login", "ghcr.io", "-u", &username, "--password-stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(format!("{}\n", token).as_bytes()).await?;
        } else {
            self.registry_status = Some("Failed to communicate with docker login".to_string());
            return Ok(false);
        }

        let output = child.wait_with_output().await?;

        if output.status.success() {
            self.registry_status = Some("Authenticated with ghcr.io successfully".to_string());
            self.ghcr_token = Some(token.clone());
            self.registry_form.error_message.clear();
            if let Err(e) = self.persist_token(&token) {
                self.registry_status =
                    Some(format!("Authenticated, but failed to cache token: {}", e));
            }
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let first_line = stderr.lines().find(|l| !l.trim().is_empty());
            self.registry_status = Some(format!(
                "Docker login failed: {}",
                first_line.unwrap_or("unknown error")
            ));
            Ok(false)
        }
    }

    async fn fetch_github_username(&self, token: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct GitHubUser {
            login: String,
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        let response = client
            .get("https://api.github.com/user")
            .header("User-Agent", "nqrust-portal")
            .header("Accept", "application/vnd.github+json")
            .bearer_auth(token)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(eyre!("GitHub API returned {}: {}", status, body));
        }

        let user: GitHubUser = response.json().await?;
        Ok(user.login)
    }

    fn token_file_path() -> PathBuf {
        utils::project_root().join(".ghcr_token")
    }

    fn load_token_from_disk() -> Option<String> {
        fs::read_to_string(Self::token_file_path())
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn persist_token(&self, token: &str) -> Result<()> {
        let path = Self::token_file_path();
        fs::write(&path, token)?;
        #[cfg(unix)]
        {
            let perms = std::fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }
        Ok(())
    }

    fn generate_env(&self) -> Result<()> {
        let root = utils::project_root();
        let env_path = root.join(".env");

        let nextauth_secret = format!(
            "{}{}",
            uuid::Uuid::new_v4().to_string().replace('-', ""),
            uuid::Uuid::new_v4().to_string().replace('-', "")
        );

        let mut content = utils::ENV_TEMPLATE.to_string();
        content = content.replace("{{NEXTAUTH_SECRET}}", &nextauth_secret);
        content = content.replace("{{HOSTNAME}}", &self.keycloak_form.hostname);
        content = content.replace("{{KEYCLOAK_PORT}}", &self.keycloak_form.keycloak_port);
        content = content.replace("{{PORTAL_PORT}}", &self.keycloak_form.portal_port);
        // These are also used directly by docker-compose ports mapping
        // (already replaced above via KEYCLOAK_PORT and PORTAL_PORT placeholders)
        content = content.replace("{{REALM_NAME}}", "myrealm");
        content = content.replace("{{KEYCLOAK_CLIENT_ID}}", "nqrust-portal");
        content = content.replace("{{KEYCLOAK_CLIENT_SECRET}}", "");

        fs::write(env_path, content)?;
        Ok(())
    }

    fn update_env_for_portal(&self) -> Result<()> {
        let root = utils::project_root();
        let env_path = root.join(".env");

        let existing = if env_path.exists() {
            fs::read_to_string(&env_path)?
        } else {
            return Err(eyre!(".env file not found. Please install Keycloak first."));
        };

        // Extract hostname and keycloak_port from existing KEYCLOAK_EXTERNAL_HOST line
        let hostname = self.keycloak_form.hostname.clone();
        let keycloak_port = self.keycloak_form.keycloak_port.clone();

        // Try to read from existing .env if keycloak_form is empty
        let (hostname, keycloak_port) = if hostname.is_empty() {
            let extracted = existing
                .lines()
                .find(|l| l.starts_with("KEYCLOAK_EXTERNAL_HOST="))
                .and_then(|l| l.strip_prefix("KEYCLOAK_EXTERNAL_HOST=http://"))
                .map(|s| {
                    let parts: Vec<&str> = s.splitn(2, ':').collect();
                    (
                        parts.first().unwrap_or(&"localhost").to_string(),
                        parts.get(1).unwrap_or(&"8082").to_string(),
                    )
                })
                .unwrap_or(("localhost".to_string(), "8082".to_string()));
            extracted
        } else {
            (hostname, keycloak_port)
        };

        let realm = &self.portal_form.realm_name;
        let client_id = &self.portal_form.client_id;
        let client_secret = &self.portal_form.client_secret;

        let new_lines: Vec<String> = existing
            .lines()
            .map(|line| {
                if line.starts_with("KEYCLOAK_CLIENT_ID=") {
                    format!("KEYCLOAK_CLIENT_ID={}", client_id)
                } else if line.starts_with("KEYCLOAK_CLIENT_SECRET=") {
                    format!("KEYCLOAK_CLIENT_SECRET={}", client_secret)
                } else if line.starts_with("KEYCLOAK_ISSUER=") {
                    format!("KEYCLOAK_ISSUER=http://keycloak:8080/realms/{}", realm)
                } else if line.starts_with("KEYCLOAK_EXTERNAL_BASE=") {
                    format!(
                        "KEYCLOAK_EXTERNAL_BASE=http://{}:{}/realms/{}",
                        hostname, keycloak_port, realm
                    )
                } else if line.starts_with("NEXT_PUBLIC_IDENTITY_ISSUER_URL=") {
                    format!(
                        "NEXT_PUBLIC_IDENTITY_ISSUER_URL=http://{}:{}/realms/{}",
                        hostname, keycloak_port, realm
                    )
                } else {
                    line.to_string()
                }
            })
            .collect();

        fs::write(env_path, new_lines.join("\n"))?;
        Ok(())
    }

    async fn run_docker_compose_services(
        &mut self,
        services: &[&str],
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        let compose_cmd = self.detect_compose_command().await?;
        let root = utils::project_root();

        let compose_candidates = [
            "docker-compose.yaml",
            "docker-compose.yml",
            "compose.yaml",
            "compose.yml",
        ];
        if !compose_candidates.iter().any(|f| root.join(f).exists()) {
            return Err(eyre!("docker-compose.yaml not found in {}", root.display()));
        }

        self.add_log(&format!(
            "🚀 Running: {} up -d {}",
            compose_cmd.join(" "),
            services.join(" ")
        ));
        let _ = self.redraw(terminal);

        let mut cmd = Command::new(&compose_cmd[0]);
        if compose_cmd.len() > 1 {
            cmd.arg(&compose_cmd[1]);
        }
        cmd.arg("up").arg("-d");
        for svc in services {
            cmd.arg(svc);
        }
        cmd.env("DOCKER_CLI_PROGRESS", "plain")
            .current_dir(&root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        loop {
            tokio::select! {
                result = stdout_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => { self.process_log_line(&line); let _ = self.redraw(terminal); }
                        Ok(None) => break,
                        Err(e) => { self.add_log(&format!("❌ stdout error: {}", e)); break; }
                    }
                }
                result = stderr_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => { self.process_log_line(&line); let _ = self.redraw(terminal); }
                        Ok(None) => break,
                        Err(e) => { self.add_log(&format!("❌ stderr error: {}", e)); break; }
                    }
                }
            }
        }

        let status = child.wait().await?;
        if status.success() {
            self.add_log("✅ Services started successfully!");
            self.progress = 100.0;
            Ok(())
        } else {
            Err(eyre!("docker compose up failed"))
        }
    }

    fn process_log_line(&mut self, line: &str) {
        let lower = line.to_lowercase();

        if lower.contains("pulling") {
            if let Some(service) = self.extract_service_name(line) {
                self.current_service = service.clone();
                self.add_log(&format!("⬇️  Pulling image for {}...", service));
            }
        } else if lower.contains("pulled") {
            self.add_log("✓ Image pulled");
        } else if lower.contains("creating") {
            if let Some(service) = self.extract_service_name(line) {
                self.current_service = service.clone();
                self.add_log(&format!("🔨 Creating {}...", service));
            }
        } else if lower.contains("created") {
            self.add_log("✓ Container created");
        } else if lower.contains("starting") {
            if let Some(service) = self.extract_service_name(line) {
                self.current_service = service.clone();
                self.add_log(&format!("▶️  Starting {}...", service));
            }
        } else if lower.contains("started") {
            self.completed_services += 1;
            self.progress = (self.completed_services as f64 / self.total_services as f64) * 100.0;
            self.add_log(&format!(
                "✅ Service started ({}/{})",
                self.completed_services, self.total_services
            ));
        } else if lower.contains("running") {
            self.add_log("🟢 Service is running");
        } else if lower.contains("error") || lower.contains("failed") {
            self.add_log(&format!("❌ {}", line));
        } else if !line.trim().is_empty() {
            self.add_log(&format!("ℹ️  {}", line));
        }
    }

    fn extract_service_name(&self, line: &str) -> Option<String> {
        let services = ["traefik", "keycloak-db", "keycloak", "portal"];
        let lower = line.to_lowercase();
        for service in services {
            if lower.contains(service) {
                return Some(service.to_string());
            }
        }
        None
    }

    async fn detect_compose_command(&self) -> Result<Vec<String>> {
        let v1 = Command::new("docker")
            .args(["compose", "version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        if v1.map(|s| s.success()).unwrap_or(false) {
            return Ok(vec!["docker".to_string(), "compose".to_string()]);
        }

        let v2 = Command::new("docker-compose")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        if v2.map(|s| s.success()).unwrap_or(false) {
            return Ok(vec!["docker-compose".to_string()]);
        }

        Err(eyre!(
            "Neither 'docker compose' nor 'docker-compose' found. Please install Docker Compose."
        ))
    }

    fn add_log(&mut self, message: &str) {
        self.logs.push(message.to_string());
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    fn redraw(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        terminal.draw(|frame| self.render(frame))?;
        Ok(())
    }

    #[allow(dead_code)]
    fn add_log_and_redraw(&mut self, terminal: &mut DefaultTerminal, msg: &str) {
        self.add_log(msg);
        let _ = self.redraw(terminal);
    }

    fn keycloak_url(&self) -> String {
        let hostname = if self.keycloak_form.hostname.is_empty() {
            "localhost".to_string()
        } else {
            self.keycloak_form.hostname.clone()
        };
        let port = if self.keycloak_form.keycloak_port.is_empty() {
            "8082".to_string()
        } else {
            self.keycloak_form.keycloak_port.clone()
        };
        format!("http://{}:{}/admin", hostname, port)
    }

    fn portal_url(&self) -> String {
        let hostname = if self.keycloak_form.hostname.is_empty() {
            "localhost".to_string()
        } else {
            self.keycloak_form.hostname.clone()
        };
        let port = if self.keycloak_form.portal_port.is_empty() {
            "8080".to_string()
        } else {
            self.keycloak_form.portal_port.clone()
        };
        format!("http://{}:{}", hostname, port)
    }

    fn render(&mut self, frame: &mut Frame) {
        match &self.state.clone() {
            AppState::RegistrySetup => {
                let view = RegistrySetupView {
                    form: &self.registry_form,
                    status: self.registry_status.as_deref(),
                };
                crate::ui::render_registry_setup(frame, &view);
            }
            AppState::Home => {
                crate::ui::render_home(frame, self);
            }
            AppState::KeycloakForm => {
                crate::ui::render_keycloak_form(frame, self);
            }
            AppState::PortalForm => {
                crate::ui::render_portal_form(frame, self);
            }
            AppState::InstallingKeycloak => {
                let view = InstallingView {
                    progress: self.progress,
                    current_service: &self.current_service,
                    completed_services: self.completed_services,
                    total_services: self.total_services,
                    logs: &self.logs,
                    phase: "Keycloak",
                };
                crate::ui::render_installing(frame, &view);
            }
            AppState::InstallingPortal => {
                let view = InstallingView {
                    progress: self.progress,
                    current_service: &self.current_service,
                    completed_services: self.completed_services,
                    total_services: self.total_services,
                    logs: &self.logs,
                    phase: "Portal",
                };
                crate::ui::render_installing(frame, &view);
            }
            AppState::KeycloakSuccess => {
                let kc_url = self.keycloak_url();
                let portal_url = self.portal_url();
                let view = SuccessView {
                    logs: &self.logs,
                    phase: "Keycloak",
                    keycloak_url: &kc_url,
                    portal_url: &portal_url,
                };
                crate::ui::render_success(frame, &view);
            }
            AppState::PortalSuccess => {
                let kc_url = self.keycloak_url();
                let portal_url = self.portal_url();
                let view = SuccessView {
                    logs: &self.logs,
                    phase: "Portal",
                    keycloak_url: &kc_url,
                    portal_url: &portal_url,
                };
                crate::ui::render_success(frame, &view);
            }
            AppState::Error(err) => {
                let view = ErrorView {
                    error: err,
                    logs: &self.logs,
                };
                crate::ui::render_error(frame, &view);
            }
        }
    }
}
