//! Headless integrator for the rotation curve and disk thickness.
//! No window, no GPU. Used by the review measurements.
//!
//! ```text
//! cargo run --release -- --headless --seconds 40 --legacy --out analysis/legacy.csv
//! ```

use crate::galaxy::{GalaxyConfig, GalaxySimulation, SnapshotStats};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

struct HeadlessArgs {
    seconds: f32,
    particles: usize,
    dt: f32,
    seed: u64,
    out: String,
    snapshot_dir: Option<String>,
    config: GalaxyConfig,
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn parse(args: &[String]) -> HeadlessArgs {
    HeadlessArgs {
        seconds: arg_value(args, "--seconds")
            .and_then(|s| s.parse().ok())
            .unwrap_or(40.0),
        particles: arg_value(args, "--particles")
            .and_then(|s| s.parse().ok())
            .unwrap_or(60_000),
        dt: arg_value(args, "--dt")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0 / 60.0),
        seed: arg_value(args, "--seed")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1),
        out: arg_value(args, "--out").unwrap_or_else(|| "galaxy_curve.csv".to_string()),
        snapshot_dir: arg_value(args, "--snapshot-dir"),
        config: GalaxyConfig::from_args(args),
    }
}

pub fn run(args: &[String]) {
    let args = parse(args);
    if let Some(parent) = Path::new(&args.out).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if let Some(dir) = &args.snapshot_dir {
        let _ = fs::create_dir_all(dir);
    }

    let mut galaxy = GalaxySimulation::with_config(args.particles, args.config, Some(args.seed));
    galaxy.verbose = false;

    println!(
        "headless  {}  particles={}  seconds={}  dt={}  seed={}",
        galaxy.config.label(),
        args.particles,
        args.seconds,
        args.dt,
        args.seed
    );

    let report_times = [0.0_f32, 5.0, 10.0, 20.0, 40.0, 60.0, 90.0, 120.0];
    let mut next_report = 1usize;
    let mut samples: Vec<SnapshotStats> = Vec::new();

    let take = |galaxy: &GalaxySimulation, samples: &mut Vec<SnapshotStats>| {
        let s = galaxy.stats();
        print_sample(&s);
        samples.push(s);
    };

    take(&galaxy, &mut samples);
    if let Some(dir) = &args.snapshot_dir {
        write_snapshot(&galaxy, &format!("{dir}/t0.csv"));
    }

    while galaxy.time() < args.seconds - 1e-4 {
        galaxy.update(args.dt);
        while next_report < report_times.len() && galaxy.time() + 1e-3 >= report_times[next_report]
        {
            if report_times[next_report] <= args.seconds + 1e-3 {
                take(&galaxy, &mut samples);
            }
            next_report += 1;
        }
    }
    // Always keep the final state even if it is not on the report grid.
    if samples
        .last()
        .map(|s| (s.t - galaxy.time()).abs() > 0.2)
        .unwrap_or(true)
    {
        take(&galaxy, &mut samples);
    }
    if let Some(dir) = &args.snapshot_dir {
        write_snapshot(&galaxy, &format!("{dir}/t_end.csv"));
    }

    write_curves(&args.out, &galaxy.config, &samples);
    println!("wrote {}", args.out);
}

fn print_sample(s: &SnapshotStats) {
    println!(
        "t={:6.2}  n={}  rms_y={:7.3}  mean|y|={:7.3}  p90|y|={:7.3}  arm_contrast={:.3}  bh_r={:6.1}  absorbed={}",
        s.t, s.n, s.rms_y, s.mean_abs_y, s.p90_abs_y, s.arm_contrast, s.bh_r, s.absorbed
    );
    println!(
        "  {:>7} {:>6} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "r", "n", "v_theta", "v_rad", "rms_y", "v_newt", "v_mond", "vt/vN"
    );
    for b in &s.bins {
        if b.count == 0 {
            continue;
        }
        let ratio = if b.v_newton > 1e-4 {
            b.v_theta / b.v_newton
        } else {
            0.0
        };
        println!(
            "  {:7.1} {:6} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3}",
            b.r_mid, b.count, b.v_theta, b.v_rad, b.rms_y, b.v_newton, b.v_mond, ratio
        );
    }
}

fn write_curves(path: &str, config: &GalaxyConfig, samples: &[SnapshotStats]) {
    let file = File::create(path).expect("create curve csv");
    let mut w = BufWriter::new(file);
    writeln!(
        w,
        "# {}\n t,r_mid,count,v_theta,v_rad,rms_y,v_newton,v_mond,disk_rms_y,mean_abs_y,p90_abs_y,arm_contrast,bh_r,absorbed",
        config.label()
    )
    .unwrap();
    for s in samples {
        for b in &s.bins {
            writeln!(
                w,
                "{:.4},{:.4},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.4},{}",
                s.t,
                b.r_mid,
                b.count,
                b.v_theta,
                b.v_rad,
                b.rms_y,
                b.v_newton,
                b.v_mond,
                s.rms_y,
                s.mean_abs_y,
                s.p90_abs_y,
                s.arm_contrast,
                s.bh_r,
                s.absorbed
            )
            .unwrap();
        }
    }
}

fn write_snapshot(galaxy: &GalaxySimulation, path: &str) {
    let file = File::create(path).expect("create snapshot");
    let mut w = BufWriter::new(file);
    writeln!(w, "x,y,z").unwrap();
    for p in &galaxy.particles {
        writeln!(w, "{:.5},{:.5},{:.5}", p.x, p.y, p.z).unwrap();
    }
}
