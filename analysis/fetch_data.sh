#!/usr/bin/env bash
# Download the public inputs used by galaxy_B_vrot.py into analysis/data/.
# Requires: curl, pdftotext (poppler-utils).
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p data && cd data
# Tabatabaei et al. 2017, ApJ 836, 185 (arXiv:1611.01705): equipartition B, SFR, sizes
curl -sL -o tab17.pdf https://arxiv.org/pdf/1611.01705v3 && pdftotext -layout tab17.pdf tab17.txt
# SPARC (Lelli, McGaugh & Schombert 2016, AJ 152, 157): galaxy table + mass models
curl -sL -o SPARC_Lelli2016c.mrt      https://astroweb.case.edu/SPARC/SPARC_Lelli2016c.mrt
curl -sL -o MassModels_Lelli2016c.mrt https://astroweb.case.edu/SPARC/MassModels_Lelli2016c.mrt
# HyperLeda (Makarov et al. 2014) inclination + vrot for the sample
names="IC2574 NGC0337 NGC0628 NGC1266 NGC2146 NGC2798 NGC2841 NGC2976 NGC3049 NGC3184 NGC3190 NGC3265 NGC3521 NGC3627 NGC3938 NGC4254 NGC4321 NGC4536 NGC4559 NGC4569 NGC4631 NGC4725 NGC4736 NGC4826 NGC5055 NGC5457 NGC5713 NGC5866 NGC6946 NGC7331 NGC5194 NGC0224 NGC0598 NGC1097 NGC5236 IC0342 NGC0253 NGC0891 NGC5907 NGC4565"
sql=$(for n in $names; do printf "objname%%3D%%27%s%%27%%20or%%20" "$n"; done); sql=${sql%%%20or%20}
curl -sL "http://atlas.obs-hp.fr/hyperleda/fG.cgi?n=meandata&c=o&of=1,leda,simbad&nra=l&nakd=1&d=objname,incl,vrot,e_vrot,vmaxg&sql=${sql}&ob=&a=csv%5B%2C%5D" | grep -v '^#' > leda.csv
echo "Fetched into $(pwd)"
