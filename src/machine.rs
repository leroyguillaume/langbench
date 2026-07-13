//! Everything about the machine under the benchmark that could plausibly
//! explain a difference between two campaigns.
//!
//! Collection is best-effort: every field is optional, because most of it comes
//! from `/proc` and `/sys` and does not exist outside Linux. A campaign run on a
//! non-Linux host is not a valid measurement — see `METHODOLOGY.md#where-it-runs`
//! — but the harness must still run there for development.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread::available_parallelism;

use serde::{Deserialize, Serialize};
use tracing::debug;

/// architecture extensions worth recording. Anything else is noise in a report.
const NOTABLE_FLAGS: &[&str] = &[
    "sse2", "avx", "avx2", "avx512f", "fma", "neon", "asimd", "sve", "sve2", "bf16",
];

/// DMI vendors that only ever appear under a hypervisor.
const VIRTUAL_DMI_VENDORS: &[&str] = &[
    "QEMU",
    "VMware",
    "Xen",
    "Microsoft Corporation",
    "Amazon EC2",
    "Google",
    "innotek GmbH",
    "Parallels",
    "Apple Inc.",
];

/// Kernel release markers left by the usual desktop virtualization stacks.
const VIRTUAL_KERNEL_MARKERS: &[&str] = &["linuxkit", "orbstack", "microsoft-standard", "wsl"];

/// A `label: value` pair for the human-facing report.
#[derive(Clone, Debug, Serialize)]
pub struct Field {
    pub label: String,
    pub value: String,
}

/// Deserializable so a report can be re-rendered from a `samples.ndjson` written
/// on another host: the machine in the report is the one that measured, never the
/// one that formatted.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Machine {
    /// A Linux `/proc` is visible. Determined at **runtime**: a
    /// `cfg!(target_os = "linux")` would be a tautology inside a container,
    /// where the binary is always a Linux binary whatever the host may be.
    pub linux: bool,
    /// Evidence that this is a virtual machine, if any was found. `None` proves
    /// nothing: it means no marker was detected, not that the host is metal.
    pub virtualization: Option<String>,
    /// The harness itself is running inside a container.
    pub containerized: bool,
    pub architecture: String,
    pub hostname: Option<String>,
    pub os: Option<String>,
    pub kernel: Option<String>,

    pub cpu_model: Option<String>,
    pub cpu_vendor: Option<String>,
    pub logical_cpus: Option<usize>,
    pub physical_cores: Option<usize>,
    pub available_parallelism: Option<usize>,
    pub smt_active: Option<bool>,
    pub isa_extensions: Vec<String>,
    pub numa_nodes: Option<usize>,

    pub scaling_governor: Option<String>,
    pub scaling_min_khz: Option<u64>,
    pub scaling_max_khz: Option<u64>,
    pub scaling_cur_khz: Option<u64>,
    /// `true` means turbo/boost is off, which is what a benchmark host wants.
    pub turbo_disabled: Option<bool>,
    pub isolated_cpus: Option<String>,

    pub memory_total_kb: Option<u64>,
    pub load_average: Option<[f64; 3]>,

    pub cgroup_version: Option<u8>,
    pub docker_version: Option<String>,
    pub docker_storage_driver: Option<String>,
}

