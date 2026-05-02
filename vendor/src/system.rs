use serde::{Deserialize, Serialize};
use sysinfo::{Disks, ProcessesToUpdate, System};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub device: String,
    pub used_space: f64,
    pub available_space: f64,
    pub total_space: f64,
    pub is_mounted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub mem: f32,
    pub user: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemData {
    pub cpu_usage: f32,
    pub cpu_cores: Vec<f32>,
    pub mem_usage: f64,
    pub total_mem: f64,
    pub swap_usage: f64,
    pub total_swap: f64,
    pub disks: Vec<DiskInfo>,
    pub processes: Vec<ProcessInfo>,
    pub net_in: u64,
    pub net_out: u64,
    pub uptime: u64,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
}

pub struct SystemMonitor {
    sys: System,
    disks: Disks,
    networks: sysinfo::Networks,
    users: sysinfo::Users,
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = sysinfo::Networks::new_with_refreshed_list();
        let users = sysinfo::Users::new_with_refreshed_list();
        Self {
            sys,
            disks,
            networks,
            users,
        }
    }

    pub fn get_data(&mut self) -> SystemData {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.disks.refresh_list();
        self.networks.refresh_list();
        self.users.refresh_list();

        let cpu_usage = self.sys.global_cpu_usage();
        let cpu_cores = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let mem_usage = self.sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0; // GB
        let total_mem = self.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0; // GB
        let swap_usage = self.sys.used_swap() as f64 / 1024.0 / 1024.0 / 1024.0; // GB
        let total_swap = self.sys.total_swap() as f64 / 1024.0 / 1024.0 / 1024.0; // GB

        let mut final_processes = Vec::new();
        for (pid, process) in self.sys.processes() {
            let user = process
                .user_id()
                .and_then(|uid| self.users.iter().find(|u| u.id() == uid))
                .map(|u| u.name().to_string())
                .unwrap_or_else(|| "root".to_string());

            final_processes.push(ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu: process.cpu_usage(),
                mem: process.memory() as f32 / 1024.0 / 1024.0, // MB
                user,
                status: format!("{:?}", process.status()),
            });
        }
        final_processes.sort_by(|a, b| {
            b.cpu
                .partial_cmp(&a.cpu)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        final_processes.truncate(200);

        let mut net_in = 0;
        let mut net_out = 0;
        for (_, data) in &self.networks {
            net_in += data.received();
            net_out += data.transmitted();
        }

        SystemData {
            cpu_usage,
            cpu_cores,
            mem_usage,
            total_mem,
            swap_usage,
            total_swap,
            disks: self.get_disk_data(),
            processes: final_processes,
            net_in,
            net_out,
            uptime: System::uptime(),
            os_name: System::name().unwrap_or_default(),
            os_version: System::os_version().unwrap_or_default(),
            kernel_version: System::kernel_version().unwrap_or_default(),
            hostname: System::host_name().unwrap_or_default(),
        }
    }
    fn get_disk_data(&mut self) -> Vec<DiskInfo> {
        let mut final_disks = Vec::new();
        // (Existing disk logic moved to helper)
        for disk in self.disks.iter() {
            let mount = disk.mount_point().to_string_lossy();
            let fs_type = disk.file_system().to_string_lossy().to_lowercase();
            let device = disk.name().to_string_lossy().to_string();

            if mount == "/" {
                final_disks.push(DiskInfo {
                    name: mount.to_string(),
                    device,
                    used_space: (disk.total_space() - disk.available_space()) as f64,
                    available_space: disk.available_space() as f64,
                    total_space: disk.total_space() as f64,
                    is_mounted: true,
                });
                continue;
            }

            let is_real_fs = fs_type.contains("ext")
                || fs_type.contains("btrfs")
                || fs_type.contains("xfs")
                || fs_type.contains("zfs")
                || fs_type.contains("vfat")
                || fs_type.contains("fat")
                || fs_type.contains("ntfs")
                || fs_type.contains("exfat")
                || fs_type.contains("fuseblk");

            let is_removable_path = mount.starts_with("/media")
                || mount.starts_with("/mnt")
                || mount.starts_with("/run/media");
            let is_system_path = (mount.starts_with("/boot")
                || mount.starts_with("/nix")
                || mount.starts_with("/run")
                || mount.starts_with("/sys")
                || mount.starts_with("/proc")
                || mount.starts_with("/dev")
                || mount.starts_with("/tmp"))
                && !is_removable_path;

            if is_real_fs
                && (is_removable_path || !is_system_path)
                && disk.total_space() > 100_000_000
            {
                final_disks.push(DiskInfo {
                    name: mount.to_string(),
                    device,
                    used_space: (disk.total_space() - disk.available_space()) as f64,
                    available_space: disk.available_space() as f64,
                    total_space: disk.total_space() as f64,
                    is_mounted: true,
                });
            }
        }

        // 2. Supplement with unmounted drives from lsblk
        if let Ok(output) = std::process::Command::new("lsblk")
            .arg("-rnbo")
            .arg("NAME,FSTYPE,SIZE,MOUNTPOINT,LABEL")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(' ').collect();
                if parts.len() >= 3 {
                    let name = parts[0];
                    let fstype = parts[1];
                    let size_str = parts[2];
                    let mountpoint = parts.get(3).cloned().unwrap_or("");
                    let label = parts.get(4).cloned().unwrap_or("");

                    if !fstype.is_empty() && mountpoint.is_empty() {
                        if let Ok(size) = size_str.parse::<f64>() {
                            if size > 100_000_000.0 {
                                let dev_path = format!("/dev/{}", name);
                                let display_name = if !label.is_empty() {
                                    label.to_string()
                                } else {
                                    let gb = size / 1_073_741_824.0;
                                    if gb >= 1.0 {
                                        format!("{:.0}G Drive", gb)
                                    } else {
                                        format!("{:.0}M Drive", size / 1_048_576.0)
                                    }
                                };

                                if fstype != "swap" && !fstype.contains("member") {
                                    final_disks.push(DiskInfo {
                                        name: display_name,
                                        device: dev_path,
                                        used_space: 0.0,
                                        available_space: size,
                                        total_space: size,
                                        is_mounted: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        final_disks
    }
}
