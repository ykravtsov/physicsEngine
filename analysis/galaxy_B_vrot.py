#!/usr/bin/env python3
"""Do galaxies with stronger magnetic fields rotate faster / need less dark matter?

Primary B sample (homogeneous method): Tabatabaei et al. 2017, ApJ 836, 185 (arXiv:1611.01705),
  Table 5/6 last column: galaxy-averaged, inclination-corrected equipartition total field B [uG];
  Table 1/2: SFR [Msun/yr] (Kennicutt+2011), optical size [arcmin] (NED), distance [Mpc], inclination.
Supplementary B (different methods; flagged, excluded from primary stats):
  Tabatabaei et al. 2013 A&A 557 A129 Table 3 (M31, M33, LMC); Basu & Roy 2013 MNRAS 433 1675 Table 4 (NGC1097, NGC5236).
Rotation speed priority:
  1) SPARC V_flat (Lelli, McGaugh & Schombert 2016, AJ 152, 157; Table 1)
  2) Tabatabaei et al. 2016 ApJL 818 L10 Table 1 v_rot (flat part of rotation curves, mostly Sofue+1999)
  3) HyperLeda 'vrot' (Makarov+2014), ONLY if HyperLeda inclination >= 40 deg (face-on vrot is unreliable)
  otherwise left blank.
Mass discrepancy: SPARC mass models, V_obs^2 / V_bar^2 at the last measured point, with
  M/L_disk = 0.5, M/L_bulge = 0.7 at 3.6um (Lelli+2016 fiducial); V_gas includes helium (x1.33).
"""
import os, re, csv, math
import numpy as np
from scipy import stats

HERE = os.path.dirname(os.path.abspath(__file__))
REF = os.path.join(HERE, "data") + os.sep   # populate with ./fetch_data.sh
G = 6.674e-11; MSUN = 1.989e30; PC = 3.0857e16; KPC = 1e3 * PC; MU0 = 4e-7 * np.pi; uG = 1e-10
MP = 1.6726e-27

def norm(n):
    n = n.replace(" ", "")
    return re.sub(r'^(NGC|IC|UGC|DDO)0*', r'\1', n)

# ---------- Tabatabaei 2017: B (Table 5/6) ----------
lines = open(REF + "tab17.txt").read().splitlines()
B17 = {}
for l in lines[500:870]:
    m = re.match(r'^\s*(NGC|IC|M)\s?(\d+)\s', l)
    if not m: continue
    mm = re.match(r'^(\d+\.\d)(\d+\.\d)$', l.split()[-1])   # "16.02.4" -> 16.0 (+/-2.4)
    if mm:
        B17[norm(m.group(1) + m.group(2)) if m.group(1) != 'M' else 'M' + m.group(2)] = (float(mm.group(1)), float(mm.group(2)))
# ---------- Tabatabaei 2017: Table 1/2 ----------
T1 = {}
for l in lines:
    m = re.match(r'^\s*(NGC \d+|IC\d+|M51|DDO\d+|HoI|M81DwB)\s+\d\d \d\d [\d.]+\s+[+-]\d\d \d\d \d\d\s+(\S+)\s+([\d.]+)×0?([\d.]+)\s+(\d+)\s+([\d.]+)e?\s+(\S+)\s+([\d.]+)\S*\s+(\S+)', l)
    if m:
        name = norm(m.group(1)) if not m.group(1).startswith('M51') else 'M51'
        sfr = m.group(9)
        T1[name] = dict(type=m.group(2), dmaj=float(m.group(3)), inc=float(m.group(5)), D=float(m.group(6)),
                        SFR=float(sfr) if sfr != '...' else None)
# ---------- SPARC ----------
sp = {}
for l in open(REF + "SPARC_Lelli2016c.mrt").read().splitlines()[98:]:
    t = l.split()
    if len(t) < 18: continue
    sp[norm(t[0])] = dict(D=float(t[2]), MHI=float(t[13]), RHI=float(t[14]),
                          Vflat=float(t[15]), eV=float(t[16]), Q=int(t[17]), raw=t[0])
mm_rows = {}
for l in open(REF + "MassModels_Lelli2016c.mrt").read().splitlines():
    t = l.split()
    if len(t) == 10 and re.match(r'^[\d.]+$', t[1]):
        try:
            mm_rows.setdefault(norm(t[0]), []).append([float(x) for x in t[1:8]])
        except ValueError:
            pass