impl Machine {
    pub fn collect() -> Self {
        let cpu = CpuInfo::collect();
        let kernel = read_trim("/proc/sys/kernel/osrelease");
        let machine = Self {
            linux: Path::new("/proc/cpuinfo").is_file(),
            virtualization: virtualization(
                kernel.as_deref(),
                cpu.hypervisor_flag,
                read_trim("/sys/hypervisor/type").as_deref(),
                read_trim("/sys/class/dmi/id/sys_vendor").as_deref(),
            ),
            containerized: containerized(),
            architecture: std::env::consts::ARCH.to_owned(),
            hostname: read_trim("/proc/sys/kernel/hostname").or_else(|| read_trim("/etc/hostname")),
            os: read_trim("/proc/sys/kernel/ostype"),
            kernel,

            cpu_model: cpu.model,
            cpu_vendor: cpu.vendor,
            logical_cpus: cpu.logical_cpus,
            physical_cores: cpu.physical_cores,
            isa_extensions: cpu.isa_extensions,
            available_parallelism: available_parallelism().map(|n| n.get()).ok(),
            smt_active: read_bool("/sys/devices/system/cpu/smt/active"),
            numa_nodes: count_numa_nodes(),

            scaling_governor: read_trim("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
            scaling_min_khz: read_u64("/sys/devices/system/cpu/cpu0/cpufreq/scaling_min_freq"),
            scaling_max_khz: read_u64("/sys/devices/system/cpu/cpu0/cpufreq/scaling_max_freq"),
            scaling_cur_khz: read_u64("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq"),
            turbo_disabled: turbo_disabled(),
            isolated_cpus: read_trim("/sys/devices/system/cpu/isolated"),

            memory_total_kb: memory_total_kb(),
            load_average: load_average(),

            cgroup_version: cgroup_version(),
            docker_version: docker_query(&["version", "--format", "{{.Server.Version}}"]),
            docker_storage_driver: docker_query(&["info", "--format", "{{.Driver}}"]),
        };

        // The caller decides how to surface `warnings()`; here we only record.
        debug!(?machine, "collected machine metadata");
        machine
    }

    /// Ordered `label: value` pairs for the report. `None` renders as `n/a`
    /// rather than being dropped: a missing field is information.
    pub fn fields(&self) -> Vec<Field> {
        let mut fields = Vec::new();
        let mut push = |label: &str, value: String| {
            fields.push(Field {
                label: label.to_owned(),
                value,
            });
        };

        push("Hostname", opt(self.hostname.as_deref()));
        // Never render a blank cell: an empty value is indistinguishable from a
        // rendering bug, whereas `n/a` is a fact about the host.
        push(
            "Architecture",
            opt((!self.architecture.is_empty()).then_some(self.architecture.as_str())),
        );
        push("OS", opt(self.os.as_deref()));
        push("Kernel", opt(self.kernel.as_deref()));
        push("Virtualization", self.virtualization_field());
        push("Harness containerized", self.containerized.to_string());
        push("CPU model", opt(self.cpu_model.as_deref()));
        push("CPU vendor", opt(self.cpu_vendor.as_deref()));
        push("Logical CPUs", opt_num(self.logical_cpus));
        push("Physical cores", opt_num(self.physical_cores));
        push("available_parallelism", opt_num(self.available_parallelism));
        push("SMT active", opt_bool(self.smt_active));
        push("NUMA nodes", opt_num(self.numa_nodes));
        push(
            "architecture extensions",
            if self.isa_extensions.is_empty() {
                "n/a".to_owned()
            } else {
                self.isa_extensions.join(", ")
            },
        );
        push("Scaling governor", opt(self.scaling_governor.as_deref()));
        push("Frequency min", opt_mhz(self.scaling_min_khz));
        push("Frequency max", opt_mhz(self.scaling_max_khz));
        push("Frequency at start", opt_mhz(self.scaling_cur_khz));
        push("Turbo disabled", opt_bool(self.turbo_disabled));
        push("Isolated CPUs", opt(self.isolated_cpus.as_deref()));
        push(
            "Memory",
            self.memory_total_kb
                .map(|kb| format!("{:.1} GiB", kb as f64 / 1024.0 / 1024.0))
                .unwrap_or_else(|| "n/a".to_owned()),
        );
        push(
            "Load average at start",
            self.load_average
                .map(|[a, b, c]| format!("{a:.2}, {b:.2}, {c:.2}"))
                .unwrap_or_else(|| "n/a".to_owned()),
        );
        push("cgroup version", opt_num(self.cgroup_version));
        push("Docker version", opt(self.docker_version.as_deref()));
        push(
            "Docker storage driver",
            opt(self.docker_storage_driver.as_deref()),
        );
        fields
    }

    /// What the hypervisor probes actually established.
    ///
    /// Every probe reads a Linux path (`/sys/hypervisor`, DMI, the kernel
    /// release, the CPU flags). Off Linux none of them exist, so the answer is
    /// `None` because we could not look — not because we looked and found
    /// nothing. Rendering that as "none detected" would state the opposite of
    /// the warning printed above the very same table, on a host where the
    /// containers demonstrably run inside a VM.
    fn virtualization_field(&self) -> String {
        match (&self.virtualization, self.linux) {
            (Some(evidence), _) => evidence.clone(),
            (None, true) => "none detected".to_owned(),
            (None, false) => "unknown (the probes need Linux; see the warning above)".to_owned(),
        }
    }

    /// A two-column table plus the host's warnings, for `langbench machine`.
    ///
    /// This is program output, not a diagnostic: the caller prints it on stdout.
    pub fn console_report(&self) -> String {
        let fields = self.fields();
        let width = fields
            .iter()
            .map(|field| field.label.chars().count())
            .chain(["PROPERTY".len()])
            .max()
            .unwrap_or_default();
        let value_width = fields
            .iter()
            .map(|field| field.value.chars().count())
            .chain(["VALUE".len()])
            .max()
            .unwrap_or_default();

        let mut out = String::new();
        let _ = writeln!(out, "{:<width$}   VALUE", "PROPERTY");
        let _ = writeln!(out, "{}   {}", "-".repeat(width), "-".repeat(value_width));
        for field in &fields {
            let _ = writeln!(out, "{:<width$}   {}", field.label, field.value);
        }

        let warnings = self.warnings();
        if warnings.is_empty() {
            let _ = writeln!(out, "\nNo warning: this host looks like a usable target.");
        } else {
            let _ = writeln!(out, "\n{} warning(s):", warnings.len());
            for warning in &warnings {
                let _ = writeln!(out, "  ! {warning}");
            }
        }
        out
    }

    /// Reasons this host is a poor benchmark target, in plain English.
    ///
    /// Rendered at the top of the report. A campaign with warnings is not
    /// invalid, but its numbers do not support percentage-level claims.
    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if !self.linux {
            warnings.push(
                "No Linux `/proc` is visible. Containers run inside a VM, so timings measure the \
                 hypervisor as much as the backend. Development only."
                    .to_owned(),
            );
        }
        if let Some(evidence) = &self.virtualization {
            warnings.push(format!(
                "Running under a hypervisor ({evidence}). Timings measure the VM's vCPU \
                 scheduling as much as the backend, and the host may throttle or migrate \
                 the guest at any moment.",
            ));
        }
        if self.containerized {
            warnings.push(
                "The harness itself runs in a container. Machine metadata describes what the \
                 container can see, which is not always the host: pass `--hostname` and check \
                 the CPU fields before trusting them."
                    .to_owned(),
            );
        }
        if self.turbo_disabled == Some(false) {
            warnings.push(
                "Turbo/boost is enabled. Early rounds run at a higher clock than late ones; \
                 that is drift, not noise, and no median rescues it."
                    .to_owned(),
            );
        }
        if let Some(governor) = &self.scaling_governor
            && governor != "performance"
        {
            warnings.push(format!(
                "CPU governor is `{governor}`, not `performance`. Frequency will vary with load.",
            ));
        }
        if self.smt_active == Some(true) {
            warnings.push(
                "SMT is active. Sibling threads share execution units, so a neighbour can halve \
                 this benchmark's IPC."
                    .to_owned(),
            );
        }
        warnings
    }
}

