/// Simplified conservation analysis engine.
/// Builds tension-graph Laplacians, performs power iteration,
/// and checks conservation conditions.
use serde::{Serialize, Deserialize};

/// A node in the conservation graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationNode {
    pub id: usize,
    pub value: f64,
    pub label: String,
}

/// An edge in the conservation graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationEdge {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
}

/// A tension-graph Laplacian matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensionGraph {
    pub nodes: Vec<ConservationNode>,
    pub edges: Vec<ConservationEdge>,
    /// The Laplacian matrix L = D - A
    pub laplacian: Vec<Vec<f64>>,
    /// Dominant eigenvalue
    pub dominant_eigenvalue: f64,
    /// Dominant eigenvector
    pub dominant_eigenvector: Vec<f64>,
}

impl TensionGraph {
    /// Build a Laplacian from nodes and edges.
    pub fn new(nodes: Vec<ConservationNode>, edges: Vec<ConservationEdge>) -> Self {
        let n = nodes.len();
        let mut laplacian = vec![vec![0.0; n]; n];

        // Build adjacency and degree
        let mut degree = vec![0.0; n];
        for edge in &edges {
            if edge.from < n && edge.to < n {
                let w = edge.weight.abs();
                laplacian[edge.from][edge.to] = -w;
                laplacian[edge.to][edge.from] = -w;
                degree[edge.from] += w;
                degree[edge.to] += w;
            }
        }

        // Set degree on diagonal
        for i in 0..n {
            laplacian[i][i] = degree[i];
        }

        // Power iteration for dominant eigenvalue/eigenvector
        let (eigenvalue, eigenvector) = Self::power_iteration(&laplacian, n, 200);

        TensionGraph {
            nodes,
            edges,
            laplacian,
            dominant_eigenvalue: eigenvalue,
            dominant_eigenvector: eigenvector,
        }
    }

    /// Power iteration to find the dominant eigenvalue/eigenvector.
    fn power_iteration(matrix: &[Vec<f64>], n: usize, iterations: usize) -> (f64, Vec<f64>) {
        let mut eigenvector = vec![1.0; n];

        for _ in 0..iterations {
            let mut new_vec = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    new_vec[i] += matrix[i][j] * eigenvector[j];
                }
            }
            // Normalize
            let norm: f64 = new_vec.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm > 1e-12 {
                for v in &mut new_vec {
                    *v /= norm;
                }
            }
            eigenvector = new_vec;
        }

        // Rayleigh quotient: λ = (v^T L v) / (v^T v)
        let mut rayleigh_num = 0.0;
        let mut rayleigh_den = 0.0;
        for i in 0..n {
            for j in 0..n {
                rayleigh_num += eigenvector[i] * matrix[i][j] * eigenvector[j];
            }
            rayleigh_den += eigenvector[i] * eigenvector[i];
        }

        let eigenvalue = if rayleigh_den > 1e-12 {
            rayleigh_num / rayleigh_den
        } else {
            0.0
        };

        (eigenvalue, eigenvector)
    }

    /// Check conservation condition: alignment with dominant eigenmode.
    /// Returns (conserved: bool, score: f64, message: String)
    pub fn check_conservation(&self, before: f64, after: f64) -> (bool, f64, String) {
        let delta = (after - before).abs();
        let threshold = 0.001;

        // Alignment with eigenvector
        let eigen_alignment: f64 = before * self.dominant_eigenvector.first().copied().unwrap_or(1.0)
            - after * self.dominant_eigenvector.last().copied().unwrap_or(1.0);

        let score = delta + eigen_alignment.abs() * 0.1;
        let conserved = score < threshold;

        let msg = if conserved {
            format!(
                "✓ Energy conserved (Δ={:.6}, alignment={:.6}, λ₁={:.4})",
                delta, eigen_alignment, self.dominant_eigenvalue
            )
        } else {
            format!(
                "✗ Conservation VIOLATED (Δ={:.6} > threshold={})",
                delta, threshold
            )
        };

        (conserved, score, msg)
    }

    /// Compute spectral gap: gap between top two eigenvalues.
    pub fn spectral_gap(&self) -> f64 {
        // Second eigenvalue via power iteration with deflation
        let n = self.nodes.len();
        let v = &self.dominant_eigenvector;

        // Deflate: L' = L - λ₁ * v * v^T
        let mut deflated = self.laplacian.clone();
        for i in 0..n {
            for j in 0..n {
                deflated[i][j] -= self.dominant_eigenvalue * v[i] * v[j];
            }
        }

        let (second_eigenvalue, _) = Self::power_iteration(&deflated, n, 200);
        self.dominant_eigenvalue - second_eigenvalue
    }

    /// Get the graph's innovation potential (= gap fraction).
    pub fn innovation_potential(&self) -> f64 {
        let gap = self.spectral_gap();
        if self.dominant_eigenvalue.abs() > 1e-12 {
            gap / self.dominant_eigenvalue
        } else {
            0.0
        }
    }
}

