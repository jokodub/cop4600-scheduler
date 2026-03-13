use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

/// Represents a process defined in the input file.
///
/// The struct stores both the static attributes provided by the input
/// configuration (arrival time and burst time) and the runtime state
/// maintained by the scheduler (remaining time and performance metrics).
#[derive(Debug, Clone)]
struct Process {
    name: String,
    arrival_time: u32,
    burst_time: u32,
    remaining_time: u32,

    // Scheduling metrics recorded during simulation
    start_time: Option<u32>,
    finish_time: Option<u32>,
    wait_time: u32,
    turnaround_time: u32,
    response_time: Option<u32>,
}

impl Process {
    /// Create a new process with runtime metrics initialized.
    fn new(name: String, arrival_time: u32, burst_time: u32) -> Self {
        Self {
            name,
            arrival_time,
            burst_time,
            remaining_time: burst_time,
            start_time: None,
            finish_time: None,
            wait_time: 0,
            turnaround_time: 0,
            response_time: None,
        }
    }
}

/// Enumeration of supported scheduling algorithms.
///
/// The enum isolates algorithm-specific behavior and avoids passing
/// around raw strings throughout the codebase.
#[derive(Debug, Clone, Copy)]
enum Algorithm {
    FCFS,
    SJF,
    RR,
}

impl Algorithm {
    /// Convert a string from the input file into an Algorithm variant.
    ///
    /// This keeps all parsing logic in one place and prevents scattered
    /// string comparisons across the codebase.
    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "fcfs" => Ok(Algorithm::FCFS),
            "sjf" => Ok(Algorithm::SJF),
            "rr" => Ok(Algorithm::RR),
            _ => Err(format!("Invalid scheduling algorithm '{}'", value)),
        }
    }

    /// Convert the enum back into its input directive string.
    /// This can be useful for serialization or debugging.
    #[allow(dead_code)]
    fn to_str(&self) -> &'static str {
        match self {
            Algorithm::FCFS => "fcfs",
            Algorithm::SJF => "sjf",
            Algorithm::RR => "rr",
        }
    }

    /// Return the human-readable algorithm name required by the assignment output.
    fn name(&self) -> &'static str {
        match self {
            Algorithm::FCFS => "First-Come First-Served",
            Algorithm::SJF => "preemptive Shortest Job First",
            Algorithm::RR => "Round-Robin",
        }
    }
}

/// Parsed configuration of the scheduler run.
///
/// This struct aggregates all directives from the input file so the
/// scheduling engine can operate without needing to know anything
/// about file parsing.
#[derive(Debug)]
struct Config {
    process_count: usize,
    run_for: u32,
    algorithm: Algorithm,
    quantum: Option<u32>,
    processes: Vec<Process>,
}

/// Convert an input filename such as `example.in` to `example.out`.
///
/// The simulator specification expects the output filename to mirror
/// the input filename with only the extension changed.
fn output_filename(input: &str) -> String {
    // Remove the ".in" suffix if it exists so the extension can be replaced
    // with ".out". If the filename does not follow the expected convention,
    // simply append ".out" so unusual filenames are still supported.
    if let Some(base) = input.strip_suffix(".in") {
        format!("{}.out", base)
    } else {
        format!("{}.out", input)
    }
}

