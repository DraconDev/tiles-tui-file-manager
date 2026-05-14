use crate::app::App;
use crate::state::DiskIo;
use dracon_system::{
    DiskSnapshot, ProcessControlContract, ProcessController, ProcessSnapshot, SystemSnapshot,
    SystemSnapshotContract, SystemSnapshotProvider,
};
use dracon_terminal_engine::system::{DiskInfo, ProcessInfo};
use std::collections::{HashMap, VecDeque};

pub struct SystemModule {
    monitor: SystemSnapshotProvider,
}

impl SystemModule {
    pub fn new() -> Self {
        Self {
            monitor: SystemSnapshotProvider::new(),
        }
    }

    pub fn get_data(&mut self) -> std::io::Result<SystemSnapshot> {
        self.monitor.capture_snapshot()
    }

    pub fn kill_process(pid: u32) -> std::io::Result<()> {
        let controller = ProcessController;
        controller.kill_process(pid, Some(9))
    }

    pub fn update_app_state(app: &mut App, data: SystemSnapshot) {
        let s = &mut app.system_state;
        s.cpu_usage = data.cpu_usage;
        s.cpu_cores = data.cpu_cores.to_vec();
        s.mem_usage = data.mem_usage as f32;
        s.total_mem = data.total_mem as f32;
        s.swap_usage = data.swap_usage as f32;
        s.total_swap = data.total_swap as f32;
        s.disks = data.disks.into_iter().map(map_disk).collect();
        s.uptime = data.uptime;
        s.processes = data.processes.into_iter().map(map_process).collect();
        s.process_ppid = read_process_ppid();
        s.hostname = data.hostname;
        s.os_name = data.os_name;
        s.os_version = data.os_version;
        s.kernel_version = data.kernel_version;

        s.cpu_history.push_back(data.cpu_usage as u64);
        if s.cpu_history.len() > 100 {
            s.cpu_history.pop_front();
        }

        if s.core_history.len() != data.cpu_cores.len() {
            s.core_history = vec![VecDeque::from(vec![0; 100]); data.cpu_cores.len()];
        }
        for (i, &usage) in data.cpu_cores.iter().enumerate() {
            s.core_history[i].push_back(usage as u64);
            if s.core_history[i].len() > 100 {
                s.core_history[i].pop_front();
            }
        }

        let mem_p = if data.total_mem > 0.0 {
            (data.mem_usage / data.total_mem) * 100.0
        } else {
            0.0
        };
        s.mem_history.push_back(mem_p as u64);
        if s.mem_history.len() > 100 {
            s.mem_history.pop_front();
        }

        let swap_p = if data.total_swap > 0.0 {
            (data.swap_usage / data.total_swap) * 100.0
        } else {
            0.0
        };
        s.swap_history.push_back(swap_p as u64);
        if s.swap_history.len() > 100 {
            s.swap_history.pop_front();
        }

        if s.last_net_in > 0 {
            let diff_in = data.net_in.saturating_sub(s.last_net_in);
            let diff_out = data.net_out.saturating_sub(s.last_net_out);
            s.net_in_history.push_back(diff_in);
            s.net_out_history.push_back(diff_out);
            if s.net_in_history.len() > 100 {
                s.net_in_history.pop_front();
            }
            if s.net_out_history.len() > 100 {
                s.net_out_history.pop_front();
            }
        }
        s.last_net_in = data.net_in;
        s.last_net_out = data.net_out;
        s.net_in = data.net_in;
        s.net_out = data.net_out;

        s.cpu_temperature = read_cpu_temperature();
        s.cpu_frequency = read_cpu_frequency();

        let mut current_disk_io = read_disk_io();
        let elapsed = s.last_update.elapsed().as_secs_f64().max(0.1);
        let mut total_read_rate: u64 = 0;
        let mut total_write_rate: u64 = 0;

        for (dev, io) in current_disk_io.iter_mut() {
            if let Some(prev) = s.last_disk_io.get(dev) {
                let dr = io.read_bytes.saturating_sub(prev.read_bytes);
                let dw = io.write_bytes.saturating_sub(prev.write_bytes);
                io.read_rate_mbps = dr as f64 / elapsed / 1_048_576.0;
                io.write_rate_mbps = dw as f64 / elapsed / 1_048_576.0;
                total_read_rate += (io.read_rate_mbps * 100.0) as u64;
                total_write_rate += (io.write_rate_mbps * 100.0) as u64;
            }
        }

        s.disk_read_history.push_back(total_read_rate);
        if s.disk_read_history.len() > 100 {
            s.disk_read_history.pop_front();
        }
        s.disk_write_history.push_back(total_write_rate);
        if s.disk_write_history.len() > 100 {
            s.disk_write_history.pop_front();
        }

        s.last_disk_io = s.disk_io.clone();
        s.disk_io = current_disk_io;

        app.apply_process_sort();
    }
}

