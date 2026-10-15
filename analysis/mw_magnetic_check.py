#!/usr/bin/env python3
"""
Milky Way magnetic-support check for the QDM-NS / "Galaxies as EM vortices" idea.

Question: can the observed Galactic magnetic field supply the extra inward
(centripetal) force that the visible baryons cannot, i.e. replace a dark halo?

Inputs (all from published literature):
  * Baryonic mass model: McMillan 2017, MNRAS 465, 76 (arXiv:1608.00971), Table 1/2
      thin disc  Sigma0=896 Msun/pc^2, Rd=2.50 kpc, zd=0.30 kpc
      thick disc Sigma0=183 Msun/pc^2, Rd=3.02 kpc, zd=0.90 kpc
      HI  disc   Sigma0=53.1 Msun/pc^2, Rd=7 kpc,  Rm=4 kpc,  zd=0.085 kpc
      H2  disc   Sigma0=2180 Msun/pc^2, Rd=1.5 kpc, Rm=12 kpc, zd=0.045 kpc
      bulge      rho0=98.4 Msun/pc^3, r0=0.075 kpc, rcut=2.1 kpc, alpha=1.8, q=0.5
  * Observed rotation curve: Eilers et al. 2019, ApJ 871, 120 (arXiv:1810.09466)
      v_c(R_sun)=229 km/s, slope -1.7 km/s/kpc for 5<=R<=25 kpc.
      We also run a strictly flat 229 km/s curve (what the poster assumes).
  * Observed total field: Beck & Wielebinski 2013 / Beck 2001 (Space Sci Rev 99, 243)
      B_tot = 6 +/- 2 uG near the Sun, 10 +/- 3 uG at R = 3 kpc,
      radial exponential scale length ~12 kpc. Inner-Galaxy ~10 uG (Beck 2015 review).

Method:
  Delta_a(R) = [v_obs^2 - v_bar^2]/R        (acceleration baryons can't provide)
  f_req      = rho_gas(R, z=0) * Delta_a     (force density needed on the gas)
  Max magnetic force density from a field B varying on a scale L:
      f_B ~ B^2/(mu0 L)    ->   B_req = sqrt(mu0 * rho_gas * Delta_a * L)
  L = 1 kpc (very steep gradients, generous), 3 kpc (task default),
  L = R (hoop stress of a purely toroidal field = the "Z-pinch" geometry).
  Magnetic forces act only on the (partly ionised) gas; stars are collisionless
  and uncharged, so we also show what B would be needed if the field somehow
  had to hold up ALL baryons (stars+gas) - physically it cannot.
"""
import os
import numpy as np
from scipy.special import i0, i1, k0, k1, ellipk
from scipy.integrate import quad

HERE = os.path.dirname(os.path.abspath(__file__))

# ---------------- constants (SI) ----------------
G    = 6.67430e-11
MSUN = 1.98892e30
PC   = 3.085677581e16
KPC  = 1e3 * PC
MU0  = 4e-7 * np.pi          # 1.2566e-6 T m / A
KMS  = 1e3
uG   = 1e-10                 # 1 microgauss = 1e-10 tesla
SIG  = MSUN / PC**2          # Msun/pc^2 -> kg/m^2
RHO  = MSUN / PC**3          # Msun/pc^3 -> kg/m^3

# ---------------- McMillan 2017 baryons ----------------
def freeman_v2(R, Sigma0, Rd):
    """Razor-thin exponential disc circular speed^2 (Freeman 1970). R,Rd in kpc."""
    y = R / (2 * Rd)
    return 4 * np.pi * G * Sigma0 * SIG * (Rd * KPC) * y**2 * (i0(y) * k0(y) - i1(y) * k1(y))

def sigma_gas(R, Sigma0, Rd, Rm):
    R = np.maximum(R, 1e-3)
    return Sigma0 * np.exp(-Rm / R - R / Rd)       # Msun/pc^2