/// Parse the scheduler input file and construct the runtime configuration.
fn parse_input(filename: &str) -> Result<Config, String> {
    let file =
        File::open(filename).map_err(|e| format!("Failed to open file '{}': {}", filename, e))?;
    let reader = BufReader::new(file);

    let mut process_count: Option<usize> = None;
    let mut run_for: Option<u32> = None;
    let mut algorithm: Option<Algorithm> = None;
    let mut quantum: Option<u32> = None;
    let mut processes: Vec<Process> = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let tokens: Vec<&str> = line.split_whitespace().collect();

        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "processcount" => {
                process_count = Some(
                    tokens[1]
                        .parse::<usize>()
                        // Mapping parse errors into readable messages prevents
                        // crashes caused by malformed configuration files.
                        .map_err(|_| "Invalid processcount value".to_string())?,
                );
            }

            "runfor" => {
                run_for = Some(
                    tokens[1]
                        .parse::<u32>()
                        .map_err(|_| "Invalid runfor value".to_string())?,
                );
            }

            "use" => {
                algorithm = Some(Algorithm::from_str(tokens[1])?);
            }

            "quantum" => {
                quantum = Some(
                    tokens[1]
                        .parse::<u32>()
                        .map_err(|_| "Invalid quantum value".to_string())?,
                );
            }

            "process" => {
                let name = tokens[2].to_string();
                let arrival = tokens[4]
                    .parse::<u32>()
                    .map_err(|_| "Invalid arrival time".to_string())?;
                let burst = tokens[6]
                    .parse::<u32>()
                    .map_err(|_| "Invalid burst time".to_string())?;

                processes.push(Process::new(name, arrival, burst));
            }

            "end" => break,
            _ => {}
        }
    }

    // Required parameter validation.
    let process_count = process_count.ok_or_else(|| "Error: Missing parameter processcount.".to_string())?;
    let run_for = run_for.ok_or_else(|| "Error: Missing parameter runfor.".to_string())?;
    let algorithm = algorithm.ok_or_else(|| "Error: Missing parameter use.".to_string())?;

    // Round Robin requires a quantum to determine how long a process
    // runs before it is preempted.
    if matches!(algorithm, Algorithm::RR) && quantum.is_none() {
        return Err("Error: Missing quantum parameter when use is 'rr'".to_string());
    }

    Ok(Config {
        process_count,
        run_for,
        algorithm,
        quantum,
        processes,
    })
}

/// Finalizes scheduling metrics when a process completes execution.
///
/// This function calculates turnaround time, wait time, and response time
/// using the timestamps recorded during simulation.
fn finalize_metrics(process: &mut Process) {
    if let (Some(start), Some(finish)) = (process.start_time, process.finish_time) {
        process.turnaround_time = finish - process.arrival_time;
        process.wait_time = process.turnaround_time - process.burst_time;
        process.response_time = Some(start - process.arrival_time);
    }
}

/// Simulates the First-Come First-Served (FCFS) CPU scheduling algorithm.
///
/// FCFS is a non-preemptive scheduler where processes are executed
/// strictly in the order they arrive. Arriving processes are placed
/// into a FIFO ready queue and the scheduler always selects the
/// process at the front of the queue when the CPU becomes available.
///
/// Execution model per tick:
/// 1. Print arrival events
/// 2. Print finish events
/// 3. Perform scheduling decisions
/// 4. Execute the running process for one time unit
///
/// A process runs until its burst time reaches zero.
fn simulate_fcfs(config: &Config, writer: &mut impl Write) -> std::io::Result<Vec<Process>> {
    let mut processes = config.processes.clone();
    let mut ready: VecDeque<usize> = VecDeque::new();
    let mut running: Option<usize> = None;
    let mut pending_finish: Option<usize> = None; // Finish events are printed at the start of the next tick.

    for time in 0..config.run_for {
        // 1) arrivals
        for (i, p) in processes.iter().enumerate() {
            if p.arrival_time == time {
                writeln!(writer, "Time {} : {} arrived", time, p.name)?;

                ready.push_back(i); // Ensures processes are executed in arrival order.
            }
        }

        // 2) finishes
        if let Some(pid) = pending_finish.take() {
            writeln!(writer, "Time {} : {} finished", time, processes[pid].name)?;
        }

        // 3) scheduling
        // A new process is only scheduled if the CPU becomes idle.
        if running.is_none() {
            if let Some(next) = ready.pop_front() {
                running = Some(next);

                if processes[next].start_time.is_none() {
                    processes[next].start_time = Some(time);
                }

                writeln!(
                    writer,
                    "Time {} : {} selected (burst {})",
                    time,
                    processes[next].name,
                    processes[next].remaining_time
                )?;
            }
        }

        // 4) execute
        if let Some(pid) = running {
            // Each loop iteration represents one unit of CPU time.
            processes[pid].remaining_time -= 1;

            if processes[pid].remaining_time == 0 {
                processes[pid].finish_time = Some(time + 1);
                finalize_metrics(&mut processes[pid]);

                pending_finish = Some(pid);
                running = None;
            }
        } else {
            writeln!(writer, "Time {} : Idle", time)?;
        }
    }

    writeln!(writer, "Finished at time {}", config.run_for)?;
    Ok(processes)
}

