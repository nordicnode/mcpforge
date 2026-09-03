use std::process::Command;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RuntimeCapabilities {
    pub has_npx: bool,
    pub has_uvx: bool,
    pub has_bunx: bool,
    pub has_docker: bool,
    pub has_gh: bool,
}

impl Default for RuntimeCapabilities {
    fn default() -> Self {
        Self::detect()
    }
}

impl RuntimeCapabilities {
    pub fn detect() -> Self {
        Self {
            has_npx: Self::check_binary("npx"),
            has_uvx: Self::check_binary("uvx"),
            has_bunx: Self::check_binary("bunx"),
            has_docker: Self::check_docker(),
            has_gh: Self::check_binary("gh"),
        }
    }

    fn check_binary(bin: &str) -> bool {
        if bin.contains('/') {
            std::path::Path::new(bin).exists()
        } else {
            std::env::var_os("PATH").is_some_and(|paths| {
                std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file())
            })
        }
    }

    fn check_docker() -> bool {
        if !Self::check_binary("docker") {
            return false;
        }
        Command::new("docker")
            .arg("info")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn validate_command(&self, command: &str) -> Result<(), String> {
        match command {
            "npx" => {
                if !self.has_npx && !self.has_bunx {
                    Err("Neither 'npx' nor 'bunx' was found on PATH. Please install Node.js or Bun.".to_string())
                } else {
                    Ok(())
                }
            }
            "uvx" => {
                if !self.has_uvx {
                    Err("'uvx' was not found on PATH. Please install uv (curl -LsSf https://astral.sh/uv/install.sh | sh).".to_string())
                } else {
                    Ok(())
                }
            }
            "docker" => {
                if !self.has_docker {
                    Err(
                        "Docker is either not installed or the Docker daemon is not running."
                            .to_string(),
                    )
                } else {
                    Ok(())
                }
            }
            custom => {
                if Self::check_binary(custom) {
                    Ok(())
                } else {
                    Err(format!("Binary '{}' not found on PATH.", custom))
                }
            }
        }
    }
}