/// What `/proc/cpuinfo` has to say. Parsed once, in one place.
#[derive(Debug, Default, PartialEq)]
struct CpuInfo {
    model: Option<String>,
    vendor: Option<String>,
    logical_cpus: Option<usize>,
    physical_cores: Option<usize>,
    isa_extensions: Vec<String>,
    /// x86 sets this flag whenever the CPU is virtualized. AArch64 never does.
    hypervisor_flag: bool,
}

impl CpuInfo {
    fn collect() -> Self {
        read_to_string("/proc/cpuinfo").map_or_else(Self::default, |raw| Self::parse(&raw))
    }

    fn parse(raw: &str) -> Self {
        let mut info = Self::default();
        let mut logical = 0usize;
        let mut cores = BTreeSet::new();
        let mut physical_id = None;

        for line in raw.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            match key {
                "processor" => logical += 1,
                "physical id" => physical_id = Some(value.to_owned()),
                "core id" => {
                    cores.insert((physical_id.clone().unwrap_or_default(), value.to_owned()));
                }
                "model name" | "Model Name" | "Hardware" if info.model.is_none() => {
                    info.model = Some(value.to_owned());
                }
                "vendor_id" | "CPU implementer" if info.vendor.is_none() => {
                    info.vendor = Some(value.to_owned());
                }
                // `flags` on x86, `Features` on AArch64.
                "flags" | "Features" if info.isa_extensions.is_empty() => {
                    let flags: Vec<&str> = value.split_whitespace().collect();
                    info.hypervisor_flag |= flags.contains(&"hypervisor");
                    info.isa_extensions = flags
                        .into_iter()
                        .filter(|flag| NOTABLE_FLAGS.contains(flag))
                        .map(str::to_owned)
                        .collect();
                }
                _ => {}
            }
        }

