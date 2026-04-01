// airgapped/docker.rs
// Docker operations for loading images in airgapped mode

use color_eyre::{Result, eyre::eyre};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// List of required Docker images (must match save-images.sh)
const REQUIRED_IMAGES: &[(&str, &str)] = &[
    ("traefik:v3.4", "traefik.tar.gz"),
    ("postgres:16-alpine", "postgres.tar.gz"),
    (
        "ghcr.io/nexusquantum/nqrust-identity:latest",
        "nqrust-identity.tar.gz",
    ),
    (
        "ghcr.io/nexusquantum/nqrust-identity-portal:latest",
        "nqrust-identity-portal.tar.gz",
    ),
];

/// Check if Docker is available
pub fn check_docker_available() -> Result<()> {
    let output = Command::new("docker").arg("--version").output();

    match output {
        Ok(_) => Ok(()),
        Err(_) => Err(eyre!(
            "Docker is not installed or not in PATH\n\n\
             Troubleshooting:\n\
             - Install Docker: https://docs.docker.com/get-docker/\n\
             - Ensure 'docker' command is in your PATH\n\
             - Try running: which docker"
        )),
    }
}

/// Check if Docker daemon is running
pub fn check_docker_running() -> Result<()> {
    let output = Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match output {
        Ok(status) if status.success() => Ok(()),
        _ => Err(eyre!(
            "Docker daemon is not running\n\n\
             Troubleshooting:\n\
             - Start Docker daemon: sudo systemctl start docker\n\
             - Enable Docker on boot: sudo systemctl enable docker\n\
             - Check Docker status: sudo systemctl status docker\n\
             - Ensure your user is in docker group: sudo usermod -aG docker $USER"
        )),
    }
}

/// Check if a specific Docker image exists locally
fn image_exists(image_name: &str) -> Result<bool> {
    let output = Command::new("docker")
        .args(&["images", "-q", image_name])
        .output()?;

    Ok(!output.stdout.is_empty())
}

/// Check if all required images are already loaded
pub fn check_all_images_exist() -> Result<bool> {
    // First check if Docker is available
    if check_docker_available().is_err() || check_docker_running().is_err() {
        return Ok(false);
    }

    // Check each required image
    for (image_name, _) in REQUIRED_IMAGES {
        if !image_exists(image_name)? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Load a single Docker image from tar.gz file using Rust native decompression
fn load_image(tar_gz_path: &Path, image_name: &str) -> Result<()> {
    println!("    Loading {}...", image_name);

    // Open the compressed tar.gz file
    let file = File::open(tar_gz_path).map_err(|e| {
        eyre!(
            "Failed to open image file '{}': {}\n\n\
             Troubleshooting:\n\
             - Verify file exists: ls -lh {}\n\
             - Check file permissions: chmod 644 {}\n\
             - Ensure sufficient disk space: df -h",
            tar_gz_path.display(),
            e,
            tar_gz_path.display(),
            tar_gz_path.display()
        )
    })?;

    // Decompress with Rust native GzDecoder
    let mut decoder = GzDecoder::new(file);

    // Spawn docker load process
    let mut docker_load = Command::new("docker")
        .arg("load")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            eyre!(
                "Failed to spawn 'docker load' command: {}\n\n\
                 Troubleshooting:\n\
                 - Ensure Docker is installed: docker --version\n\
                 - Check Docker daemon is running: sudo systemctl status docker\n\
                 - Verify Docker permissions: docker ps",
                e
            )
        })?;

    // Get stdin handle
    let mut stdin = docker_load
        .stdin
        .take()
        .ok_or_else(|| eyre!("Failed to open stdin for docker load"))?;

    // Stream decompressed data to docker load
    io::copy(&mut decoder, &mut stdin).map_err(|e| {
        eyre!(
            "Failed to stream image data to Docker: {}\n\n\
             Troubleshooting:\n\
             - Check disk space: df -h\n\
             - Verify image file is not corrupted: gzip -t {}\n\
             - Check Docker daemon logs: sudo journalctl -u docker -n 50",
            e,
            tar_gz_path.display()
        )
    })?;

    // Close stdin to signal end of input
    drop(stdin);

    // Wait for docker load to complete
    let output = docker_load
        .wait_with_output()
        .map_err(|e| eyre!("Failed to wait for docker load: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!(
            "Failed to load image '{}': {}\n\n\
             Troubleshooting:\n\
             - Ensure Docker daemon is running: sudo systemctl start docker\n\
             - Check disk space: df -h /var/lib/docker\n\
             - Verify image file integrity: sha256sum {}\n\
             - Check Docker logs: sudo journalctl -u docker -n 50\n\
             - Try manual load: gunzip -c {} | docker load",
            image_name,
            stderr.trim(),
            tar_gz_path.display(),
            tar_gz_path.display()
        ));
    }

    Ok(())
}

/// Load all Docker images from extracted payload directory
pub fn load_all_images(payload_dir: &Path) -> Result<()> {
    // Pre-flight checks
    check_docker_available()?;
    check_docker_running()?;

    let total = REQUIRED_IMAGES.len();
    println!("  Loading {} Docker images...", total);

    for (idx, (image_name, filename)) in REQUIRED_IMAGES.iter().enumerate() {
        let tar_gz_path = payload_dir.join(filename);

        if !tar_gz_path.exists() {
            return Err(eyre!("Image file not found: {}", filename));
        }

        println!("  [{}/{}] {}", idx + 1, total, image_name);
        load_image(&tar_gz_path, image_name)?;
    }

    println!("  ✓ All images loaded successfully");

    Ok(())
}

/// Verify all images are loaded correctly
#[allow(dead_code)]
pub fn verify_images_loaded() -> Result<()> {
    println!("  Verifying images...");

    for (image_name, _) in REQUIRED_IMAGES {
        if !image_exists(image_name)? {
            return Err(eyre!("Image not found after loading: {}", image_name));
        }
    }

    println!("  ✓ All images verified");
    Ok(())
}
