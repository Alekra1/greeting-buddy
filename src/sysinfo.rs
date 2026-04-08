use std::fs;
use std::net::UdpSocket;
use std::process::Command;

pub struct SysInfo {
    pub os: String,
    pub kernel: String,
    pub local_ip: String,
    pub packages: String,
}

impl SysInfo {
    pub fn gather() -> Self {
        SysInfo {
            os: read_os(),
            kernel: read_kernel(),
            local_ip: read_local_ip(),
            packages: read_packages(),
        }
    }
}

fn read_os() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

fn read_kernel() -> String {
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "?".to_string())
}

fn read_local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .ok()
        .and_then(|s| {
            s.connect("8.8.8.8:80").ok()?;
            s.local_addr().ok()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn read_packages() -> String {
    // Arch Linux
    if let Ok(out) = Command::new("pacman").args(["-Q"]).output() {
        if out.status.success() {
            let n = String::from_utf8_lossy(&out.stdout).lines().count();
            if n > 0 {
                return n.to_string();
            }
        }
    }
    // Debian / Ubuntu
    if let Ok(out) = Command::new("dpkg-query").args(["-l"]).output() {
        if out.status.success() {
            let n = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| l.starts_with("ii"))
                .count();
            if n > 0 {
                return n.to_string();
            }
        }
    }
    "?".to_string()
}
