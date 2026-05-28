/// SHA-256 proof chain — every operation appends to a hash chain.
/// This is the "provably correct execution" foundation.
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::fmt;

/// A single link in the proof chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofLink {
    /// The hash of the previous link (all zeros for root)
    pub prev_hash: String,
    /// The operation that produced this proof
    pub operation: String,
    /// The result of the operation
    pub result: String,
    /// The hash = SHA256(prev_hash || operation || result)
    pub hash: String,
    /// Cycle number at which this link was created
    pub cycle: u16,
}

impl fmt::Display for ProofLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "#{} {} → {} [{}]",
            self.cycle, self.operation, self.result, &self.hash[..16]
        )
    }
}

/// A SHA-256 proof certificate chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofChain {
    /// All links in the chain
    links: Vec<ProofLink>,
    /// The root hash (SHA256 of empty string for genesis)
    root_hash: String,
    /// Current chain length
    length: usize,
    /// Whether the chain has been tampered with
    tampered: bool,
}

impl ProofChain {
    /// Create a new empty proof chain.
    pub fn new() -> Self {
        let root = hex::encode(Sha256::digest(b"FLUX-GENESIS"));
        ProofChain {
            links: Vec::new(),
            root_hash: root.clone(),
            length: 0,
            tampered: false,
        }
    }

    /// Append a new operation to the proof chain.
    /// hash(n) = SHA256(hash(n-1) || operation || result)
    pub fn extend(&mut self, operation: &str, result: &str) -> String {
        let prev_hash = if self.links.is_empty() {
            &self.root_hash
        } else {
            &self.links.last().unwrap().hash
        };

        let input = format!("{}||{}||{}", prev_hash, operation, result);
        let hash = hex::encode(Sha256::digest(input.as_bytes()));

        let link = ProofLink {
            prev_hash: prev_hash.clone(),
            operation: operation.to_string(),
            result: result.to_string(),
            hash: hash.clone(),
            cycle: self.length as u16,
        };

        self.links.push(link);
        self.length += 1;
        hash
    }

    /// Verify the entire proof chain is intact.
    /// Returns (valid: bool, detail: String)
    pub fn verify(&self) -> (bool, String) {
        if self.links.is_empty() {
            return (true, "Empty chain (valid by definition)".to_string());
        }

        for (i, link) in self.links.iter().enumerate() {
            let expected_prev = if i == 0 {
                &self.root_hash
            } else {
                &self.links[i - 1].hash
            };

            // Check previous hash link
            if link.prev_hash != *expected_prev {
                return (
                    false,
                    format!(
                        "Chain broken at link {}: prev_hash mismatch (expected {}, got {})",
                        i, expected_prev, link.prev_hash
                    ),
                );
            }

            // Check hash integrity
            let input = format!("{}||{}||{}", link.prev_hash, link.operation, link.result);
            let expected_hash = hex::encode(Sha256::digest(input.as_bytes()));
            if link.hash != expected_hash {
                return (
                    false,
                    format!(
                        "Chain broken at link {}: hash mismatch (expected {}, got {})",
                        i, expected_hash, link.hash
                    ),
                );
            }
        }

        (true, format!("Proof chain valid ({} steps, no violations)", self.length))
    }

    /// Get the current hash (last link's hash, or root if empty).
    pub fn current_hash(&self) -> String {
        self.links.last().map(|l| l.hash.clone()).unwrap_or_else(|| self.root_hash.clone())
    }

    /// Get the root hash.
    pub fn root_hash(&self) -> &str {
        &self.root_hash
    }

    /// Get all links.
    pub fn links(&self) -> &[ProofLink] {
        &self.links
    }

    /// Number of links.
    pub fn len(&self) -> usize {
        self.length
    }

    /// Serialize the proof chain to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    /// Load a proof chain from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Mark the chain as tampered (for testing).
    pub fn mark_tampered(&mut self) {
        self.tampered = true;
    }

    /// Check if tampered.
    pub fn is_tampered(&self) -> bool {
        self.tampered
    }
}

impl Default for ProofChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_chain() {
        let chain = ProofChain::new();
        let (valid, msg) = chain.verify();
        assert!(valid, "Empty chain should be valid: {}", msg);
    }

    #[test]
    fn test_extend_and_verify() {
        let mut chain = ProofChain::new();
        chain.extend("PUSH 440.0", "440.0");
        chain.extend("SNAP pythagorean", "440.0 (exact)");
        chain.extend("CHECK |v|² == 1.0", "true");

        let (valid, msg) = chain.verify();
        assert!(valid, "Chain should be valid: {}", msg);
        assert_eq!(chain.len(), 3, "Should have 3 links");
    }

    #[test]
    fn test_integrity_check() {
        let mut chain = ProofChain::new();
        chain.extend("OP1", "result1");
        chain.extend("OP2", "result2");

        // Manually tamper with a link
        if let Some(link) = chain.links.first_mut() {
            link.result = "MANIPULATED".to_string();
        }

        let (valid, _) = chain.verify();
        assert!(!valid, "Tampered chain should be invalid");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut chain = ProofChain::new();
        chain.extend("TEST", "data");
        let json = chain.to_json();
        let restored = ProofChain::from_json(&json).unwrap();
        assert_eq!(chain.len(), restored.len());
        assert_eq!(chain.current_hash(), restored.current_hash());
    }
}