/// Simulates preemptive Shortest Job First scheduling (also called
/// Shortest Remaining Time First, or SRTF).
///
/// At every scheduling decision point the scheduler selects the process
/// with the smallest remaining burst time among all ready processes.
/// If a newly arrived process has a shorter remaining burst than the
/// currently running process, a preemption occurs.
///
/// Execution model per tick:
/// 1. Print arrival events
/// 2. Print finish events
/// 3. Reevaluate which process has the shortest remaining time
/// 4. Execute the chosen process for one time unit
fn simulate_sjf(config: &Config, writer: &mut impl Write) -> std::io::Result<Vec<Process>> {
    let mut processes = config.processes.clone();
    let mut ready: Vec<usize> = Vec::new();
    let mut running: Option<usize> = None;

    let mut pending_finish: Option<usize> = None;

    for time in 0..config.run_for {
        // 1) arrivals
        for (i, p) in processes.iter().enumerate() {
            if p.arrival_time == time {
                writeln!(writer, "Time {} : {} arrived", time, p.name)?;
                ready.push(i);
            }
        }

        // 2) finishes
        if let Some(pid) = pending_finish.take() {
            writeln!(writer, "Time {} : {} finished", time, processes[pid].name)?;
        }

        // 3) scheduling
        // Include running process in candidate pool
        let mut candidate_pool = ready.clone();
        if let Some(r) = running {
            candidate_pool.push(r);
        }

        if !candidate_pool.is_empty() {
            // Choose the process with the smallest remaining burst time.
            candidate_pool.sort_by_key(|&i| processes[i].remaining_time);
            let next = candidate_pool[0];

            // If the chosen process differs from the currently running one,
            // preemption occurs.
            if running != Some(next) {
                if let Some(prev) = running {
                    ready.push(prev);
                }

                ready.retain(|&x| x != next);
                running = Some(next);

                if processes[next].start_time.is_none() {
                    processes[next].start_time = Some(time);
                }

                writeln!(
                    writer,
                    "Time {} : {} selected (burst {})",
                    time,
                    processes[next].name,
                    processes[next].remaining_time
                )?;
            }
        }

        // 4) execute
        if let Some(pid) = running {
            // Each loop iteration represents one unit of CPU time.
            processes[pid].remaining_time -= 1;

            if processes[pid].remaining_time == 0 {
                processes[pid].finish_time = Some(time + 1);
                finalize_metrics(&mut processes[pid]);

                pending_finish = Some(pid);
                running = None;
            }
        } else {
            writeln!(writer, "Time {} : Idle", time)?;
        }
    }

    writeln!(writer, "Finished at time {}", config.run_for)?;
    Ok(processes)
}

