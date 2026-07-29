use crate::{gpu, process::GameProcess};
use std::{io, process::Command};

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub target_process: String,
    pub dry_run: bool,
    pub set_process_priority: bool,
    pub enable_power_mode: bool,
    pub apply_gpu_profile: bool,
    pub enable_driver_upscaling: bool,
}

pub struct Optimizer {
    config: SessionConfig,
    revert_commands: Vec<CommandSpec>,
    intel_gpu_state: Option<gpu::IntelGpuState>,
}

#[derive(Clone)]
struct CommandSpec {
    program: String,
    args: Vec<String>,
}

impl Optimizer {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            revert_commands: Vec::new(),
            intel_gpu_state: None,
        }
    }

    pub fn apply(&mut self, process: &GameProcess) -> io::Result<()> {
        if self.config.enable_power_mode {
            self.apply_power_mode()?;
        }
        if self.config.set_process_priority {
            self.set_priority(process.pid)?;
        }
        if self.config.apply_gpu_profile {
            self.apply_gpu_profile()?;
        }
        if self.config.enable_driver_upscaling {
            self.enable_driver_upscaling()?;
        }
        Ok(())
    }

    pub fn revert(&mut self) -> io::Result<()> {
        while let Some(command) = self.revert_commands.pop() {
            self.run(command)?;
        }
        if let Some(ref state) = self.intel_gpu_state {
            if let Err(e) = gpu::restore_intel_gpu(state) {
                println!("⚠️  [GPU Optimizer] Failed to restore Intel iGPU frequency limits: {}", e);
            }
        }
        self.intel_gpu_state = None;
        Ok(())
    }

    fn apply_power_mode(&mut self) -> io::Result<()> {
        if cfg!(target_os = "windows") {
            self.run(CommandSpec::new("powercfg", &["/setactive", "SCHEME_MIN"]))?;
            self.revert_commands.push(CommandSpec::new(
                "powercfg",
                &["/setactive", "SCHEME_BALANCED"],
            ));
        } else if cfg!(target_os = "macos") {
            self.run(CommandSpec::new("pmset", &["-a", "lowpowermode", "0"]))?;
            self.revert_commands
                .push(CommandSpec::new("pmset", &["-a", "lowpowermode", "1"]));
        } else {
            self.run(CommandSpec::new("sh", &["-c", "command -v cpupower >/dev/null && cpupower frequency-set -g performance || true"]))?;
            self.revert_commands.push(CommandSpec::new(
                "sh",
                &[
                    "-c",
                    "command -v cpupower >/dev/null && cpupower frequency-set -g powersave || true",
                ],
            ));
        }
        Ok(())
    }

    fn set_priority(&self, pid: u32) -> io::Result<()> {
        if cfg!(target_os = "windows") {
            if let Err(e) = self.run(CommandSpec::new(
                "powershell",
                &[
                    "-NoProfile",
                    "-Command",
                    &format!("(Get-Process -Id {pid}).PriorityClass='High'"),
                ],
            )) {
                println!("⚠️  [Optimizer] Failed to set process priority on Windows: {}", e);
            }
        } else {
            // Try renice. Since standard users cannot set negative nice values (-5),
            // it may fail with permission denied. If so, print a warning instead of aborting the whole run.
            match self.run(CommandSpec::new(
                "renice",
                &["-n", "-5", "-p", &pid.to_string()],
            )) {
                Ok(_) => {
                    println!("🚀 [Optimizer] Elevated process {pid} priority (renice -5) successfully.");
                }
                Err(e) => {
                    println!("⚠️  [Optimizer] Failed to elevate priority via renice (requires root or CAP_SYS_NICE): {}", e);
                    println!("💡 [Optimizer] Continuing with standard priority levels.");
                }
            }
        }
        Ok(())
    }

    fn apply_gpu_profile(&mut self) -> io::Result<()> {
        let gpus = gpu::detect_gpus().unwrap_or_default();
        println!(
            "GPU profile step for {}: {}",
            self.config.target_process,
            gpu::recommendation_for(&gpus)
        );
        if gpus.iter().any(|gpu| gpu.vendor == gpu::GpuVendor::Intel) {
            if let Some(state) = gpu::pin_intel_gpu() {
                self.intel_gpu_state = Some(state);
            }
        }
        Ok(())
    }

    fn enable_driver_upscaling(&self) -> io::Result<()> {
        println!("Upscaling step: enable only OS/driver-exposed FSR/XeSS/NIS-style display scaling where available.");
        Ok(())
    }

    fn run(&self, command: CommandSpec) -> io::Result<()> {
        if self.config.dry_run {
            println!("dry-run: {} {}", command.program, command.args.join(" "));
            return Ok(());
        }
        let status = Command::new(&command.program)
            .args(&command.args)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("command failed: {}", command.program),
            ))
        }
    }
}

impl CommandSpec {
    fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::GameProcess;

    #[test]
    fn optimizer_respects_dry_run_and_collects_revert_commands() {
        let mut optimizer = Optimizer::new(SessionConfig {
            target_process: "TestGame".to_string(),
            dry_run: true,
            set_process_priority: true,
            enable_power_mode: true,
            apply_gpu_profile: true,
            enable_driver_upscaling: true,
        });

        let process = GameProcess {
            pid: 99999,
            name: "TestGame".to_string(),
        };

        // Applying on dry-run should succeed without running commands
        assert!(optimizer.apply(&process).is_ok());

        // In dry-run, power scheme revert command should still be collected
        assert!(!optimizer.revert_commands.is_empty());

        // Reverting on dry-run should succeed and empty the revert stack
        assert!(optimizer.revert().is_ok());
        assert!(optimizer.revert_commands.is_empty());
    }
}
