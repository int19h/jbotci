use bityzba::{invariant, requires};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use crate::CliStatus;

#[derive(Debug)]
#[invariant(true)]
pub(crate) struct BenchmarkMeasurement {
    iterations: NonZeroUsize,
    started_at: Instant,
    resource_start: ProcessResourceUsage,
    iteration_wall_times: Vec<Duration>,
    status_counts: BenchmarkStatusCounts,
}

impl BenchmarkMeasurement {
    #[requires(true)]
    #[ensures(ret.iteration_wall_times.is_empty())]
    pub(crate) fn start(iterations: NonZeroUsize) -> Self {
        Self {
            iterations,
            started_at: Instant::now(),
            resource_start: sample_process_resource_usage(),
            iteration_wall_times: Vec::with_capacity(iterations.get()),
            status_counts: BenchmarkStatusCounts::default(),
        }
    }

    #[requires(self.iteration_wall_times.len() < self.iterations.get())]
    #[ensures(self.iteration_wall_times.len() == old(self.iteration_wall_times.len()) + 1)]
    pub(crate) fn record_iteration(&mut self, wall_time: Duration, status: CliStatus) {
        self.iteration_wall_times.push(wall_time);
        self.status_counts.record(status);
    }

    #[requires(self.iteration_wall_times.len() == self.iterations.get())]
    #[ensures(ret.iterations.get() > 0)]
    #[ensures(!ret.iteration_wall_times.is_empty())]
    pub(crate) fn finish(self) -> BenchmarkReport {
        let total_wall_time = self.started_at.elapsed();
        BenchmarkReport {
            iterations: self.iterations,
            total_wall_time,
            iteration_wall_times: self.iteration_wall_times,
            status_counts: self.status_counts,
            resource_start: self.resource_start,
            resource_end: sample_process_resource_usage(),
        }
    }
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub(crate) struct BenchmarkReport {
    iterations: NonZeroUsize,
    total_wall_time: Duration,
    iteration_wall_times: Vec<Duration>,
    status_counts: BenchmarkStatusCounts,
    resource_start: ProcessResourceUsage,
    resource_end: ProcessResourceUsage,
}

impl BenchmarkReport {
    #[requires(!self.iteration_wall_times.is_empty())]
    #[ensures(!ret.is_empty())]
    pub(crate) fn render(&self) -> String {
        let wall_time = wall_time_stats(&self.iteration_wall_times, self.total_wall_time);
        let resources = ProcessResourceDelta::between(self.resource_start, self.resource_end);
        let mut output = String::new();
        output.push_str("benchmark:\n");
        output.push_str(&format!("  iterations: {}\n", self.iterations));
        output.push_str(&format!(
            "  statuses: success={} failure={} valid-missing={} invalid-input={}\n",
            self.status_counts.success,
            self.status_counts.failure,
            self.status_counts.valid_missing,
            self.status_counts.invalid_input
        ));
        output.push_str(&format!(
            "  wall: total={} mean={} median={} min={} p95={} max={} throughput={}\n",
            format_duration_ms(wall_time.total),
            format_duration_ms(wall_time.mean),
            format_duration_ms(wall_time.median),
            format_duration_ms(wall_time.min),
            format_duration_ms(wall_time.p95),
            format_duration_ms(wall_time.max),
            format_throughput(wall_time.throughput)
        ));
        output.push_str(&format!(
            "  cpu: {}\n",
            format_cpu(&resources, wall_time.total)
        ));
        output.push_str(&format!("  memory: {}\n", format_memory(&resources)));
        output.push_str(&format!(
            "  page-faults: {}\n",
            format_page_faults(&resources)
        ));
        output.push_str(&format!(
            "  context-switches: {}\n",
            format_context_switches(&resources)
        ));
        output.push_str(&format!("  block-io: {}\n", format_block_io(&resources)));
        output
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn final_status(&self) -> CliStatus {
        self.status_counts.final_status()
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[invariant(true)]
pub(crate) struct BenchmarkStatusCounts {
    success: usize,
    failure: usize,
    valid_missing: usize,
    invalid_input: usize,
}

impl BenchmarkStatusCounts {
    #[requires(true)]
    #[ensures(self.success + self.failure + self.valid_missing + self.invalid_input == old(self.success + self.failure + self.valid_missing + self.invalid_input) + 1)]
    fn record(&mut self, status: CliStatus) {
        match status {
            CliStatus::Success => self.success += 1,
            CliStatus::Failure => self.failure += 1,
            CliStatus::ValidMissing => self.valid_missing += 1,
            CliStatus::InvalidInput => self.invalid_input += 1,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn final_status(&self) -> CliStatus {
        if self.invalid_input > 0 {
            CliStatus::InvalidInput
        } else if self.valid_missing > 0 {
            CliStatus::ValidMissing
        } else if self.failure > 0 {
            CliStatus::Failure
        } else {
            CliStatus::Success
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct WallTimeStats {
    total: Duration,
    mean: Duration,
    median: Duration,
    min: Duration,
    p95: Duration,
    max: Duration,
    throughput: Option<f64>,
}

#[requires(!iteration_wall_times.is_empty())]
#[ensures(true)]
fn wall_time_stats(iteration_wall_times: &[Duration], total_wall_time: Duration) -> WallTimeStats {
    let mut sorted = iteration_wall_times.to_vec();
    sorted.sort();
    let count = sorted.len();
    let min = sorted[0];
    let max = sorted[count - 1];
    let mean = duration_from_nanos_saturating(
        sorted
            .iter()
            .map(|duration| duration.as_nanos())
            .sum::<u128>()
            / count as u128,
    );
    let median = if count % 2 == 0 {
        let right = sorted[count / 2].as_nanos();
        let left = sorted[count / 2 - 1].as_nanos();
        duration_from_nanos_saturating((left + right) / 2)
    } else {
        sorted[count / 2]
    };
    let p95_index = count.saturating_mul(95).div_ceil(100).saturating_sub(1);
    let throughput =
        (total_wall_time > Duration::ZERO).then_some(count as f64 / total_wall_time.as_secs_f64());
    WallTimeStats {
        total: total_wall_time,
        mean,
        median,
        min,
        p95: sorted[p95_index],
        max,
        throughput,
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[invariant(true)]
struct ProcessResourceUsage {
    user_cpu: Option<Duration>,
    system_cpu: Option<Duration>,
    peak_rss_bytes: Option<u64>,
    minor_page_faults: Option<u64>,
    major_page_faults: Option<u64>,
    voluntary_context_switches: Option<u64>,
    involuntary_context_switches: Option<u64>,
    block_input_ops: Option<u64>,
    block_output_ops: Option<u64>,
}

impl ProcessResourceUsage {
    #[requires(true)]
    #[ensures(ret.user_cpu.is_none())]
    #[ensures(ret.system_cpu.is_none())]
    fn unavailable() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct ProcessResourceDelta {
    user_cpu: Option<Duration>,
    system_cpu: Option<Duration>,
    peak_rss_bytes: Option<u64>,
    minor_page_faults: Option<u64>,
    major_page_faults: Option<u64>,
    voluntary_context_switches: Option<u64>,
    involuntary_context_switches: Option<u64>,
    block_input_ops: Option<u64>,
    block_output_ops: Option<u64>,
}

impl ProcessResourceDelta {
    #[requires(true)]
    #[ensures(true)]
    fn between(start: ProcessResourceUsage, end: ProcessResourceUsage) -> Self {
        Self {
            user_cpu: duration_delta(start.user_cpu, end.user_cpu),
            system_cpu: duration_delta(start.system_cpu, end.system_cpu),
            peak_rss_bytes: end.peak_rss_bytes,
            minor_page_faults: counter_delta(start.minor_page_faults, end.minor_page_faults),
            major_page_faults: counter_delta(start.major_page_faults, end.major_page_faults),
            voluntary_context_switches: counter_delta(
                start.voluntary_context_switches,
                end.voluntary_context_switches,
            ),
            involuntary_context_switches: counter_delta(
                start.involuntary_context_switches,
                end.involuntary_context_switches,
            ),
            block_input_ops: counter_delta(start.block_input_ops, end.block_input_ops),
            block_output_ops: counter_delta(start.block_output_ops, end.block_output_ops),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn duration_delta(start: Option<Duration>, end: Option<Duration>) -> Option<Duration> {
    Some(end?.saturating_sub(start?))
}

#[requires(true)]
#[ensures(true)]
fn counter_delta(start: Option<u64>, end: Option<u64>) -> Option<u64> {
    Some(end?.saturating_sub(start?))
}

#[cfg(unix)]
#[requires(true)]
#[ensures(true)]
fn sample_process_resource_usage() -> ProcessResourceUsage {
    use std::mem::MaybeUninit;

    let mut usage = MaybeUninit::<libc::rusage>::uninit();
    // getrusage initializes the rusage buffer when it returns 0.
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return ProcessResourceUsage::unavailable();
    }
    let usage = unsafe { usage.assume_init() };
    ProcessResourceUsage {
        user_cpu: duration_from_timeval(usage.ru_utime),
        system_cpu: duration_from_timeval(usage.ru_stime),
        peak_rss_bytes: peak_rss_bytes_from_ru_maxrss(usage.ru_maxrss),
        minor_page_faults: nonnegative_counter(usage.ru_minflt),
        major_page_faults: nonnegative_counter(usage.ru_majflt),
        voluntary_context_switches: nonnegative_counter(usage.ru_nvcsw),
        involuntary_context_switches: nonnegative_counter(usage.ru_nivcsw),
        block_input_ops: nonnegative_counter(usage.ru_inblock),
        block_output_ops: nonnegative_counter(usage.ru_oublock),
    }
}

#[cfg(windows)]
#[requires(true)]
#[ensures(true)]
fn sample_process_resource_usage() -> ProcessResourceUsage {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    let process = unsafe { GetCurrentProcess() };
    let mut creation_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let times_available = unsafe {
        GetProcessTimes(
            process,
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        )
    } != 0;

    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..PROCESS_MEMORY_COUNTERS::default()
    };
    let memory_available =
        unsafe { K32GetProcessMemoryInfo(process, &mut counters, counters.cb) } != 0;

    ProcessResourceUsage {
        user_cpu: times_available.then_some(duration_from_filetime(user_time)),
        system_cpu: times_available.then_some(duration_from_filetime(kernel_time)),
        peak_rss_bytes: memory_available
            .then_some(counters.PeakWorkingSetSize)
            .and_then(|bytes| u64::try_from(bytes).ok()),
        minor_page_faults: None,
        major_page_faults: None,
        voluntary_context_switches: None,
        involuntary_context_switches: None,
        block_input_ops: None,
        block_output_ops: None,
    }
}

#[cfg(not(any(unix, windows)))]
#[requires(true)]
#[ensures(true)]
fn sample_process_resource_usage() -> ProcessResourceUsage {
    ProcessResourceUsage::unavailable()
}

#[cfg(unix)]
#[requires(true)]
#[ensures(true)]
fn duration_from_timeval(value: libc::timeval) -> Option<Duration> {
    let seconds = u64::try_from(value.tv_sec).ok()?;
    let microseconds = u32::try_from(value.tv_usec).ok()?;
    if microseconds >= 1_000_000 {
        return None;
    }
    Some(Duration::new(seconds, microseconds * 1_000))
}

#[cfg(windows)]
#[requires(true)]
#[ensures(true)]
fn duration_from_filetime(value: windows_sys::Win32::Foundation::FILETIME) -> Duration {
    let ticks = (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime);
    duration_from_nanos_saturating(u128::from(ticks) * 100)
}

#[cfg(unix)]
#[requires(true)]
#[ensures(true)]
fn nonnegative_counter<T>(value: T) -> Option<u64>
where
    u64: TryFrom<T>,
{
    u64::try_from(value).ok()
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
#[requires(true)]
#[ensures(true)]
fn peak_rss_bytes_from_ru_maxrss(value: libc::c_long) -> Option<u64> {
    u64::try_from(value).ok()
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
#[requires(true)]
#[ensures(true)]
fn peak_rss_bytes_from_ru_maxrss(value: libc::c_long) -> Option<u64> {
    u64::try_from(value)
        .ok()
        .and_then(|kib| kib.checked_mul(1024))
}

#[requires(true)]
#[ensures(true)]
fn duration_from_nanos_saturating(nanos: u128) -> Duration {
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_duration_ms(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_throughput(throughput: Option<f64>) -> String {
    throughput
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.1} iter/s"))
        .unwrap_or_else(|| "unavailable".to_owned())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_cpu(resources: &ProcessResourceDelta, total_wall_time: Duration) -> String {
    let Some(user_cpu) = resources.user_cpu else {
        return "unavailable".to_owned();
    };
    let Some(system_cpu) = resources.system_cpu else {
        return "unavailable".to_owned();
    };
    let total_cpu = user_cpu.saturating_add(system_cpu);
    let utilization = (total_wall_time > Duration::ZERO)
        .then_some(total_cpu.as_secs_f64() / total_wall_time.as_secs_f64() * 100.0);
    format!(
        "total={} user={} system={} utilization={}",
        format_duration_ms(total_cpu),
        format_duration_ms(user_cpu),
        format_duration_ms(system_cpu),
        utilization
            .filter(|value| value.is_finite())
            .map(|value| format!("{value:.1}%"))
            .unwrap_or_else(|| "unavailable".to_owned())
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_memory(resources: &ProcessResourceDelta) -> String {
    resources
        .peak_rss_bytes
        .map(|bytes| format!("peak-rss={}", format_bytes(bytes)))
        .unwrap_or_else(|| "unavailable".to_owned())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_page_faults(resources: &ProcessResourceDelta) -> String {
    match (resources.minor_page_faults, resources.major_page_faults) {
        (Some(minor), Some(major)) => format!("minor={minor} major={major}"),
        _ => "unavailable".to_owned(),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_context_switches(resources: &ProcessResourceDelta) -> String {
    match (
        resources.voluntary_context_switches,
        resources.involuntary_context_switches,
    ) {
        (Some(voluntary), Some(involuntary)) => {
            format!("voluntary={voluntary} involuntary={involuntary}")
        }
        _ => "unavailable".to_owned(),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_block_io(resources: &ProcessResourceDelta) -> String {
    match (resources.block_input_ops, resources.block_output_ops) {
        (Some(input), Some(output)) => format!("input={input} output={output}"),
        _ => "unavailable".to_owned(),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_bytes(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
}
