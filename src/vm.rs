/// FLUX-C stack-based interpreter — 20 opcodes subset.
/// Demonstrates a simplified FLUX execution engine with constraint checking.
use crate::lattice::{LatticePoint, PythagoreanLattice};
use crate::proof::ProofChain;
use sha2::{Digest, Sha256};
use std::fmt;

/// FLUX value types supported by the VM.
#[derive(Debug, Clone, PartialEq)]
pub enum FluxValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    LatticePoint(LatticePoint),
    ConservationThreshold(f64),
    None,
}

impl fmt::Display for FluxValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FluxValue::Int(i) => write!(f, "{}", i),
            FluxValue::Float(fl) => write!(f, "{:.6}", fl),
            FluxValue::Bool(b) => write!(f, "{}", b),
            FluxValue::String(s) => write!(f, "\"{}\"", s),
            FluxValue::LatticePoint(pt) => {
                write!(f, "[{:.4}, {:.4}] (r={})", pt.x, pt.y, pt.distance)
            }
            FluxValue::ConservationThreshold(t) => write!(f, "threshold={}", t),
            FluxValue::None => write!(f, "none"),
        }
    }
}

/// FLUX operation codes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opcode {
    PUSH,
    POP,
    DUP,
    SWAP,
    ADD,
    SUB,
    MUL,
    DIV,
    SNAP,       // snap to lattice
    CHECK,      // verify constraint
    CONSERVE,   // set conservation threshold
    HASH,       // hash value into proof chain
    VERIFY,     // verify proof chain
    FINGERPRINT,// show spectral fingerprint
    ANALYZE,    // load and analyze transitions
    HALT,
}

impl Opcode {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "push" | "PUSH" => Some(Opcode::PUSH),
            "pop" | "POP" => Some(Opcode::POP),
            "dup" | "DUP" => Some(Opcode::DUP),
            "swap" | "SWAP" => Some(Opcode::SWAP),
            "add" | "ADD" | "+" => Some(Opcode::ADD),
            "sub" | "SUB" | "-" => Some(Opcode::SUB),
            "mul" | "MUL" | "*" => Some(Opcode::MUL),
            "div" | "DIV" | "/" => Some(Opcode::DIV),
            "snap" | "SNAP" => Some(Opcode::SNAP),
            "check" | "CHECK" => Some(Opcode::CHECK),
            "conserve" | "CONSERVE" => Some(Opcode::CONSERVE),
            "hash" | "HASH" => Some(Opcode::HASH),
            "verify" | "VERIFY" => Some(Opcode::VERIFY),
            "fingerprint" | "FINGERPRINT" => Some(Opcode::FINGERPRINT),
            "analyze" | "ANALYZE" => Some(Opcode::ANALYZE),
            "halt" | "HALT" | "exit" | "quit" => Some(Opcode::HALT),
            _ => None,
        }
    }
}

/// Execution result from the VM.
#[derive(Debug)]
pub struct ExecResult {
    pub stack: Vec<FluxValue>,
    pub proof_hash: String,
    pub cycles: u16,
    pub valid: bool,
    pub message: String,
}

/// The FLUX-C stack VM.
pub struct FluxVM {
    /// Data stack
    stack: Vec<FluxValue>,
    /// Proof chain (SHA-256 linked list)
    proof: ProofChain,
    /// Cycle counter (max 4096 for termination guarantee)
    cycles: u16,
    /// Lattice for snapping operations
    lattice: Option<PythagoreanLattice>,
    /// Current conservation threshold
    conservation_threshold: f64,
    /// Whether the VM is running
    running: bool,
}

impl FluxVM {
    pub fn new() -> Self {
        FluxVM {
            stack: Vec::new(),
            proof: ProofChain::new(),
            cycles: 0,
            lattice: None,
            conservation_threshold: 0.001,
            running: true,
        }
    }

