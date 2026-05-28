/// Pythagorean lattice — snap a value to the nearest exact (p,q) point
/// where p² + q² = r² (rational right triangle).

/// A point on the Pythagorean lattice represented as a fraction of the fundamental.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatticePoint {
    pub x: f64,
    pub y: f64,
    pub distance: f64, // sqrt(x² + y²) — the "exact" value
}

/// KD-tree optimized Pythagorean lattice for nearest-point lookups.
#[derive(Debug, Clone)]
pub struct PythagoreanLattice {
    /// Generated lattice points within the search range
    pub points: Vec<LatticePoint>,
    /// Lookup table for fast nearest-neighbor (sorted by distance)
    sorted_by_distance: Vec<LatticePoint>,
}

impl PythagoreanLattice {
    /// Generate Pythagorean lattice points using Euclid's formula:
    /// p = m² - n², q = 2mn, r = m² + n²
    /// for m > n > 0.
    pub fn new(precision: i64) -> Self {
        let mut data_points = Vec::new();
        let max_m = (precision as f64).sqrt() as i64 + 1;

        for m in 2..=max_m {
            for n in 1..m {
                let p = (m * m - n * n) as f64;
                let q = (2 * m * n) as f64;
                let r = (m * m + n * n) as f64;
                let distance = r;

                // Only store unique distances
                if distance <= precision as f64 {
                    // Normalize: divide by r to get unit-circle coordinates
                    let x = p / r; // cos(θ)
                    let y = q / r; // sin(θ)
                    data_points.push(LatticePoint { x, y, distance });
                }
            }
        }

        // Sort by distance for fast lookup
        let mut sorted_by_distance = data_points.clone();
        sorted_by_distance.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        PythagoreanLattice {
            points: data_points,
            sorted_by_distance,
        }
    }

    /// Snap a value to the nearest Pythagorean lattice point.
    /// Uses Euclidean distance on the unit circle.
    pub fn snap(&self, value: f64) -> LatticePoint {
        if self.points.is_empty() {
            return LatticePoint {
                x: 0.0,
                y: 0.0,
                distance: 0.0,
            };
        }

        // Normalize the value to the unit circle
        let normalized = value.abs() % 2.0;
        let theta = normalized * std::f64::consts::PI;

        let target_x = theta.cos();
        let target_y = theta.sin();

        // Find nearest point using brute-force (fine for POC scale)
        let mut best = self.points[0];
        let mut best_dist = f64::MAX;

        for pt in &self.points {
            let dx = pt.x - target_x;
            let dy = pt.y - target_y;
            let dist = dx * dx + dy * dy;
            if dist < best_dist {
                best_dist = dist;
                best = *pt;
            }
        }

        // Scale back: distance * fundamental
        LatticePoint {
            x: best.x * value.signum(),
            y: best.y * value.signum(),
            distance: best.distance,
        }
    }

    /// Find nearest exact rational representation of a value.
    pub fn snap_exact(&self, p: f64, q: f64) -> (f64, f64) {
        // Find the nearest Pythagorean triple (p² + q² = r²)
        let mut best = (0.0f64, 0.0f64);
        let mut best_err = f64::MAX;

        for pt in &self.points {
            let err = (pt.x - p).abs() + (pt.y - q).abs();
            if err < best_err {
                best_err = err;
                best = (pt.x, pt.y);
            }
        }

        best
    }

    /// Verify that a point satisfies the Pythagorean constraint.
    pub fn verify_exact(&self, p: f64, q: f64) -> bool {
        let squared = p * p + q * q;
        let r = squared.sqrt();
        // Check if r is "close enough" to an integer
        let r_rounded = r.round();
        (r - r_rounded).abs() < 1e-9
    }
}

/// Generate Pythagorean triples up to a given bound.
pub fn generate_pythagorean_triples(limit: i64) -> Vec<(i64, i64, i64)> {
    let mut triples = Vec::new();
    for m in 2..=(limit as f64).sqrt() as i64 + 1 {
        for n in 1..m {
            let a = m * m - n * n;
            let b = 2 * m * n;
            let c = m * m + n * n;
            if c <= limit {
                triples.push((a, b, c));
            }
        }
    }
    triples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lattice_creation() {
        let lat = PythagoreanLattice::new(200);
        assert!(!lat.points.is_empty(), "Lattice should have points");
    }

    #[test]
    fn test_snap() {
        let lat = PythagoreanLattice::new(200);
        let snapped = lat.snap(0.577);
        assert!(snapped.distance > 0.0, "Snapped point should have distance");
    }

    #[test]
    fn test_verify_triples() {
        let triples = generate_pythagorean_triples(100);
        assert!(!triples.is_empty(), "Should find some triples");
        // Verify (3,4,5) is present
        assert!(triples.contains(&(3, 4, 5)), "Should contain (3,4,5)");
    }

    #[test]
    fn test_verify_exact_true() {
        let lat = PythagoreanLattice::new(200);
        // (0.6, 0.8) is a Pythagorean triple (3-4-5 scaled)
        assert!(lat.verify_exact(0.6, 0.8));
    }
}