def ring_v2(R_eval, sigma_fn, eps=0.1, Rmax=80.0, n=40000):
    """Thin-disc circular speed^2 for arbitrary Sigma(R) by summing ring potentials
    (softened by eps kpc ~ disc thickness), v^2 = R dPhi/dR."""
    Rp = np.linspace(1e-3, Rmax, n)
    dRp = Rp[1] - Rp[0]
    S = sigma_fn(Rp) * SIG
    def phi(R):
        d2 = (R + Rp)**2 + eps**2
        m = 4 * R * Rp / d2
        return -G * np.sum(S * Rp * KPC * 4 * ellipk(m) / np.sqrt(d2 * KPC**2) * dRp * KPC)
    out = []
    for R in np.atleast_1d(R_eval):
        h = 0.01
        dphi = (phi(R + h) - phi(R - h)) / (2 * h * KPC)
        out.append(R * KPC * dphi)
    return np.array(out)

def bulge_v2(R):
    """McMillan 2017 bulge, treated as spherical with the oblate (q=0.5) volume factor."""
    rho0, r0, rcut, alpha, q = 98.4, 0.075, 2.1, 1.8, 0.5
    def M(r):
        f = lambda s: 4 * np.pi * s**2 * rho0 / (1 + s / r0)**alpha * np.exp(-(s / rcut)**2)
        return q * quad(f, 0, r, limit=200)[0] * 1e9  # kpc^3 * Msun/pc^3 -> Msun
    return np.array([G * M(r) * MSUN / (r * KPC) for r in np.atleast_1d(R)])

HI = dict(Sigma0=53.1, Rd=7.0, Rm=4.0, zd=0.085)
H2 = dict(Sigma0=2180.0, Rd=1.5, Rm=12.0, zd=0.045)
THIN = dict(Sigma0=896.0, Rd=2.50, zd=0.30)
THICK = dict(Sigma0=183.0, Rd=3.02, zd=0.90)

def v_bar2(R):
    R = np.atleast_1d(R).astype(float)
    v2 = freeman_v2(R, THIN["Sigma0"], THIN["Rd"]) + freeman_v2(R, THICK["Sigma0"], THICK["Rd"])
    v2 += ring_v2(R, lambda x: sigma_gas(x, HI["Sigma0"], HI["Rd"], HI["Rm"]))
    v2 += ring_v2(R, lambda x: sigma_gas(x, H2["Sigma0"], H2["Rd"], H2["Rm"]))
    v2 += bulge_v2(R)
    return v2

def rho_gas_mid(R):
    """Midplane gas density (kg/m^3): McMillan sech^2 discs, rho0 = Sigma/(4 zd)."""
    s = 0.0
    for d in (HI, H2):
        s = s + sigma_gas(R, d["Sigma0"], d["Rd"], d["Rm"]) / (4 * d["zd"] * 1e3)  # Msun/pc^3
    return s * RHO

def rho_gas_mid_flared(R):
    """Same, but HI midplane density reduced by the observed outer-disc flaring
    (Kalberla & Kerp 2009 ARA&A 47, 27: HWHM h(R) = 0.15 kpc exp[(R-R_sun)/9.8 kpc], 5-35 kpc).
    Same surface density, thicker layer -> lower midplane density (only applied for R > R_sun)."""
    R = np.atleast_1d(R).astype(float)
    flare = np.where(R > 8.2, np.exp(-(R - 8.2) / 9.8), 1.0)
    hi = sigma_gas(R, HI["Sigma0"], HI["Rd"], HI["Rm"]) / (4 * HI["zd"] * 1e3) * flare
    h2 = sigma_gas(R, H2["Sigma0"], H2["Rd"], H2["Rm"]) / (4 * H2["zd"] * 1e3)
    return (hi + h2) * RHO

def rho_star_mid(R):
    s = 0.0
    for d in (THIN, THICK):
        s = s + d["Sigma0"] * np.exp(-R / d["Rd"]) / (2 * d["zd"] * 1e3)
    return s * RHO

def sigma_all(R):
    return (sigma_gas(R, **{k: HI[k] for k in ("Sigma0", "Rd", "Rm")})
            + sigma_gas(R, **{k: H2[k] for k in ("Sigma0", "Rd", "Rm")})
            + THIN["Sigma0"] * np.exp(-R / THIN["Rd"]) + THICK["Sigma0"] * np.exp(-R / THICK["Rd"]))

