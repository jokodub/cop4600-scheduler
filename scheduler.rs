use std::collections::VecDeque;
use std::env;
use std::fs;

#[derive(Clone)]
struct Process {
    name: String,
    arrival: usize,
    burst: usize,
    remaining: usize,
    start_time: Option<usize>,
    finish_time: Option<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum Algorithm {
    FCFS,
    RR,
    SJF,
}

struct Config {
    process_count: usize,
    run_for: usize,
    algorithm: Algorithm,
    quantum: usize,
    processes: Vec<Process>,
}

fn parse_input(path: &str) -> Config {
    let content = fs::read_to_string(path).expect("failed to read input");

    let mut process_count = 0;
    let mut run_for = 0;
    let mut algorithm = Algorithm::FCFS;
    let mut quantum = 1;
    let mut processes = Vec::new();

    for line in content.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "processcount" => process_count = tokens[1].parse().unwrap(),
            "runfor" => run_for = tokens[1].parse().unwrap(),

            "use" => {
                algorithm = match tokens[1] {
                    "fcfs" => Algorithm::FCFS,
                    "rr" => Algorithm::RR,
                    "sjf" => Algorithm::SJF,
                    _ => panic!("unknown algorithm"),
                }
            }

            "quantum" => quantum = tokens[1].parse().unwrap(),

            "process" => {
                let name = tokens[2].to_string();
                let arrival = tokens[4].parse().unwrap();
                let burst = tokens[6].parse().unwrap();

                processes.push(Process {
                    name,
                    arrival,
                    burst,
                    remaining: burst,
                    start_time: None,
                    finish_time: None,
                });
            }

            "end" => break,

            _ => {}
        }
    }

    Config {
        process_count,
        run_for,
        algorithm,
        quantum,
        processes,
    }
}

fn print_header(cfg: &Config) {
    println!("{} processes", cfg.process_count);

    match cfg.algorithm {
        Algorithm::FCFS => println!("Using First-Come First-Served"),
        Algorithm::RR => {
            println!("Using Round-Robin");
            println!("Quantum {}", cfg.quantum);
            println!();
        }
        Algorithm::SJF => println!("Using preemptive Shortest Job First"),
    }
}

fn simulate(mut cfg: Config) {
    let mut ready: VecDeque<usize> = VecDeque::new();
    let mut current: Option<usize> = None;

    let mut quantum_left = cfg.quantum;
    let mut just_selected = false;
    let mut sjf_event = false;

    for time in 0..cfg.run_for {
        // ----------------
        // ARRIVALS
        // ----------------
        for i in 0..cfg.processes.len() {
            if cfg.processes[i].arrival == time {
                println!("Time {:>3} : {} arrived", time, cfg.processes[i].name);

                ready.push_back(i);

                if cfg.algorithm == Algorithm::SJF {
                    sjf_event = true;
                }
            }
        }

        // ----------------
        // RUN CURRENT
        // ----------------
        if let Some(p) = current {
            let proc = &mut cfg.processes[p];
            proc.remaining -= 1;

            if proc.remaining == 0 {
                proc.finish_time = Some(time);

                println!("Time {:>3} : {} finished", time, proc.name);

                current = None;

                if cfg.algorithm == Algorithm::SJF {
                    sjf_event = true;
                }
            } else if cfg.algorithm == Algorithm::RR {
                quantum_left -= 1;

                if quantum_left == 0 {
                    ready.push_back(p);
                    current = None;
                }
            }
        }

        // ----------------
        // SJF SELECTION
        // ----------------
        if cfg.algorithm == Algorithm::SJF {
            if sjf_event {
                // Determine the shortest job in ready + current
                let mut candidates: Vec<usize> = ready.iter().copied().collect();
                if let Some(p) = current {
                    candidates.push(p);
                }

                if let Some(&pid) = candidates
                    .iter()
                    .min_by_key(|&&i| cfg.processes[i].remaining)
                {
                    if Some(pid) != current {
                        // Only change selection if new process is shorter
                        if let Some(c) = current {
                            ready.push_back(c);
                        }

                        // Remove from ready queue
                        ready.retain(|&x| x != pid);

                        current = Some(pid);
                        just_selected = true;
                    }
                }

                sjf_event = false;
            }
        }

        // ----------------
        // FCFS / RR SELECTION
        // ----------------
        if current.is_none() {
            match cfg.algorithm {
                Algorithm::FCFS => {
                    if !ready.is_empty() {
                        current = Some(ready.pop_front().unwrap());
                        just_selected = true;
                    }
                }

                Algorithm::RR => {
                    if !ready.is_empty() {
                        current = Some(ready.pop_front().unwrap());
                        quantum_left = cfg.quantum;
                        just_selected = true;
                    }
                }

                _ => {}
            }
        }

        // ----------------
        // PRINT SELECTION
        // ----------------
        if let Some(p) = current {
            let proc = &mut cfg.processes[p];

            if proc.start_time.is_none() {
                proc.start_time = Some(time);
            }

            if proc.start_time == Some(time) || just_selected {
                println!(
                    "Time {:>3} : {} selected (burst {:>3})",
                    time, proc.name, proc.remaining
                );
            }
        } else {
            println!("Time {:>3} : Idle", time);
        }

        just_selected = false;
    }

    println!("Finished at time {:>3}\n", cfg.run_for);

    for p in &cfg.processes {
        if let Some(finish) = p.finish_time {
            let turnaround = finish - p.arrival;
            let wait = turnaround - p.burst;
            let response = p.start_time.unwrap() - p.arrival;

            println!(
                "{} wait {:>3} turnaround {:>3} response {}",
                p.name, wait, turnaround, response
            );
        } else {
            println!("{} did not finish", p.name);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: scheduler <input_file>");
        return;
    }

    let cfg = parse_input(&args[1]);

    print_header(&cfg);

    simulate(cfg);
}
