use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System};

const START_TOLERANCE: u64 = 2;

fn with_process<T>(pid: u32, f: impl FnOnce(&Process) -> T) -> Option<T> {
    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::new(),
    );
    system
        .process(pid)
        .filter(|p| !matches!(p.status(), ProcessStatus::Zombie | ProcessStatus::Dead))
        .map(f)
}

pub fn start_time(pid: u32) -> Option<u64> {
    with_process(pid, |p| p.start_time())
}

pub fn is_alive(pid: u32, proc_start: u64) -> bool {
    start_time(pid).is_some_and(|start| start.abs_diff(proc_start) <= START_TOLERANCE)
}

pub fn kill(pid: u32, proc_start: u64) -> bool {
    with_process(pid, |p| {
        p.start_time().abs_diff(proc_start) <= START_TOLERANCE && p.kill()
    })
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_own_process_and_rejects_wrong_start_time() {
        let pid = std::process::id();
        let start = start_time(pid).expect("自己的程序一定存在");
        assert!(is_alive(pid, start));
        assert!(!is_alive(pid, start.saturating_sub(3600)));
        assert!(!kill(pid, start.saturating_sub(3600)));
    }
}