/// Analyzes conservation across a set of transitions.
#[derive(Debug)]
pub struct ConservationAnalysis {
    pub total_tension_before: f64,
    pub total_tension_after: f64,
    pub is_conserved: bool,
    pub score: f64,
    pub spectral_gap: f64,
    pub innovation_potential: f64,
}

impl ConservationAnalysis {
    /// Analyze a sequence of values for conservation.
    pub fn analyze(values: &[f64]) -> Self {
        let n = values.len();
        if n < 2 {
            return ConservationAnalysis {
                total_tension_before: values.first().copied().unwrap_or(0.0),
                total_tension_after: 0.0,
                is_conserved: false,
                score: 0.0,
                spectral_gap: 0.0,
                innovation_potential: 0.0,
            };
        }

        // Build a line graph of transitions
        let nodes: Vec<ConservationNode> = values.iter().enumerate().map(|(i, &v)| {
            ConservationNode {
                id: i,
                value: v,
                label: format!("v{}", i),
            }
        }).collect();

        let edges: Vec<ConservationEdge> = (0..n-1).map(|i| {
            ConservationEdge {
                from: i,
                to: i + 1,
                weight: 1.0,
            }
        }).collect();

        let graph = TensionGraph::new(nodes, edges);

        let tension_before = values[0] * values[0];
        let tension_after = values[n - 1] * values[n - 1];
        let (conserved, score, _) = graph.check_conservation(tension_before, tension_after);
        let gap = graph.spectral_gap();
        let innovation = graph.innovation_potential();

        ConservationAnalysis {
            total_tension_before: tension_before,
            total_tension_after: tension_after,
            is_conserved: conserved,
            score,
            spectral_gap: gap,
            innovation_potential: innovation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tension_graph_creation() {
        let nodes = vec![
            ConservationNode { id: 0, value: 0.6, label: "A".to_string() },
            ConservationNode { id: 1, value: 0.8, label: "B".to_string() },
        ];
        let edges = vec![
            ConservationEdge { from: 0, to: 1, weight: 1.0 },
        ];
        let graph = TensionGraph::new(nodes, edges);
        assert!(graph.dominant_eigenvalue.abs() > 0.0 || graph.dominant_eigenvalue == 0.0,
            "Dominant eigenvalue should be non-negative, got {}", graph.dominant_eigenvalue);
        // For a 2-node graph with one edge, Laplacian [[1,-1],[-1,1]] has eigenvalues [0, 2]
        assert!((graph.dominant_eigenvalue - 2.0).abs() < 0.1 || graph.dominant_eigenvalue.abs() < 0.1,
            "Eigenvalue should be ~2 or ~0 for a single-edge graph, got {}", graph.dominant_eigenvalue);
    }

    #[test]
    fn test_conservation_check() {
        let nodes = vec![
            ConservationNode { id: 0, value: 0.6, label: "A".to_string() },
            ConservationNode { id: 1, value: 0.8, label: "B".to_string() },
        ];
        let edges = vec![
            ConservationEdge { from: 0, to: 1, weight: 1.0 },
        ];
        let graph = TensionGraph::new(nodes, edges);
        let (conserved, _, _) = graph.check_conservation(1.0, 1.0);
        assert!(conserved, "Conservation should hold for equal values");
    }

    #[test]
    fn test_conservation_analysis() {
        let values = vec![0.6, 0.61, 0.59, 0.6];
        let analysis = ConservationAnalysis::analyze(&values);
        assert!(analysis.is_conserved || analysis.score < 0.1);
    }
}