fn read_disk_io() -> HashMap<String, DiskIo> {
    let mut io_map = HashMap::new();
    if let Ok(content) = std::fs::read_to_string("/proc/diskstats") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 14 {
                let device_name = parts[2].to_string();
                let _reads_completed: u64 = parts[3].parse().unwrap_or(0);
                let sectors_read: u64 = parts[5].parse().unwrap_or(0);
                let _writes_completed: u64 = parts[7].parse().unwrap_or(0);
                let sectors_written: u64 = parts[9].parse().unwrap_or(0);
                let sector_size = 512;
                io_map.insert(
                    device_name,
                    DiskIo {
                        read_bytes: sectors_read * sector_size,
                        write_bytes: sectors_written * sector_size,
                        read_rate_mbps: 0.0,
                        write_rate_mbps: 0.0,
                    },
                );
            }
        }
    }
    io_map
}

fn read_cpu_temperature() -> Option<f32> {
    let thermal_dirs = std::fs::read_dir("/sys/class/thermal").ok()?;
    for entry in thermal_dirs.flatten() {
        let path = entry.path();
        let zone_type = path.join("type");
        if let Ok(content) = std::fs::read_to_string(zone_type) {
            let trimmed = content.trim();
            if trimmed == "x86_pkg_temp" || trimmed == "CPU" || trimmed == "acpitz" {
                let temp_file = path.join("temp");
                if let Ok(temp_str) = std::fs::read_to_string(temp_file) {
                    if let Ok(temp_millidegrees) = temp_str.trim().parse::<i64>() {
                        return Some(temp_millidegrees as f32 / 1000.0);
                    }
                }
            }
        }
    }
    std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .ok()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .map(|t| t as f32 / 1000.0)
}

fn read_cpu_frequency() -> Option<f32> {
    let freq_path = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq";
    std::fs::read_to_string(freq_path)
        .ok()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .map(|f| f as f32 / 1000.0)
}

fn map_disk(d: DiskSnapshot) -> DiskInfo {
    DiskInfo {
        name: d.name,
        device: d.device,
        used_space: d.used_space,
        available_space: d.available_space,
        total_space: d.total_space,
        is_mounted: d.is_mounted,
    }
}

fn map_process(p: ProcessSnapshot) -> ProcessInfo {
    ProcessInfo {
        pid: p.pid,
        name: p.name,
        cpu: p.cpu,
        mem: p.mem,
        user: p.user,
        status: p.status,
    }
}

fn read_process_ppid() -> HashMap<u32, u32> {
    let mut ppid_map = HashMap::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let pid_str = name.to_string_lossy();
            if let Ok(pid) = pid_str.parse::<u32>() {
                let stat_path = std::path::PathBuf::from(format!("/proc/{}/stat", pid));
                if let Ok(stat_content) = std::fs::read_to_string(&stat_path) {
                    if let Some(ppid) = parse_ppid_from_stat(&stat_content) {
                        ppid_map.insert(pid, ppid);
                    }
                }
            }
        }
    }
    ppid_map
}

fn parse_ppid_from_stat(stat: &str) -> Option<u32> {
    let start = stat.rfind(')')?;
    let rest = &stat[start + 2..];
    let mut fields = rest.split_whitespace();
    fields.nth(1)?;
    fields.next().and_then(|s| s.parse().ok())
}

pub fn process_tree_depth(pid: u32, ppid_map: &std::collections::HashMap<u32, u32>) -> usize {
    let mut depth = 0;
    let mut current = pid;
    let mut visited = std::collections::HashSet::new();
    while let Some(&parent) = ppid_map.get(&current) {
        if parent == 0 || parent == 1 || visited.contains(&parent) {
            break;
        }
        visited.insert(parent);
        current = parent;
        depth += 1;
        if depth > 20 {
            break;
        }
    }
    depth
}

pub fn tree_sort_processes(
    processes: &mut Vec<dracon_terminal_engine::system::ProcessInfo>,
    ppid_map: &std::collections::HashMap<u32, u32>,
) {
    let pid_to_idx: std::collections::HashMap<u32, usize> = processes
        .iter()
        .enumerate()
        .map(|(i, p)| (p.pid, i))
        .collect();

    let mut children: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for p in processes.iter() {
        if let Some(&ppid) = ppid_map.get(&p.pid) {
            children.entry(ppid).or_default().push(p.pid);
        }
    }

    let mut sorted = Vec::with_capacity(processes.len());
    let mut stack: Vec<(u32, usize)> = vec![(0, 0), (1, 0)];

    while let Some((pid, _depth)) = stack.pop() {
        if let Some(&idx) = pid_to_idx.get(&pid) {
            sorted.push(processes[idx].clone());
        }
        if let Some(mut kids) = children.get(&pid).cloned() {
            kids.sort_by(|a, b| {
                let a_cpu = processes.iter().find(|p| p.pid == *a).map(|p| p.cpu).unwrap_or(0.0);
                let b_cpu = processes.iter().find(|p| p.pid == *b).map(|p| p.cpu).unwrap_or(0.0);
                b_cpu.partial_cmp(&a_cpu).unwrap_or(std::cmp::Ordering::Equal)
            });
            for k in kids.into_iter().rev() {
                stack.push((k, 0));
            }
        }
    }

    if sorted.len() == processes.len() {
        *processes = sorted;
    }
}
