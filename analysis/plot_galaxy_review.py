#!/usr/bin/env python3
"""Plot the headless rotation-curve and thickness CSVs from this review.

Regenerate the CSVs with, from the repo root:

    cargo run --release -- --headless --legacy --seconds 40 --particles 60000 --seed 1 \\
        --out analysis/review_data/legacy_curve.csv --snapshot-dir /tmp/legacy_snap
    cargo run --release -- --headless --seconds 40 --particles 60000 --seed 1 \\
        --out analysis/review_data/modern_curve.csv --snapshot-dir /tmp/modern_snap
    cargo run --release -- --headless --no-mond --seconds 40 --particles 60000 --seed 1 \\
        --out analysis/review_data/vortex_only_curve.csv --snapshot-dir /tmp/vortex_snap

The CSVs under analysis/review_data/ are generated and gitignored.
Then:  python3 analysis/plot_galaxy_review.py
"""

import numpy as np
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

def load_curve(path):
    rows = []
    with open(path) as f:
        for line in f:
            if line.startswith("#") or line.startswith(" t") or not line.strip():
                continue
            rows.append([float(x) for x in line.strip().split(",")])
    return np.array(rows)


def series(data, t, col, min_n=80):
    sub = data[np.abs(data[:, 0] - t) < 0.05]
    sub = sub[sub[:, 2] >= min_n]
    return sub[:, 1], sub[:, col]


def hist2(path, plane):
    xyz = np.loadtxt(path, delimiter=",", skiprows=1)
    if plane == "xz":
        a, b = xyz[:, 0], xyz[:, 2]
        lim = 260
    else:
        a, b = xyz[:, 0], xyz[:, 1]
        lim = 80
    h, xe, ye = np.histogram2d(a, b, bins=180, range=[[-260, 260], [-lim, lim]])
    return np.log1p(h.T), lim


def main():
    leg = load_curve("analysis/review_data/legacy_curve.csv")
    mod = load_curve("analysis/review_data/modern_curve.csv")
    vor = load_curve("analysis/review_data/vortex_only_curve.csv")

    fig, ax = plt.subplots(figsize=(8.6, 5.4))
    for data, t, label, color, ls, lw in (
        (leg, 40.0, "legacy t = 40 s  (damping wins)", "#c0392b", "-", 2.0),
        (vor, 40.0, "vortex only t = 40 s  (Newtonian)", "#d68910", "-", 2.0),
        (mod, 40.0, "MOND + vortex t = 40 s", "#1a7f37", "-", 2.2),
        (mod, 0.0, "MOND + vortex t = 0", "#5dade2", "--", 1.3),
    ):
        r, v = series(data, t, 3)
        ax.plot(r, v, ls=ls, color=color, lw=lw, label=label)
    r, vn = series(leg, 0.0, 6, min_n=0)
    _, vm = series(mod, 0.0, 7, min_n=0)
    ax.plot(r, vn, color="#bbbbbb", lw=1.0, label="Plummer circular speed")
    ax.plot(
        r,
        vm,
        color="#1a5276",
        lw=1.2,
        ls=":",
        label="MOND circular speed of the core (not a target)",
    )
    ax.set_xlabel("cylindrical radius")
    ax.set_ylabel("mean tangential speed")
    ax.set_title("Rotation curve from the forces  ·  60 000 particles, seed 1")
    ax.set_xlim(0, 260)
    ax.set_ylim(0, 16)
    ax.legend(frameon=False, fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    fig.savefig("docs/img/rotation_curve.png", dpi=140)

    fig, ax = plt.subplots(figsize=(8.2, 4.6))
    for data, label, color in (
        (leg, "legacy", "#c0392b"),
        (vor, "vortex only", "#d68910"),
        (mod, "MOND + vortex", "#1a7f37"),
    ):
        ts, ys = [], []
        for t in sorted(set(np.round(data[:, 0], 3))):
            m = np.abs(data[:, 0] - t) < 0.02
            ts.append(t)
            ys.append(data[m][0, 8])
        ax.plot(ts, ys, color=color, lw=2.2, label=label)
    ax.set_xlabel("time (s)")
    ax.set_ylabel("rms height  √⟨y²⟩")
    ax.set_title("Disk thickness")
    ax.legend(frameon=False)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    fig.savefig("docs/img/thickness.png", dpi=140)

    snaps = (
        ("/tmp/legacy_snap/t_end.csv", "legacy"),
        ("/tmp/modern_snap/t_end.csv", "MOND + vortex"),
        ("/tmp/vortex_snap/t_end.csv", "vortex only"),
    )
    fig, axes = plt.subplots(2, 3, figsize=(11.2, 7.2))
    for col, (path, title) in enumerate(snaps):
        for row, plane in enumerate(("xz", "xy")):
            img, lim = hist2(path, plane)
            ax = axes[row, col]
            ax.imshow(
                img,
                origin="lower",
                extent=[-260, 260, -lim, lim],
                cmap="magma",
                aspect="equal",
            )
            ax.set_title(f"{title}  {'top-down' if plane == 'xz' else 'edge-on'}")
            ax.set_xlabel("x")
            ax.set_ylabel("z" if plane == "xz" else "y")
    fig.suptitle("t = 40 s, 60 000 particles, seed 1", fontsize=11)
    fig.tight_layout()
    fig.savefig("docs/img/morphology.png", dpi=130)
    print("wrote docs/img/rotation_curve.png, thickness.png, morphology.png")


if __name__ == "__main__":
    main()
