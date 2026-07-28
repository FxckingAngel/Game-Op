use std::{io, process::Command};

#[derive(Debug, Clone)]
pub struct GameProcess {
    pub pid: u32,
    pub name: String,
}

pub struct ProcessWatcher {
    target: String,
}

impl ProcessWatcher {
    pub fn new(target: String) -> Self {
        Self { target }
    }

    pub fn find(&self) -> io::Result<Option<GameProcess>> {
        self.processes().map(|processes| {
            processes.into_iter().find(|process| {
                // Extract only the file name of the executable (splitting by UNIX / and Windows \ paths)
                let file_name = process.name
                    .split('/')
                    .last()
                    .unwrap_or(&process.name)
                    .split('\\')
                    .last()
                    .unwrap_or(&process.name);
                
                let file_name_lower = file_name.to_ascii_lowercase();
                let target_lower = self.target.to_ascii_lowercase();
                
                // Exclude Steam launcher, Proton wrappers, and our own proxy/optimizer to prevent false-positives
                if file_name_lower.contains("steam-launch-wrapper")
                    || file_name_lower.contains("reaper")
                    || file_name_lower.contains("pressure-vessel")
                    || file_name_lower.contains("proton waitforexitandrun")
                    || file_name_lower.contains("mitmdump")
                    || file_name_lower.contains("key_sniffer")
                    || file_name_lower.contains("game-op")
                    || file_name_lower.contains("launch")
                    || file_name_lower.contains("protected")
                    || file_name_lower.contains("crash")
                {
                    return false;
                }

                file_name_lower == target_lower
                    || file_name_lower == target_lower.trim_end_matches(".exe")
                    || (file_name_lower.contains(&target_lower) 
                        && !file_name_lower.contains("launch") 
                        && !file_name_lower.contains("protected")
                        && !file_name_lower.contains("crash"))
            })
        })
    }

    pub fn is_running(&self, pid: u32) -> io::Result<bool> {
        Ok(self
            .processes()?
            .into_iter()
            .any(|process| process.pid == pid))
    }

    fn processes(&self) -> io::Result<Vec<GameProcess>> {
        let our_pid = std::process::id();
        if cfg!(target_os = "windows") {
            parse_tasklist(&run("tasklist", &["/FO", "CSV", "/NH"])?)
                .map_err(invalid_data)
                .map(|list| list.into_iter().filter(|p| p.pid != our_pid).collect())
        } else if cfg!(target_os = "linux") {
            let mut list = Vec::new();
            for entry in std::fs::read_dir("/proc")? {
                let entry = entry?;
                let path = entry.path();
                if let Some(pid) = path.file_name()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    if pid == our_pid {
                        continue;
                    }
                    if let Ok(cmdline_bytes) = std::fs::read(path.join("cmdline")) {
                        if !cmdline_bytes.is_empty() {
                            let cmdline = cmdline_bytes.iter()
                                .map(|&b| if b == 0 { b' ' } else { b })
                                .collect::<Vec<_>>();
                            if let Ok(name) = String::from_utf8(cmdline) {
                                let name_trimmed = name.trim().to_string();
                                if !name_trimmed.contains("<defunct>") {
                                    list.push(GameProcess {
                                        pid,
                                        name: name_trimmed,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Ok(list)
        } else {
            parse_ps(&run("ps", &["-eo", "pid=,args="])?)
                .map_err(invalid_data)
                .map(|list| {
                    list.into_iter()
                        .filter(|p| p.pid != our_pid && !p.name.contains("<defunct>"))
                        .collect()
                })
        }
    }
}

fn run(command: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new(command).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

fn parse_ps(output: &str) -> Result<Vec<GameProcess>, String> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.trim().splitn(2, char::is_whitespace);
            let pid = parts
                .next()
                .ok_or_else(|| "missing pid".to_string())?
                .parse::<u32>()
                .map_err(|error| error.to_string())?;
            let name = parts.next().unwrap_or_default().trim().to_string();
            Ok(GameProcess { pid, name })
        })
        .collect()
}

fn parse_tasklist(output: &str) -> Result<Vec<GameProcess>, String> {
    output
        .lines()
        .filter_map(|line| {
            let columns = parse_csv_line(line);
            if columns.len() < 2 {
                return None;
            }
            let pid = columns[1].parse::<u32>().ok()?;
            Some(GameProcess {
                pid,
                name: columns[0].clone(),
            })
        })
        .collect::<Vec<_>>()
        .pipe(Ok)
}

fn parse_csv_line(line: &str) -> Vec<String> {
    line.trim_matches('"')
        .split("\",\"")
        .map(|value| value.to_string())
        .collect()
}

fn invalid_data(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_linux_ps_output() {
        let output = "  1234 VRChat.exe --verbose\n  5678 /usr/bin/bash\n";
        let processes = parse_ps(output).unwrap();
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0].pid, 1234);
        assert_eq!(processes[0].name, "VRChat.exe --verbose");
        assert_eq!(processes[1].pid, 5678);
        assert_eq!(processes[1].name, "/usr/bin/bash");
    }

    #[test]
    fn parses_windows_tasklist_csv() {
        let output = "\"System Idle Process\",\"0\",\"Services\",\"0\",\"24 K\"\n\"VRChat.exe\",\"1234\",\"Console\",\"1\",\"2,048,128 K\"\n";
        let processes = parse_tasklist(output).unwrap();
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0].pid, 0);
        assert_eq!(processes[0].name, "System Idle Process");
        assert_eq!(processes[1].pid, 1234);
        assert_eq!(processes[1].name, "VRChat.exe");
    }
}

