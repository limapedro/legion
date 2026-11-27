//! Launch-timing harness: launch <runs> -- <program> [args...]
//! Uses std::process::Command with null stdio to keep harness overhead low.
use std::process::{Command, Stdio};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let sep = args.iter().position(|s| s == "--").expect("usage: launch <runs> -- <prog> [args]");
    let runs: usize = args[0].parse().expect("runs must be an integer");
    let prog = &args[sep + 1];
    let rest = &args[sep + 2..];
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let t0 = Instant::now();
        let status = Command::new(prog)
            .args(rest)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("spawn failed");
        assert!(status.success());
        samples.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = samples[samples.len() / 2];
    println!(
        "launch runs={} prog={} median_ms={:.3} min_ms={:.3} p90_ms={:.3}",
        runs,
        prog,
        median,
        samples[0],
        samples[(samples.len() * 9) / 10]
    );
}
