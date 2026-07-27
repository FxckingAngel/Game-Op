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
                process.name.eq_ignore_ascii_case(&self.target)
                    || process
                        .name
                        .to_ascii_lowercase()
                        .contains(&self.target.to_ascii_lowercase())
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
        if cfg!(target_os = "windows") {
            parse_tasklist(&run("tasklist", &["/FO", "CSV", "/NH"])?).map_err(invalid_data)
        } else {
            parse_ps(&run("ps", &["-eo", "pid=,comm="])?).map_err(invalid_data)
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
