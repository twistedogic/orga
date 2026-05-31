use std::fs;
use std::process::Command;

use crate::error::OrgaError;

pub fn install_service(system: bool, config_path: &str) -> Result<(), OrgaError> {
    if !cfg!(target_os = "linux") {
        return Err(OrgaError::SystemdNotLinux);
    }

    if system {
        check_root()?;
    }

    let bin_path = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .map_err(|e| OrgaError::SystemdWriteFailed(format!("could not determine binary path: {e}")))?;

    let unit = generate_unit_file(bin_path.to_string_lossy().as_ref(), config_path, system);

    let target_dir = if system {
        std::path::PathBuf::from("/etc/systemd/system")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
        std::path::PathBuf::from(home).join(".config/systemd/user")
    };

    fs::create_dir_all(&target_dir)
        .map_err(|e| OrgaError::SystemdWriteFailed(format!("could not create directory {}: {e}", target_dir.display())))?;

    let unit_path = target_dir.join("orga-agent.service");
    fs::write(&unit_path, &unit)
        .map_err(|e| OrgaError::SystemdWriteFailed(format!("could not write {}: {e}", unit_path.display())))?;

    let reload_ok = reload_daemon(system);

    println!("Wrote: {}", unit_path.display());
    if !reload_ok {
        eprintln!("warning: daemon-reload failed or systemctl not found; run manually:");
        if system {
            eprintln!("  sudo systemctl daemon-reload");
        } else {
            eprintln!("  systemctl --user daemon-reload");
        }
    }

    println!("\nTo enable and start the service:");
    if system {
        println!("  sudo systemctl enable orga-agent");
        println!("  sudo systemctl start orga-agent");
    } else {
        println!("  systemctl --user enable orga-agent");
        println!("  systemctl --user start orga-agent");
    }

    Ok(())
}

pub fn generate_unit_file(bin_path: &str, config_path: &str, system: bool) -> String {
    let wanted_by = if system { "multi-user.target" } else { "default.target" };
    format!(
        "[Unit]\n\
         Description=orga agent\n\
         After=network.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart={bin_path} --config {config_path} agent\n\
         Restart=on-failure\n\
         RestartSec=30\n\
         \n\
         [Install]\n\
         WantedBy={wanted_by}\n"
    )
}

fn check_root() -> Result<(), OrgaError> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|_| OrgaError::SystemdRootRequired)?;
    let uid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uid_str != "0" {
        return Err(OrgaError::SystemdRootRequired);
    }
    Ok(())
}

fn reload_daemon(system: bool) -> bool {
    let mut cmd = Command::new("systemctl");
    if !system {
        cmd.arg("--user");
    }
    cmd.arg("daemon-reload");
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unit_file_user() {
        let unit = generate_unit_file("/usr/local/bin/orga", "/home/user/.orga/config.toml", false);
        assert!(unit.contains("ExecStart=/usr/local/bin/orga --config /home/user/.orga/config.toml agent"));
        assert!(unit.contains("WantedBy=default.target"));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("RestartSec=30"));
    }

    #[test]
    fn test_generate_unit_file_system() {
        let unit = generate_unit_file("/usr/local/bin/orga", "/etc/orga/config.toml", true);
        assert!(unit.contains("WantedBy=multi-user.target"));
    }
}
