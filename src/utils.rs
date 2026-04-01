use color_eyre::eyre::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub const ENV_TEMPLATE: &str = include_str!("../env_template");
pub const COMPOSE_TEMPLATE: &str = include_str!("../docker-compose.yaml");
pub const TRAEFIK_DYNAMIC_YML: &str = include_str!("../traefik/dynamic.yml");
pub const DB_INIT_SQL: &str = include_str!("../db/init.sql");

#[allow(dead_code)]
pub fn find_file(filename: &str) -> bool {
    let root = project_root();
    root.join(filename).exists()
}

pub fn project_root() -> PathBuf {
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let candidates = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];

    let mut current = start.as_path();
    loop {
        if candidates.iter().any(|name| current.join(name).exists()) {
            return current.to_path_buf();
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // Fallback: if running from target/*/ build dirs, hop two parents
    if start
        .to_str()
        .map(|s| s.contains("target"))
        .unwrap_or(false)
    {
        if let Some(parent) = start.parent().and_then(|p| p.parent()) {
            return parent.to_path_buf();
        }
    }

    start
}

/// Scaffold all files required by docker-compose into `root` if they don't already exist.
/// Safe to call multiple times — never overwrites existing files.
pub fn ensure_compose_bundle(root: &Path) -> Result<()> {
    // docker-compose.yaml
    let compose_candidates = [
        "docker-compose.yaml",
        "docker-compose.yml",
        "compose.yaml",
        "compose.yml",
    ];
    if !compose_candidates
        .iter()
        .any(|name| root.join(name).exists())
    {
        fs::write(root.join("docker-compose.yaml"), COMPOSE_TEMPLATE)?;
    }

    // traefik/dynamic.yml
    let traefik_dir = root.join("traefik");
    fs::create_dir_all(&traefik_dir)?;
    let traefik_dynamic = traefik_dir.join("dynamic.yml");
    if !traefik_dynamic.exists() {
        fs::write(&traefik_dynamic, TRAEFIK_DYNAMIC_YML)?;
    }

    // db/init.sql
    let db_dir = root.join("db");
    fs::create_dir_all(&db_dir)?;
    let db_init = db_dir.join("init.sql");
    if !db_init.exists() {
        fs::write(&db_init, DB_INIT_SQL)?;
    }

    Ok(())
}