def sigma_gas_tot(R):
    return (sigma_gas(R, **{k: HI[k] for k in ("Sigma0", "Rd", "Rm")})
            + sigma_gas(R, **{k: H2[k] for k in ("Sigma0", "Rd", "Rm")}))

# ---------------- observations ----------------
R_SUN = 8.2
def v_obs_flat(R):   return 229.0 * KMS * np.ones_like(np.atleast_1d(R), dtype=float)
def v_obs_eilers(R): return (229.0 - 1.7 * (np.atleast_1d(R) - 8.122)) * KMS
def B_obs(R, B_sun=6.0, L_B=12.0):
    return B_sun * np.exp(-(np.atleast_1d(R) - R_SUN) / L_B) * uG   # tesla

def B_req(R, v_obs_fn, L_kpc, rho_fn=rho_gas_mid):
    R = np.atleast_1d(R).astype(float)
    da = (v_obs_fn(R)**2 - v_bar2(R)) / (R * KPC)
    L = (R if L_kpc == "R" else np.full_like(R, L_kpc)) * KPC
    return np.sqrt(np.maximum(MU0 * rho_fn(R) * da * L, 0)), da

def fmt(x, p=2): return f"{x:.{p}g}"

if __name__ == "__main__":
    print("=" * 78)
    print("Milky Way magnetic-support check (SI units; 1 uG = 1e-10 T)")
    print("=" * 78)

    # sanity: baryonic model
    tot_bulge = bulge_v2(np.array([50.0]))[0] * 50 * KPC / G / MSUN
    print(f"Bulge mass check: {tot_bulge:.3g} Msun (McMillan 2017 quotes 9.23e9)")
    print(f"Freeman-vs-ring sanity at 8.2 kpc (thin disc): "
          f"{np.sqrt(freeman_v2(8.2, 896, 2.5))/KMS:.1f} vs "
          f"{np.sqrt(ring_v2(8.2, lambda x: 896*np.exp(-x/2.5), eps=0.01)[0])/KMS:.1f} km/s")

    radii = np.array([8.2, 15.0, 25.0])
    vb = np.sqrt(v_bar2(radii)) / KMS
    print("\nRadius  v_bar   v_obs(flat) v_obs(Eilers)  Delta_a(flat)   rho_gas(mid)      B_obs")
    print("[kpc]   [km/s]  [km/s]      [km/s]         [m/s^2]         [Msun/pc^3]      [uG]")
    for R, v in zip(radii, vb):
        da = ((229e3)**2 - (v * KMS)**2) / (R * KPC)
        print(f"{R:5.1f}  {v:6.1f}   {229.0:6.1f}      {v_obs_eilers(R)[0]/KMS:6.1f}        "
              f"{da:.3e}       {rho_gas_mid(R)/RHO:.3e}      {B_obs(R)[0]/uG:5.2f}")

    for label, vfn in (("FLAT 229 km/s", v_obs_flat), ("EILERS+19 (229 - 1.7 km/s/kpc)", v_obs_eilers)):
        print(f"\n--- Required field to support the GAS, v_obs = {label} ---")
        print("Radius |  B_req (L=1 kpc)  B_req (L=3 kpc)  B_req (L=R, toroidal hoop) | B_obs | ratio B_req/B_obs (L=3) | energy-density ratio (L=3)")
        for R in radii:
            b1, _ = B_req(R, vfn, 1.0)
            b3, _ = B_req(R, vfn, 3.0)
            bR, _ = B_req(R, vfn, "R")
            bo = B_obs(R)[0]
            print(f"{R:5.1f}  | {b1[0]/uG:8.1f} uG      {b3[0]/uG:8.1f} uG      {bR[0]/uG:8.1f} uG"
                  f"                | {bo/uG:5.2f} | {b3[0]/bo:6.1f}x                  | {(b3[0]/bo)**2:7.0f}x")

    print("\n--- Same, flat 229 km/s, but with the observed HI flaring (lower outer midplane gas density) ---")
    print("Radius |  B_req (L=1 kpc)  B_req (L=3 kpc)  B_req (L=R) | B_obs | ratio (L=1) (L=3) (L=R)")
    for R in radii:
        b1, _ = B_req(R, v_obs_flat, 1.0, rho_fn=rho_gas_mid_flared)
        b3, _ = B_req(R, v_obs_flat, 3.0, rho_fn=rho_gas_mid_flared)
        bR, _ = B_req(R, v_obs_flat, "R", rho_fn=rho_gas_mid_flared)
        bo = B_obs(R)[0]
        print(f"{R:5.1f}  | {b1[0]/uG:8.1f} uG      {b3[0]/uG:8.1f} uG      {bR[0]/uG:8.1f} uG | {bo/uG:5.2f} |"
              f" {b1[0]/bo:5.1f}x {b3[0]/bo:5.1f}x {bR[0]/bo:5.1f}x")

    print("\n--- If the field had to hold up ALL baryons (stars+gas) at the midplane, L = 3 kpc ---")
    print("(physically impossible: stars feel no Lorentz force; shown only for scale)")
    for R in radii:
        b3, _ = B_req(R, v_obs_flat, 3.0, rho_fn=lambda r: rho_gas_mid(r) + rho_star_mid(r))
        print(f"R={R:5.1f} kpc: B_req = {b3[0]/uG:7.1f} uG  vs observed {B_obs(R)[0]/uG:4.2f} uG  "
              f"-> {b3[0]/B_obs(R)[0]:.0f}x")

    # Direction check: a toroidal field B_phi(R) gives f_R = -(1/(2 mu0 R^2)) d(R^2 B^2)/dR
    print("\n--- Sign check: radial force from the OBSERVED field if it were purely toroidal ---")
    print("f_R = -(B^2/mu0) * (1/R - 1/L_B) with L_B = 12 kpc  (negative = inward)")
    for R in radii:
        B = B_obs(R)[0]
        fR = -(B**2 / MU0) * (1 / (R * KPC) - 1 / (12 * KPC))
        aB = fR / rho_gas_mid(R)
        da = ((229e3)**2 - v_bar2(R)[0]) / (R * KPC)
        print(f"R={R:5.1f} kpc: a_B = {aB:+.2e} m/s^2 on the gas  vs needed -{da:.2e} m/s^2 "
              f"-> supplies {(-aB/da)*100:+.1f}% of what is needed")

    # Energy-density comparison near the Sun
    B = B_obs(R_SUN)[0]
    uB = B**2 / (2 * MU0)
    ukin = 0.5 * rho_gas_mid(R_SUN) * (229e3)**2
    eV_cm3 = 1.602176634e-19 / 1e-6
    print(f"\nNear the Sun: B^2/2mu0 = {uB:.2e} J/m^3 = {uB/eV_cm3:.2f} eV/cm^3;  "
          f"gas rotational KE density 1/2 rho v^2 = {ukin:.2e} J/m^3 = {ukin/eV_cm3:.0f} eV/cm^3 "
          f"(ratio {ukin/uB:.0f}:1)")

    # Global energy budget out to 30 kpc
    Rg = np.linspace(0.5, 30, 300)
    dR = (Rg[1] - Rg[0]) * KPC
    vb2 = v_bar2(Rg)
    dv2 = np.maximum(v_obs_flat(Rg)**2 - vb2, 0)
    EB = {H: np.sum(2 * np.pi * Rg * KPC * dR * 2 * H * KPC * B_obs(Rg)**2 / (2 * MU0)) for H in (1.5, 5.0)}
    Kgas = np.sum(0.5 * sigma_gas_tot(Rg) * SIG * dv2 * 2 * np.pi * Rg * KPC * dR)
    Kall = np.sum(0.5 * sigma_all(Rg) * SIG * dv2 * 2 * np.pi * Rg * KPC * dR)
    R25 = 25.0
    Mmiss = R25 * KPC * (v_obs_flat(R25)[0]**2 - v_bar2(R25)[0]) / G
    Whalo = Mmiss * (229e3)**2
    print("\n--- Global energy budget (R < 30 kpc) ---")
    print(f"Magnetic energy, magnetised layer half-thickness 1.5 kpc : {EB[1.5]:.2e} J")
    print(f"Magnetic energy, generous half-thickness 5 kpc            : {EB[5.0]:.2e} J")
    print(f"'Missing' rotational KE of the GAS   1/2 Sum m_gas (v_obs^2-v_bar^2): {Kgas:.2e} J")
    print(f"'Missing' rotational KE of ALL baryons                              : {Kall:.2e} J")
    print(f"Missing (dark) mass inside 25 kpc  R(v_obs^2-v_bar^2)/G = {Mmiss/MSUN:.2e} Msun;"
          f"  its energy scale M v^2 = {Whalo:.2e} J")
    print(f"Ratios: E_B(1.5kpc)/K_gas = {EB[1.5]/Kgas:.3f};  E_B(1.5kpc)/K_all = {EB[1.5]/Kall:.4f};"
          f"  E_B(5kpc)/(M_miss v^2) = {EB[5.0]/Whalo:.1e}")
    print("Note (virial theorem): a magnetic field's energy enters the virial balance with the same sign as")
    print("kinetic energy/pressure - globally it pushes a system apart; it cannot confine it without external")
    print("currents or walls. So even matching energies would not give net inward support.")

    # Repo's own model: Plummer core in code units (src/galaxy.rs: GM_CORE=4000, EPS_CORE=8)
    print("\n--- Repo's own rotation law (src/galaxy.rs, code units, GM_CORE=4000, EPS_CORE=8) ---")
    for r in (20, 60, 120, 230):
        vc = np.sqrt(4000 * r**2 / (r**2 + 64)**1.5)
        print(f"r={r:4d}: Plummer v_c = {vc:5.2f}  (Keplerian ~ r^-1/2, i.e. NOT flat)")

    # Plot
    try:
        import matplotlib; matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        Rp = np.linspace(4, 28, 49)
        fig, ax = plt.subplots(1, 2, figsize=(12, 4.8))
        a = ax[0]
        a.plot(Rp, v_obs_flat(Rp)/KMS, "k-", label="observed, flat 229 km/s")
        a.plot(Rp, v_obs_eilers(Rp)/KMS, "k--", label="observed, Eilers+2019 fit")
        a.plot(Rp, np.sqrt(v_bar2(Rp))/KMS, "C1-", label="baryons only (McMillan 2017)")
        a.set_xlabel("Galactocentric radius R [kpc]"); a.set_ylabel("circular speed [km/s]")
        a.set_ylim(0, 260); a.legend(); a.set_title("Milky Way rotation: observed vs visible matter")
        a = ax[1]
        for L, c in ((1.0, "C2"), (3.0, "C3"), ("R", "C4")):
            b, _ = B_req(Rp, v_obs_flat, L)
            a.semilogy(Rp, b/uG, c, label=f"B required on gas, L={L if L!='R' else 'R (toroidal hoop)'}"
                       + (" kpc" if L != "R" else ""))
        b, _ = B_req(Rp, v_obs_flat, 1.0, rho_fn=rho_gas_mid_flared)
        a.semilogy(Rp, b/uG, "C2:", label="B required, L=1 kpc, with HI flaring (most generous)")
        a.fill_between(Rp, B_obs(Rp, 4.0)/uG, B_obs(Rp, 8.0)/uG, color="C0", alpha=0.3,
                       label="observed total B (6±2 µG at Sun, 12 kpc scale)")
        a.semilogy(Rp, B_obs(Rp)/uG, "C0-")
        a.set_xlabel("Galactocentric radius R [kpc]"); a.set_ylabel("field strength [µG]")
        a.set_title("Field needed to replace dark matter (gas only)")
        a.legend(fontsize=8); a.grid(True, which="both", alpha=0.3)
        fig.tight_layout(); fig.savefig(os.path.join(HERE, "mw_magnetic_check.png"), dpi=130)
        print("\nPlot written to", os.path.join(HERE, "mw_magnetic_check.png"))
    except Exception as e:
        print("plot skipped:", e)