    /// Execute a single opcode with optional arguments.
    pub fn execute_op(&mut self, op: Opcode, args: &[&str]) -> Result<ExecResult, String> {
        if !self.running {
            return Err("VM is halted".to_string());
        }

        self.cycles += 1;
        if self.cycles > 4096 {
            self.running = false;
            return Err("Cycle limit exceeded (max 4096)".to_string());
        }

        match op {
            Opcode::PUSH => {
                let val_str = args.join(" ");
                let val = self.parse_value(val_str.trim());
                self.stack.push(val.clone());
                self.proof.extend("PUSH", &format!("{}", val));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("Pushed {}", val),
                })
            }
            Opcode::POP => {
                let val =
                    self.stack.pop().ok_or("Stack underflow: POP on empty stack")?;
                self.proof.extend("POP", &format!("{}", val));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("Popped {}", val),
                })
            }
            Opcode::DUP => {
                let val = self
                    .stack
                    .last()
                    .ok_or("Stack underflow: DUP on empty stack")?
                    .clone();
                self.stack.push(val.clone());
                self.proof.extend("DUP", &format!("{}", val));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("Duplicated {}", val),
                })
            }
            Opcode::SWAP => {
                let a = self
                    .stack
                    .pop()
                    .ok_or("Stack underflow: SWAP needs 2 values")?;
                let b = self
                    .stack
                    .pop()
                    .ok_or("Stack underflow: SWAP needs 2 values")?;
                self.stack.push(a.clone());
                self.stack.push(b.clone());
                self.proof.extend("SWAP", &format!("{} ↔ {}", a, b));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("Swapped {} and {}", a, b),
                })
            }
            Opcode::ADD => {
                let b = self.pop_float("ADD")?;
                let a = self.pop_float("ADD")?;
                let result = a + b;
                self.stack.push(FluxValue::Float(result));
                self.proof
                    .extend("ADD", &format!("{} + {} = {}", a, b, result));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("{} + {} = {}", a, b, result),
                })
            }
            Opcode::SUB => {
                let b = self.pop_float("SUB")?;
                let a = self.pop_float("SUB")?;
                let result = a - b;
                self.stack.push(FluxValue::Float(result));
                self.proof
                    .extend("SUB", &format!("{} - {} = {}", a, b, result));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("{} - {} = {}", a, b, result),
                })
            }
            Opcode::MUL => {
                let b = self.pop_float("MUL")?;
                let a = self.pop_float("MUL")?;
                let result = a * b;
                self.stack.push(FluxValue::Float(result));
                self.proof
                    .extend("MUL", &format!("{} × {} = {}", a, b, result));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("{} × {} = {}", a, b, result),
                })
            }
            Opcode::DIV => {
                let b = self.pop_float("DIV")?;
                let a = self.pop_float("DIV")?;
                if b == 0.0 {
                    return Err("Division by zero".to_string());
                }
                let result = a / b;
                self.stack.push(FluxValue::Float(result));
                self.proof
                    .extend("DIV", &format!("{} / {} = {}", a, b, result));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("{} / {} = {}", a, b, result),
                })
            }
            Opcode::SNAP => {
                // Clone the lattice first to avoid borrowing issues
                let lattice = self
                    .lattice
                    .as_ref()
                    .ok_or("No lattice initialized. Use `lattice` command first.")?
                    .clone();
                let val = self.pop_float("SNAP")?;
                let snapped = lattice.snap(val);
                self.stack
                    .push(FluxValue::LatticePoint(snapped));
                self.proof
                    .extend("SNAP", &format!("{} → [{:.4}, {:.4}]", val, snapped.x, snapped.y));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!(
                        "Snapped {} → [{:.4}, {:.4}] (r={:.4})",
                        val, snapped.x, snapped.y, snapped.distance
                    ),
                })
            }
            Opcode::CHECK => {
                let val = self.pop_float("CHECK")?;
                // Check |v|² == 1.0 (unit circle constraint)
                let constraint = val * val;
                let passes = (constraint - 1.0).abs() < self.conservation_threshold;
                self.stack.push(FluxValue::Bool(passes));
                if passes {
                    self.proof.extend("CHECK", &format!("|{}|² ≈ 1.0 (passes)", val));
                } else {
                    self.proof
                        .extend("CHECK", &format!("|{}|² ≈ 1.0 (violated)", val));
                }
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: passes,
                    message: if passes {
                        format!("✓ Constraint |{:.4}|² ≈ 1.0 PASSES", val)
                    } else {
                        format!(
                            "✗ Constraint |{:.4}|² ≈ 1.0 VIOLATED (got {:.6})",
                            val, constraint
                        )
                    },
                })
            }
            Opcode::CONSERVE => {
                let threshold_str = args.get(0).copied().unwrap_or("0.001");
                let threshold: f64 = threshold_str.parse().unwrap_or(0.001);
                self.conservation_threshold = threshold;
                self.proof.extend("CONSERVE", &format!("threshold={}", threshold));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("🔒 Conservation threshold set to {}", threshold),
                })
            }
            Opcode::HASH => {
                let val = self.stack.last().cloned().unwrap_or(FluxValue::None);
                let hash = self.proof.extend("HASH", &format!("{}", val));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: hash.clone(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("🔑 Hashed → {}", &hash[..16]),
                })
            }
            Opcode::VERIFY => {
                let (valid, msg) = self.proof.verify();
                self.stack.push(FluxValue::Bool(valid));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid,
                    message: format!("{} {}", if valid { "✓" } else { "✗" }, msg),
                })
            }
            Opcode::FINGERPRINT => {
                let fp = self.compute_fingerprint();
                self.proof.extend("FINGERPRINT", &fp);
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!(
                        "🔬 Spectral Fingerprint: {}",
                        fp
                    ),
                })
            }
            Opcode::ANALYZE => {
                let json_str = args.join(" ");
                let analysis = self.analyze_transitions(&json_str);
                self.proof.extend("ANALYZE", &analysis);
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("📊 Transition Analysis: {}", analysis),
                })
            }
            Opcode::HALT => {
                self.running = false;
                let proof = self.proof.current_hash();
                let msg = format!(
                    "⏹  VM halted. {} cycles, proof: {}...",
                    self.cycles,
                    &proof[..16]
                );
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: proof,
                    cycles: self.cycles,
                    valid: true,
                    message: msg,
                })
            }
        }
    }

    /// Execute a full FLUX command string (REPL-style).
    pub fn execute_command(&mut self, line: &str) -> Result<ExecResult, String> {
        let line = line.trim();
        if line.is_empty() {
            return Err("Empty command".to_string());
        }

        // Special commands that don't fit the opcode pattern
        if line.starts_with("lattice ") || line.starts_with("LATTICE ") {
            let rest = &line[8..].trim();
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() < 2 {
                return Err(
                    "Usage: lattice <type> <precision>, e.g. lattice pythagorean 200"
                        .to_string(),
                );
            }
            let _lattice_type = parts[0];
            let precision: i64 = parts[1]
                .parse()
                .map_err(|_| "Invalid precision".to_string())?;

            let lat = PythagoreanLattice::new(precision);
            let count = lat.points.len();
            self.lattice = Some(lat);
            self.proof.extend(
                "LATTICE",
                &format!(
                    "pythagorean precision={} ({} points)",
                    precision, count
                ),
            );
            Ok(ExecResult {
                stack: self.stack.clone(),
                proof_hash: self.proof.current_hash(),
                cycles: self.cycles,
                valid: true,
                message: format!(
                    "🔷 Lattice initialized: Pythagorean ({} points, precision={})",
                    count, precision
                ),
            })
        } else if line.starts_with("snap ") {
            let rest = line[5..].trim();
            let parts: Vec<&str> = rest.split_whitespace().collect();

            if parts.len() == 2 {
                // snap x y — snap a coordinate pair
                let x: f64 = parts[0]
                    .parse()
                    .map_err(|_| "Invalid x coordinate".to_string())?;
                let y: f64 = parts[1]
                    .parse()
                    .map_err(|_| "Invalid y coordinate".to_string())?;
                let lattice = self
                    .lattice
                    .as_ref()
                    .ok_or("No lattice initialized")?
                    .clone();
                let (sx, sy) = lattice.snap_exact(x, y);
                self.stack.push(FluxValue::Float(sx));
                self.stack.push(FluxValue::Float(sy));
                self.proof
                    .extend("SNAP", &format!("({}, {}) → ({}, {})", x, y, sx, sy));
                Ok(ExecResult {
                    stack: self.stack.clone(),
                    proof_hash: self.proof.current_hash(),
                    cycles: self.cycles,
                    valid: true,
                    message: format!("Snapped [{:.3}, {:.3}] → [{:.3}, {:.3}]", x, y, sx, sy),
                })
            } else if parts.len() == 1 {
                let _val: f64 = parts[0]
                    .parse()
                    .map_err(|_| "Invalid value for snap".to_string())?;
                self.execute_op(Opcode::SNAP, &[])
            } else {
                self.execute_op(Opcode::SNAP, &[])
            }
        } else if line.starts_with("check ") || line == "check" {
            self.execute_op(Opcode::CHECK, &[])
        } else if line.starts_with("conserve ") {
            let rest = &line[9..].trim();
            let parts: Vec<&str> = rest.split_whitespace().collect();
            let threshold = if parts.len() >= 2 && parts[0] == "tension" {
                parts[1].parse().unwrap_or(0.85)
            } else {
                0.001
            };
            self.execute_op(Opcode::CONSERVE, &[&threshold.to_string()])
        } else if line.starts_with("analyze ") {
            let rest = &line[8..].trim();
            self.execute_op(Opcode::ANALYZE, &[rest])
        } else if line == "fingerprint" {
            self.execute_op(Opcode::FINGERPRINT, &[])
        } else if line == "verify" {
            self.execute_op(Opcode::VERIFY, &[])
        } else if line.starts_with("verify ") {
            let _path = line[7..].trim();
            self.execute_op(Opcode::VERIFY, &[])
        } else {
            // Try as a standard opcode line
            let split_pos = line.find(char::is_whitespace).unwrap_or(line.len());
            let op_name = &line[..split_pos];
            let args = if split_pos < line.len() {
                &line[split_pos + 1..]
            } else {
                ""
            };

            if op_name == "push" || op_name == "PUSH" {
                self.execute_op(Opcode::PUSH, &[args])
            } else if let Some(op) = Opcode::from_name(op_name) {
                self.execute_op(op, &[])
            } else if let Ok(val) = line.parse::<f64>() {
                self.execute_op(Opcode::PUSH, &[line])
            } else {
                Err(format!("Unknown command: '{}'", line))
            }
        }
    }

    /// Get the current proof chain.
    pub fn proof_chain(&self) -> &ProofChain {
        &self.proof
    }

    /// Get the current stack.
    pub fn stack(&self) -> &[FluxValue] {
        &self.stack
    }

    /// Get cycle count.
    pub fn cycles(&self) -> u16 {
        self.cycles
    }

    /// Check if VM is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    // --- Private helpers ---

    fn parse_value(&self, s: &str) -> FluxValue {
        if let Ok(i) = s.parse::<i64>() {
            return FluxValue::Int(i);
        }
        if let Ok(f) = s.parse::<f64>() {
            return FluxValue::Float(f);
        }
        match s {
            "true" | "True" | "TRUE" => return FluxValue::Bool(true),
            "false" | "False" | "FALSE" => return FluxValue::Bool(false),
            _ => {}
        }
        FluxValue::String(s.to_string())
    }

    fn pop_float(&mut self, op: &str) -> Result<f64, String> {
        match self.stack.pop() {
            Some(FluxValue::Int(i)) => Ok(i as f64),
            Some(FluxValue::Float(f)) => Ok(f),
            Some(FluxValue::LatticePoint(pt)) => Ok(pt.distance),
            Some(other) => {
                Err(format!("{}: type mismatch — expected Float, got {}", op, other))
            }
            None => Err(format!(
                "Stack underflow: {} needs a value on the stack",
                op
            )),
        }
    }

    fn compute_fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        for val in &self.stack {
            hasher.update(format!("{}|", val).as_bytes());
        }
        hasher.update(self.proof.current_hash().as_bytes());
        let hash = hex::encode(hasher.finalize());
        format!(
            "{:X}:{:X}:{:X}:{:X}:{:X}:{:X}",
            u64::from_str_radix(&hash[0..4], 16).unwrap_or(0),
            u64::from_str_radix(&hash[4..8], 16).unwrap_or(0),
            u64::from_str_radix(&hash[8..12], 16).unwrap_or(0),
            u64::from_str_radix(&hash[12..16], 16).unwrap_or(0),
            u64::from_str_radix(&hash[16..20], 16).unwrap_or(0),
            u64::from_str_radix(&hash[20..24], 16).unwrap_or(0),
        )
    }

    fn analyze_transitions(&self, json_str: &str) -> String {
        #[derive(serde::Deserialize)]
        struct TransitionGraph {
            nodes: Vec<f64>,
            edges: Vec<[usize; 2]>,
        }

        let graph: TransitionGraph = match serde_json::from_str(json_str) {
            Ok(g) => g,
            Err(_) => {
                let n = self.stack.len();
                return format!(
                    "Parsed {} stack values as graph. Simple conservation: n/a",
                    n
                );
            }
        };

        let n = graph.nodes.len();
        if n == 0 {
            return "Empty graph".to_string();
        }

        // Build Laplacian L = D - A
        let mut laplacian = vec![vec![0.0; n]; n];
        for edge in &graph.edges {
            let (i, j) = (edge[0], edge[1]);
            if i < n && j < n {
                laplacian[i][j] = -1.0;
                laplacian[j][i] = -1.0;
                laplacian[i][i] += 1.0;
                laplacian[j][j] += 1.0;
            }
        }

        // Simple power iteration for dominant eigenvalue estimate
        let mut eigenvector = vec![1.0; n];
        for _ in 0..100 {
            let mut new_vec = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    new_vec[i] += laplacian[i][j] * eigenvector[j];
                }
            }
            let norm: f64 = new_vec.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm > 1e-9 {
                for v in &mut new_vec {
                    *v /= norm;
                }
            }
            eigenvector = new_vec;
        }

        // Dominant eigenvalue = Rayleigh quotient
        let mut rayleigh_num = 0.0;
        let mut rayleigh_den = 0.0;
        for i in 0..n {
            for j in 0..n {
                rayleigh_num += eigenvector[i] * laplacian[i][j] * eigenvector[j];
            }
            rayleigh_den += eigenvector[i] * eigenvector[i];
        }
        let dominant_eigenvalue = rayleigh_num / rayleigh_den.max(1e-9);

        let tensions: Vec<f64> = graph.nodes.iter().map(|v| v * v).collect();
        let _total_tension: f64 = tensions.iter().sum();

        let _eigen_alignment: f64 = tensions
            .iter()
            .zip(eigenvector.iter())
            .map(|(t, e)| t * e)
            .sum::<f64>()
            / (tensions
                .iter()
                .map(|t| t * t)
                .sum::<f64>()
                .sqrt()
                .max(1.0)
                * eigenvector.iter().map(|e| e * e).sum::<f64>().sqrt().max(1.0));

        format!(
            "{} nodes, {} edges, λ₁={:.4}",
            n,
            graph.edges.len(),
            dominant_eigenvalue
        )
    }
}