/// Simulates the Round-Robin (RR) CPU scheduling algorithm.
///
/// Round-Robin uses a FIFO ready queue and assigns each running process
/// a fixed time slice called a *quantum*. When the quantum expires,
/// the process is preempted and placed at the back of the ready queue
/// if it still has remaining burst time.
///
/// Important ordering rules implemented by this simulation:
/// - Arrival events are processed before scheduling decisions
/// - Finished processes are reported before new selections
/// - New arrivals enter the ready queue before a preempted process
///   is requeued
///
/// This ordering ensures fair queue behavior and matches the
/// specification used by the reference scheduler.
fn simulate_rr(config: &Config, writer: &mut impl Write) -> std::io::Result<Vec<Process>> {
    let mut processes = config.processes.clone();
    let mut ready: VecDeque<usize> = VecDeque::new();

    let quantum = config.quantum.unwrap(); // unwrap() is safe here because earlier of validation guarantees.

    let mut running: Option<usize> = None;
    let mut quantum_remaining = quantum;
    let mut requeue: Option<usize> = None;
    let mut pending_finish: Option<usize> = None;

    for time in 0..config.run_for {
        // 1) arrivals
        for (i, p) in processes.iter().enumerate() {
            if p.arrival_time == time {
                writeln!(writer, "Time {} : {} arrived", time, p.name)?;
                ready.push_back(i); // Preserves fairness by placing newly arrived processes 
                                    // at the end of the ready queue.
            }
        }

        // 2) finishes
        if let Some(pid) = pending_finish.take() {
            writeln!(writer, "Time {} : {} finished", time, processes[pid].name)?;
        }

        // requeue after arrivals and finishes
        if let Some(pid) = requeue.take() {
            ready.push_back(pid);
        }

        // 3) scheduling
        if running.is_none() {
            // Run the next process in the queue for quantum number of ticks.
            if let Some(next) = ready.pop_front() {
                running = Some(next);
                quantum_remaining = quantum;

                if processes[next].start_time.is_none() {
                    processes[next].start_time = Some(time);
                }

                writeln!(
                    writer,
                    "Time {} : {} selected (burst {})",
                    time,
                    processes[next].name,
                    processes[next].remaining_time
                )?;
            }
        }

        // 4) execute
        if let Some(pid) = running {
            // Each loop iteration represents one unit of CPU time.
            processes[pid].remaining_time -= 1;
            quantum_remaining -= 1;

            if processes[pid].remaining_time == 0 {
                processes[pid].finish_time = Some(time + 1);
                finalize_metrics(&mut processes[pid]);

                pending_finish = Some(pid);
                running = None;
            } else if quantum_remaining == 0 {
                // When a process exhausts its quantum without finishing,
                // it is moved to the back of the ready queue so other
                // processes receive CPU time.
                requeue = Some(pid);
                running = None;
            }
        } else {
            writeln!(writer, "Time {} : Idle", time)?;
        }
    }

    writeln!(writer, "Finished at time {}", config.run_for)?;
    Ok(processes)
}

/// Writes the final statistics summary for all processes.
///
/// After the simulation finishes, this function prints one line per
/// process containing:
/// - wait time
/// - turnaround time
/// - response time
///
/// If a process did not complete within the simulation window,
/// it is reported as "did not finish".
fn print_summary(processes: &[Process], writer: &mut impl Write) -> std::io::Result<()> {
    writeln!(writer)?;

    for p in processes {
        if p.finish_time.is_some() {
            writeln!(
                writer,
                "{} wait {} turnaround {} response {}",
                p.name,
                p.wait_time,
                p.turnaround_time,
                p.response_time.unwrap_or(0)
            )?;
        } else {
            writeln!(writer, "{} did not finish", p.name)?;
        }
    }
    Ok(())
}

/// Program entry point for the scheduler simulator.
///
/// Responsibilities:
/// 1. Parse command-line arguments
/// 2. Load and validate the scheduler input file
/// 3. Create the corresponding `.out` output file
/// 4. Run the selected scheduling algorithm simulation
/// 5. Print the final metrics summary
///
/// All scheduler output is written to the `.out` file using a
/// buffered writer to improve I/O efficiency when writing
/// many small lines of simulation output.
fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Collect input file from CLI.
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: scheduler-gpt <input file>");
        return Ok(());
    }

    let input_file = &args[1];

    let config = match parse_input(input_file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            return Ok(());
        }
    };

    // Create an output file based on the input's name.
    let output_path = output_filename(input_file);

    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    // Begin simulation by printing input summary.
    writeln!(writer, "{} processes", config.process_count)?;
    writeln!(writer, "Using {}", config.algorithm.name())?;

    if matches!(config.algorithm, Algorithm::RR) {
        writeln!(writer, "Quantum {}", config.quantum.unwrap())?;
    }

    writeln!(writer)?;

    // Perform the scheduling simulation.
    let processes = match config.algorithm {
        Algorithm::FCFS => simulate_fcfs(&config, &mut writer)?,
        Algorithm::SJF => simulate_sjf(&config, &mut writer)?,
        Algorithm::RR => simulate_rr(&config, &mut writer)?,
    };

    // End by printing a summary of each process's metrics.
    print_summary(&processes, &mut writer)?;

    Ok(())
}