def sparc_massmodel(name):
    a = np.array(mm_rows[name]); R, Vo, eVo, Vg, Vd, Vb = a[:, 1], a[:, 2], a[:, 3], a[:, 4], a[:, 5], a[:, 6]
    Vbar2 = Vg * np.abs(Vg) + 0.5 * Vd**2 + 0.7 * Vb**2
    return R, Vo, Vbar2
# ---------- Tabatabaei 2016 Table 1 v_rot ----------
T16 = {'IC342': 193, 'NGC6946': 220, 'NGC253': 211, 'NGC3628': 215, 'NGC4565': 255, 'NGC4736': 178, 'NGC5907': 254,
       'NGC891': 225, 'NGC1097': 295, 'NGC1365': 235, 'M31': 229, 'M33': 120, 'LMC': 69, 'SMC': 59}
# ---------- HyperLeda ----------
leda = {}
for r in csv.DictReader(open(REF + "leda.csv")):
    if r['vrot']:
        leda[norm(r['objname'])] = (float(r['vrot']), float(r['incl']))
alias = {'M51': 'NGC5194', 'M31': 'NGC224', 'M33': 'NGC598', 'NGC5236': 'NGC5236'}

rows = []
def add(name, B, eB, Bsrc, sample, SFR=None, dmaj=None, D=None):
    r = dict(galaxy=name, B_total_uG=B, B_err_plus_uG=eB, B_source=Bsrc, sample=sample, v_flat_kms='', v_source='',
             SFR_Msun_yr=SFR if SFR is not None else '', R25_kpc='', SigmaSFR_Msun_yr_kpc2='',
             mass_discrepancy_Vobs2_over_Vbar2_outer='', R_outer_kpc='', SPARC_MHI_1e9Msun='', SPARC_RHI_kpc='',
             md_source='')
    if dmaj and D:
        R25 = dmaj / 2 / 60 * math.pi / 180 * D * 1e3
        r['R25_kpc'] = round(R25, 2)
        if SFR: r['SigmaSFR_Msun_yr_kpc2'] = round(SFR / (math.pi * R25**2), 5)
    if name in sp:
        s = sp[name]; r['v_flat_kms'] = s['Vflat']; r['v_source'] = 'SPARC Lelli+2016 Vflat'
        R, Vo, Vbar2 = sparc_massmodel(name)
        r['mass_discrepancy_Vobs2_over_Vbar2_outer'] = round(Vo[-1]**2 / Vbar2[-1], 2)
        r['R_outer_kpc'] = R[-1]; r['SPARC_MHI_1e9Msun'] = s['MHI']; r['SPARC_RHI_kpc'] = s['RHI']
        r['md_source'] = 'SPARC mass models (Y_disk=0.5,Y_bul=0.7)'
    elif name in T16:
        r['v_flat_kms'] = T16[name]; r['v_source'] = 'Tabatabaei+2016 Table 1 (flat part of RC)'
    else:
        k = alias.get(name, name)
        if k in leda and leda[k][1] >= 40:
            r['v_flat_kms'] = leda[k][0]; r['v_source'] = f'HyperLeda vrot (incl {leda[k][1]:.0f} deg)'
    rows.append(r)

for name, (B, eB) in B17.items():
    t = T1.get(name, {})
    add(name, B, eB, 'Tabatabaei+2017 ApJ 836 185 Table 5/6', 'primary (homogeneous)', t.get('SFR'), t.get('dmaj'), t.get('D'))
for name, B, eB, src in [('M31', 6.6, 0.3, 'Tabatabaei+2013 A&A 557 A129 Table 3 (R=30-50 arcmin ring)'),
                         ('M33', 8.1, 0.5, 'Tabatabaei+2013 A&A 557 A129 Table 3'),
                         ('LMC', 4.2, '', 'Tabatabaei+2013 Table 3 (from Gaensler+2005)'),
                         ('NGC1097', 9.0, 2.0, 'Basu & Roy 2013 MNRAS 433 1675 Table 4'),
                         ('NGC5236', 12.2, 2.5, 'Basu & Roy 2013 MNRAS 433 1675 Table 4')]:
    add(name, B, eB, src, 'supplementary (different method)')

