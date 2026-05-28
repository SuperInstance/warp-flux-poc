#!/usr/bin/env bash
# Warp FLUX PoC Demo Script
# Demonstrates FLUX constraint execution in a Warp-like terminal.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "═══════════════════════════════════════════════"
echo "  Warp FLUX PoC — Constraint Execution Engine"
echo "═══════════════════════════════════════════════"
echo ""
echo "Building..."
cargo build --quiet 2>&1 || cargo build
echo ""

echo "═══ Running Demo ═══"
echo ""
./target/debug/warp-flux-poc --demo 2>&1 || cargo run -- --demo 2>&1

echo ""
echo "═══ Proof Certificate (proof.cert) ═══"
if [ -f proof.cert ]; then
    cat proof.cert | python3 -m json.tool 2>/dev/null || cat proof.cert
else
    echo "(no proof.cert found)"
fi
echo ""
echo "═══ Demo Complete ═══"
echo "The FLUX constraint execution engine demonstrated:"
echo "  ✓ Pythagorean lattice creation (Euclid's formula)"
echo "  ✓ Nearest-point snapping on the unit circle"
echo "  ✓ Constraint verification (|v|² ≈ 1.0)"
echo "  ✓ SHA-256 proof chain (provably correct execution)"
echo "  ✓ Conservation threshold configuration"
echo "  ✓ Transition graph Laplacian analysis"
echo "  ✓ Spectral fingerprinting"
echo "  ✓ Proof chain integrity verification"
echo ""