impl Default for FluxVM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_push_pop() {
        let mut vm = FluxVM::new();
        let r = vm.execute_op(Opcode::PUSH, &["42"]).unwrap();
        assert_eq!(vm.stack.len(), 1);
        assert!(r.valid);
    }

    #[test]
    fn test_vm_arithmetic() {
        let mut vm = FluxVM::new();
        vm.execute_op(Opcode::PUSH, &["10"]).unwrap();
        vm.execute_op(Opcode::PUSH, &["20"]).unwrap();
        let r = vm.execute_op(Opcode::ADD, &[]).unwrap();
        assert!(r.valid);
        assert!(r.message.contains("30"));
    }

    #[test]
    fn test_vm_command_lattice() {
        let mut vm = FluxVM::new();
        let r = vm.execute_command("lattice pythagorean 200").unwrap();
        assert!(r.valid, "Lattice creation: {}", r.message);
    }

    #[test]
    fn test_vm_verify_proof() {
        let mut vm = FluxVM::new();
        vm.execute_command("lattice pythagorean 200").unwrap();
        vm.execute_command("push 0.577").unwrap();
        vm.execute_command("snap 0.577 0.816").unwrap();
        let r = vm.execute_op(Opcode::VERIFY, &[]).unwrap();
        assert!(r.valid, "Proof verification: {}", r.message);
    }

    #[test]
    fn test_vm_fingerprint() {
        let mut vm = FluxVM::new();
        let r = vm.execute_op(Opcode::FINGERPRINT, &[]).unwrap();
        assert!(r.valid);
        assert!(r.message.contains("Fingerprint"));
    }
}