        info.logical_cpus = (logical > 0).then_some(logical);
        info.physical_cores = (!cores.is_empty()).then_some(cores.len());
        info
    }
}

/// Evidence that we are inside a VM, or `None` if no marker was found.
///
/// `None` does not prove bare metal — nothing short of physical access does. It
/// only means every cheap check came back empty.
fn virtualization(
    kernel: Option<&str>,
    hypervisor_flag: bool,
    hypervisor_type: Option<&str>,
    dmi_vendor: Option<&str>,
) -> Option<String> {
    if hypervisor_flag {
        return Some("hypervisor CPU flag".to_owned());
    }
    if let Some(kind) = hypervisor_type {
        return Some(format!("/sys/hypervisor/type is `{kind}`"));
    }
    if let Some(vendor) = dmi_vendor
        && VIRTUAL_DMI_VENDORS.contains(&vendor)
    {
        return Some(format!("DMI vendor is `{vendor}`"));
    }
    let release = kernel?.to_ascii_lowercase();
    VIRTUAL_KERNEL_MARKERS
        .iter()
        .find(|marker| release.contains(*marker))
        .map(|marker| format!("kernel release contains `{marker}`"))
}

/// Docker leaves `/.dockerenv`; Podman leaves `/run/.containerenv`.
fn containerized() -> bool {
    Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists()
}

fn opt(value: Option<&str>) -> String {
    value.unwrap_or("n/a").to_owned()
}

fn opt_num<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |v| v.to_string())
}

fn opt_bool(value: Option<bool>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |v| v.to_string())
}

fn opt_mhz(khz: Option<u64>) -> String {
    khz.map_or_else(|| "n/a".to_owned(), |v| format!("{} MHz", v / 1000))
}

fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    read_to_string(path)
        .map(|raw| raw.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    read_trim(path)?.parse().ok()
}

fn read_bool(path: impl AsRef<Path>) -> Option<bool> {
    match read_trim(path)?.as_str() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

/// Intel exposes `no_turbo` (1 = off); AMD exposes `boost` (1 = on).
fn turbo_disabled() -> Option<bool> {
    if let Some(no_turbo) = read_bool("/sys/devices/system/cpu/intel_pstate/no_turbo") {
        return Some(no_turbo);
    }
    read_bool("/sys/devices/system/cpu/cpufreq/boost").map(|boost| !boost)
}

fn count_numa_nodes() -> Option<usize> {
    let count = fs::read_dir("/sys/devices/system/node")
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("node"))
        .count();
    (count > 0).then_some(count)
}