cols = list(rows[0].keys())
with open(os.path.join(HERE, "galaxy_B_vrot.csv"), "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=cols); w.writeheader(); [w.writerow(r) for r in rows]
print(f"Wrote {len(rows)} rows to {os.path.join(HERE, 'galaxy_B_vrot.csv')}")

def num(x): return float(x) if x not in ('', None) else np.nan
B = np.array([num(r['B_total_uG']) for r in rows]); V = np.array([num(r['v_flat_kms']) for r in rows])
S = np.array([num(r['SFR_Msun_yr']) for r in rows]); SS = np.array([num(r['SigmaSFR_Msun_yr_kpc2']) for r in rows])
MD = np.array([num(r['mass_discrepancy_Vobs2_over_Vbar2_outer']) for r in rows])
prim = np.array([r['sample'].startswith('primary') for r in rows])

def corr(x, y, label):
    ok = np.isfinite(x) & np.isfinite(y) & (x > 0) & (y > 0)
    lx, ly = np.log10(x[ok]), np.log10(y[ok])
    if ok.sum() < 4: print(label, "n<4"); return None
    rp, pp = stats.pearsonr(lx, ly); rs, ps = stats.spearmanr(x[ok], y[ok])
    fit = stats.linregress(lx, ly)
    print(f"{label:55s} n={ok.sum():2d}  Pearson(log) r={rp:+.2f} (p={pp:.3f})  Spearman rho={rs:+.2f} (p={ps:.3f})  "
          f"slope dlogY/dlogX={fit.slope:.2f}±{fit.stderr:.2f}")
    return fit

def partial(x, y, z, label):
    ok = np.isfinite(x) & np.isfinite(y) & np.isfinite(z) & (x > 0) & (y > 0) & (z > 0)
    if ok.sum() < 5: print(label, "n<5"); return
    def resid(a, b):
        f = stats.linregress(b, a); return a - (f.intercept + f.slope * b)
    lx, ly, lz = np.log10(x[ok]), np.log10(y[ok]), np.log10(z[ok])
    rp, pp = stats.pearsonr(resid(lx, lz), resid(ly, lz))
    rx, ry, rz = stats.rankdata(x[ok]), stats.rankdata(y[ok]), stats.rankdata(z[ok])
    rs, ps = stats.pearsonr(resid(rx, rz), resid(ry, rz))
    # multiple regression log B = a + b log v + c log z
    X = np.column_stack([np.ones(ok.sum()), lx, lz]); coef, *_ = np.linalg.lstsq(X, ly, rcond=None)
    print(f"{label:55s} n={ok.sum():2d}  partial Pearson r={rp:+.2f} (p={pp:.3f})  partial Spearman={rs:+.2f} (p={ps:.3f})  "
          f"multi-fit: logB = {coef[0]:.2f} + {coef[1]:.2f} log v + {coef[2]:.2f} log Z")

print("\n=== Correlations (primary homogeneous Tabatabaei+2017 sample) ===")
corr(V[prim], B[prim], "B vs v_flat")
corr(S[prim], B[prim], "B vs SFR")
corr(SS[prim], B[prim], "B vs Sigma_SFR (SFR / pi R25^2)")
corr(V[prim], S[prim], "SFR vs v_flat")
corr(MD[prim], B[prim], "B vs mass discrepancy (SPARC overlap only)")
partial(V[prim], B[prim], SS[prim], "B vs v_flat | controlling Sigma_SFR")
partial(V[prim], B[prim], S[prim], "B vs v_flat | controlling SFR")
print("\n=== All galaxies incl. supplementary ===")
corr(V, B, "B vs v_flat (all)")

# ---------- physics test for SPARC overlap galaxies ----------
print("\n=== Magnetic support test at the HI edge R_HI (where Sigma_HI = 1 Msun/pc^2 by SPARC definition) ===")
print("rho_gas = 1.33*Sigma_HI/(2h), h = 0.3 kpc assumed (thick-ish outer HI layer; thicker => lower B_req)")
print("Delta_a = (Vobs^2 - Vbar^2)/R at R_HI from SPARC mass models; B_obs = galaxy-AVERAGE B (overestimates the edge field)")
print("B_edge (illustrative) = B_avg * exp(-(R_HI - 0.5 R25)/12 kpc): Milky-Way-like 12 kpc field scale length (generous)")
print(f"{'galaxy':9s} {'B_obs':>6s} {'R_HI':>6s} {'Vobs':>6s} {'Vbar':>6s} {'Delta_a':>9s} {'B_req L=1kpc':>12s} {'B_req L=R':>10s} {'ratio(L=1)':>10s} {'ratio(L=R)':>10s} {'B_edge':>7s} {'Breq/Bedge':>11s}")
phys = []
for r in rows:
    n = r['galaxy']
    if n not in sp or r['B_total_uG'] == '': continue
    R, Vo, Vbar2 = sparc_massmodel(n); RHI = sp[n]['RHI']
    Rx = min(RHI, R[-1])
    vo = np.interp(Rx, R, Vo); vb2 = np.interp(Rx, R, Vbar2)
    da = (vo**2 - vb2) * 1e6 / (Rx * KPC)
    rho = 1.33 * 1.0 * MSUN / PC**2 / (2 * 0.3 * KPC)
    b1 = np.sqrt(max(MU0 * rho * da * 1 * KPC, 0)) / uG
    bR = np.sqrt(max(MU0 * rho * da * Rx * KPC, 0)) / uG
    Bo = float(r['B_total_uG'])
    R25 = float(r['R25_kpc']) if r['R25_kpc'] != '' else np.nan
    Be = Bo * np.exp(-(Rx - 0.5 * R25) / 12.0)
    print(f"{n:9s} {Bo:6.1f} {Rx:6.1f} {vo:6.1f} {np.sqrt(max(vb2,0)):6.1f} {da:9.2e} {b1:12.1f} {bR:10.1f} {b1/Bo:10.2f} {bR/Bo:10.2f} {Be:7.2f} {b1/Be:5.1f}-{bR/Be:4.0f}x")
    phys.append((n, Bo, b1, bR))

print("\n=== Energy density: B^2/2mu0 vs 1/2 rho_gas v_flat^2 ===")
print("rho_gas: (a) typical midplane n_H = 1 cm^-3 (x1.36 for He); (b) SPARC mean HI disc: 1.33*M_HI/(pi R_HI^2)/(2h), h=0.15 kpc")
rat_a = []; rat_b = []
for r in rows:
    if r['v_flat_kms'] == '': continue
    Bo = float(r['B_total_uG']) * uG; v = float(r['v_flat_kms']) * 1e3
    uB = Bo**2 / (2 * MU0)
    rho_a = 1.36 * MP * 1e6
    ra = 0.5 * rho_a * v**2 / uB; rat_a.append(ra)
    rb = ''
    if r['SPARC_MHI_1e9Msun'] != '':
        Sig = 1.33 * float(r['SPARC_MHI_1e9Msun']) * 1e9 * MSUN / (math.pi * (float(r['SPARC_RHI_kpc']) * KPC)**2)
        rho_b = Sig / (2 * 0.15 * KPC); rb = 0.5 * rho_b * v**2 / uB; rat_b.append(rb)
    r['_ra'] = ra; r['_rb'] = rb
print(f"(a) KE/uB: median {np.median(rat_a):.0f}, range {min(rat_a):.0f}-{max(rat_a):.0f}  (n={len(rat_a)})")
print(f"(b) KE/uB: median {np.median(rat_b):.0f}, range {min(rat_b):.0f}-{max(rat_b):.0f}  (n={len(rat_b)})")
print("    i.e. magnetic energy density is typically ~{:.1f}% (a) / ~{:.1f}% (b) of the gas rotational KE density".format(100/np.median(rat_a), 100/np.median(rat_b)))

# ---------- plot ----------
import matplotlib; matplotlib.use("Agg"); import matplotlib.pyplot as plt
plt.rcParams.update({"font.size": 11})
fig, ax = plt.subplots(1, 3, figsize=(17, 5.8))
src_mk = {'SPARC': ('o', 'C0'), 'Tabatabaei+2016': ('s', 'C2'), 'HyperLeda': ('^', 'C1')}
def mk(r):
    for k, v in src_mk.items():
        if r['v_source'].startswith(k): return v
    return ('x', 'k')
a = ax[0]
for r in rows:
    if r['v_flat_kms'] == '': continue
    m, c = mk(r); filled = r['sample'].startswith('primary')
    a.scatter(float(r['v_flat_kms']), float(r['B_total_uG']), marker=m, s=55,
              facecolors=c if filled else 'none', edgecolors=c, linewidths=1.5)
    a.annotate(r['galaxy'], (float(r['v_flat_kms']), float(r['B_total_uG'])), fontsize=7, xytext=(3, 3), textcoords='offset points')
okp = prim & np.isfinite(V)
f = stats.linregress(np.log10(V[okp]), np.log10(B[okp])); xx = np.linspace(40, 330, 50)
a.plot(xx, 10**(f.intercept + f.slope * np.log10(xx)), 'k--', lw=1, label=f'fit (primary): B ∝ v^{f.slope:.2f}')
for k, (m, c) in src_mk.items():
    a.scatter([], [], marker=m, color=c, label=f'v from {k}')
a.scatter([], [], marker='o', facecolors='none', edgecolors='gray', label='open = supplementary B')
a.set_xscale('log'); a.set_yscale('log'); a.set_xlabel('rotation speed v_flat [km/s]'); a.set_ylabel('total magnetic field B [µG]')
a.set_title('B vs rotation speed'); a.legend(fontsize=8, loc='upper left'); a.grid(True, which='both', alpha=0.3)
a = ax[1]
ok = np.isfinite(S) & prim
SS_plot = S
sc = a.scatter(SS_plot[ok], B[ok], c=np.where(np.isfinite(V[ok]), V[ok], np.nan), cmap='viridis', s=55, edgecolors='k')
a.scatter(SS_plot[ok & ~np.isfinite(V)], B[ok & ~np.isfinite(V)], facecolors='lightgray', edgecolors='k', s=55)
f2 = stats.linregress(np.log10(SS_plot[ok]), np.log10(B[ok])); xx = np.logspace(np.log10(SS_plot[ok].min()), np.log10(SS_plot[ok].max()), 50)
a.plot(xx, 10**(f2.intercept + f2.slope * np.log10(xx)), 'k--', lw=1, label=f'fit: B ∝ SFR^{f2.slope:.2f}')
plt.colorbar(sc, ax=a, label='v_flat [km/s] (gray = none)')
a.set_xscale('log'); a.set_yscale('log'); a.set_xlabel('star formation rate SFR [M☉/yr]'); a.set_ylabel('total magnetic field B [µG]')
a.set_title('B vs star formation rate (Tabatabaei+2017)'); a.legend(fontsize=8); a.grid(True, which='both', alpha=0.3)
a = ax[2]
okm = np.isfinite(MD)
a.scatter(MD[okm], B[okm], s=60, c='C0', edgecolors='k', label='B observed (galaxy average)')
for r in rows:
    if r['mass_discrepancy_Vobs2_over_Vbar2_outer'] != '':
        a.annotate(r['galaxy'], (float(r['mass_discrepancy_Vobs2_over_Vbar2_outer']), float(r['B_total_uG'])), fontsize=8, xytext=(3, 3), textcoords='offset points')
for n, Bo, b1, bR in phys:
    md = [float(r['mass_discrepancy_Vobs2_over_Vbar2_outer']) for r in rows if r['galaxy'] == n][0]
    a.plot([md, md], [b1, bR], color='C3', lw=2, alpha=0.7)
    a.scatter([md], [b1], marker='_', color='C3', s=120)
a.plot([], [], color='C3', lw=2, label='B needed to hold gas at HI edge\n(gradient scale L = 1 kpc to L = R)')
a.set_yscale('log'); a.set_xlabel('mass discrepancy V_obs²/V_bar² at last point (SPARC)'); a.set_ylabel('B [µG]')
a.set_title('B vs dark-matter need (SPARC overlap, n=8)'); a.legend(fontsize=8, loc='upper left'); a.grid(True, which='both', alpha=0.3)
from matplotlib.ticker import ScalarFormatter, NullFormatter
for aa in ax:
    for axis in ([aa.yaxis] + ([aa.xaxis] if aa.get_xscale()=='log' else [])):
        axis.set_major_formatter(ScalarFormatter()); axis.set_minor_formatter(NullFormatter())
ax[0].set_xticks([50,70,100,150,200,300]); ax[0].set_yticks([4,6,10,15,20,30]); ax[1].set_yticks([4,6,10,15,20,30]); ax[2].set_yticks([3,4,6,10,15,20,30])
ax[1].set_xticks([0.1,0.3,1,3,10]); ax[1].xaxis.set_major_formatter(matplotlib.ticker.FormatStrFormatter('%g'))
fig.tight_layout(); fig.savefig(os.path.join(HERE, 'galaxy_B_vrot.png'), dpi=130)
print("\nPlot:", os.path.join(HERE, "galaxy_B_vrot.png"))