fn memory_total_kb() -> Option<u64> {
    read_to_string("/proc/meminfo")?
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn load_average() -> Option<[f64; 3]> {
    let raw = read_to_string("/proc/loadavg")?;
    let mut values = raw.split_whitespace();
    let mut next = || values.next()?.parse::<f64>().ok();
    Some([next()?, next()?, next()?])
}

fn cgroup_version() -> Option<u8> {
    if Path::new("/sys/fs/cgroup/cgroup.controllers").exists() {
        Some(2)
    } else if Path::new("/sys/fs/cgroup/memory").exists() {
        Some(1)
    } else {
        None
    }
}

fn docker_query(args: &[&str]) -> Option<String> {
    let output = Command::new("docker").args(args).output().ok()?;
    if !output.status.success() {
        debug!(?args, "docker query failed");
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_values_render_as_not_available() {
        let machine = Machine::default();
        let fields = machine.fields();
        let hostname = fields.iter().find(|f| f.label == "Hostname").unwrap();
        assert_eq!(hostname.value, "n/a");
    }

    #[test]
    fn a_non_linux_host_is_flagged() {
        let machine = Machine::default();
        assert!(machine.warnings().iter().any(|w| w.contains("Linux")));
    }

    fn virtualization_of(machine: &Machine) -> String {
        machine
            .fields()
            .into_iter()
            .find(|field| field.label == "Virtualization")
            .expect("the machine table has a Virtualization row")
            .value
    }

    /// Every hypervisor probe reads a Linux path. Off Linux we did not look, and
    /// saying "none detected" would contradict the warning printed above the very
    /// same table — on a host whose containers demonstrably run inside a VM.
    #[test]
    fn a_host_we_cannot_probe_reports_unknown_not_none() {
        let machine = Machine {
            linux: false,
            virtualization: None,
            ..Machine::default()
        };
        assert!(virtualization_of(&machine).starts_with("unknown"));
    }

    #[test]
    fn a_linux_host_with_no_hypervisor_evidence_reports_none_detected() {
        let machine = Machine {
            linux: true,
            virtualization: None,
            ..Machine::default()
        };
        assert_eq!(virtualization_of(&machine), "none detected");
    }

    #[test]
    fn evidence_of_a_hypervisor_is_reported_verbatim() {
        let machine = Machine {
            linux: true,
            virtualization: Some("DMI vendor is `QEMU`".to_owned()),
            ..Machine::default()
        };
        assert_eq!(virtualization_of(&machine), "DMI vendor is `QEMU`");
    }

    #[test]
    fn the_console_report_aligns_every_label_and_lists_the_warnings() {
        let machine = Machine {
            linux: true,
            virtualization: Some("DMI vendor is `QEMU`".to_owned()),
            hostname: Some("bench-01".to_owned()),
            ..Machine::default()
        };
        let report = machine.console_report();

        // Every value must begin exactly under the header's VALUE column.
        let value_column = report.lines().next().unwrap().find("VALUE").unwrap();
        let rows = report.lines().skip(2).take_while(|line| !line.is_empty());
        for row in rows {
            let chars: Vec<char> = row.chars().collect();
            assert_eq!(chars[value_column - 1], ' ', "row is padded: {row:?}");
            assert_ne!(chars[value_column], ' ', "value starts here: {row:?}");
        }

        assert!(report.contains("bench-01"));
        assert!(report.contains("1 warning(s):"));
        assert!(report.contains("! Running under a hypervisor (DMI vendor is `QEMU`)"));
    }

    #[test]
    fn the_console_report_says_so_when_nothing_is_wrong() {
        let machine = Machine {
            linux: true,
            turbo_disabled: Some(true),
            scaling_governor: Some("performance".to_owned()),
            smt_active: Some(false),
            ..Machine::default()
        };
        let report = machine.console_report();
        assert!(report.contains("No warning"));
        assert!(!report.contains("warning(s):"));
    }

    #[test]
    fn a_virtual_machine_is_flagged_even_when_proc_looks_healthy() {
        // The regression that motivated this: inside a container the binary is
        // always a Linux binary, so a compile-time check would stay silent on
        // exactly the host that most needs the warning.
        let machine = Machine {
            linux: true,
            virtualization: Some("kernel release contains `orbstack`".to_owned()),
            ..Machine::default()
        };
        let warnings = machine.warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("hypervisor"));
    }

    #[test]
    fn a_containerized_harness_is_flagged() {
        let machine = Machine {
            linux: true,
            containerized: true,
            ..Machine::default()
        };
        assert!(machine.warnings().iter().any(|w| w.contains("container")));
    }

    #[test]
    fn the_x86_hypervisor_flag_wins_over_every_other_signal() {
        let evidence = virtualization(Some("6.1.0-generic"), true, None, None).unwrap();
        assert_eq!(evidence, "hypervisor CPU flag");
    }

    #[test]
    fn a_virtual_dmi_vendor_is_evidence() {
        let evidence = virtualization(Some("6.1.0-generic"), false, None, Some("QEMU")).unwrap();
        assert!(evidence.contains("QEMU"));
    }

    #[test]
    fn a_desktop_virtualization_kernel_is_evidence() {
        let evidence = virtualization(Some("7.0.5-orbstack-00330-ge3df4e19"), false, None, None);
        assert!(evidence.unwrap().contains("orbstack"));
        let evidence = virtualization(Some("6.6.12-linuxkit"), false, None, None);
        assert!(evidence.unwrap().contains("linuxkit"));
    }

    #[test]
    fn a_bare_metal_host_yields_no_evidence() {
        assert_eq!(
            virtualization(Some("6.8.0-45-generic"), false, None, Some("Dell Inc.")),
            None,
        );
    }

    #[test]
    fn cpuinfo_parses_an_x86_block_and_spots_the_hypervisor_flag() {
        let info = CpuInfo::parse(
            "processor\t: 0\n\
             vendor_id\t: GenuineIntel\n\
             model name\t: Intel(R) Xeon(R) Platinum 8370C\n\
             physical id\t: 0\n\
             core id\t\t: 0\n\
             flags\t\t: fpu sse2 avx2 fma hypervisor\n\
             \n\
             processor\t: 1\n\
             physical id\t: 0\n\
             core id\t\t: 0\n",
        );
        assert_eq!(info.logical_cpus, Some(2));
        assert_eq!(info.physical_cores, Some(1));
        assert!(info.hypervisor_flag);
        assert_eq!(info.isa_extensions, ["sse2", "avx2", "fma"]);
        assert_eq!(info.vendor.as_deref(), Some("GenuineIntel"));
    }

    #[test]
    fn cpuinfo_parses_an_aarch64_block_which_names_no_model_and_no_cores() {
        let info = CpuInfo::parse(
            "processor\t: 0\n\
             Features\t: fp asimd bf16\n\
             CPU implementer\t: 0x61\n\
             \n\
             processor\t: 1\n\
             Features\t: fp asimd bf16\n",
        );
        assert_eq!(info.logical_cpus, Some(2));
        assert_eq!(info.physical_cores, None);
        assert!(!info.hypervisor_flag);
        assert_eq!(info.isa_extensions, ["asimd", "bf16"]);
        assert_eq!(info.model, None);
    }

    #[test]
    fn an_absent_cpuinfo_yields_an_empty_description() {
        assert_eq!(CpuInfo::parse(""), CpuInfo::default());
    }

    #[test]
    fn enabled_turbo_and_a_lazy_governor_are_flagged() {
        let machine = Machine {
            linux: true,
            turbo_disabled: Some(false),
            scaling_governor: Some("powersave".to_owned()),
            smt_active: Some(true),
            ..Machine::default()
        };
        let warnings = machine.warnings();
        assert_eq!(warnings.len(), 3);
        assert!(warnings.iter().any(|w| w.contains("Turbo")));
        assert!(warnings.iter().any(|w| w.contains("powersave")));
        assert!(warnings.iter().any(|w| w.contains("SMT")));
    }

    #[test]
    fn a_properly_prepared_host_raises_nothing() {
        let machine = Machine {
            linux: true,
            virtualization: None,
            containerized: false,
            turbo_disabled: Some(true),
            scaling_governor: Some("performance".to_owned()),
            smt_active: Some(false),
            ..Machine::default()
        };
        assert!(machine.warnings().is_empty());
    }
}
