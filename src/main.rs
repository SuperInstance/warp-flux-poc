//! Warp FLUX PoC — Constraint-Native Terminal REPL
//!
//! A proof-of-concept demonstrating FLUX constraint execution
//! in a Warp-like terminal environment.
//!
//! Features:
//! - Stack-based FLUX-C interpreter (20 opcodes)
//! - Pythagorean lattice snapping with Euclid's formula
//! - SHA-256 proof chain for provably correct execution
//! - Conservation analysis via Laplacian power iteration
//! - Spectral fingerprinting
//! - Warp-styled colored terminal output

mod lattice;
mod proof;
mod vm;
mod conservation;

use std::io::{self, BufRead, Write};

/// ANSI color codes for Warp-styled terminal output.
mod style {
    /// Reset to default
    pub const RESET: &str = "\x1b[0m";
    /// Green (success)
    pub const GREEN: &str = "\x1b[32m";
    /// Red (error/constraint violation)
    pub const RED: &str = "\x1b[31m";
    /// Cyan (proof/hashes)
    pub const CYAN: &str = "\x1b[36m";
    /// Yellow (warnings/info)
    pub const YELLOW: &str = "\x1b[33m";
    /// Bold
    pub const BOLD: &str = "\x1b[1m";
    /// Dim
    pub const DIM: &str = "\x1b[2m";
    /// Blue (for info)
    pub const BLUE: &str = "\x1b[34m";
    /// Magenta (for lattice)
    pub const MAGENTA: &str = "\x1b[35m";
}

/// Print the Warp-styled prompt.
fn print_prompt(cycle: u16) {
    let prompt_color = if cycle > 3000 { style::RED } else { style::GREEN };
    print!(
        "{prompt_color}flux@warp{reset} {prompt_arrow}❯{reset} ",
        prompt_color = style::CYAN,
        reset = style::RESET,
        prompt_arrow = prompt_color,
    );
    io::stdout().flush().unwrap();
}

/// Print a result with appropriate coloring.
fn print_result(msg: &str, valid: bool) {
    if !valid || msg.contains("VIOLATED") || msg.contains("broken") || msg.contains("Error") {
        println!("  {red}✗{reset} {bold}{msg}{reset}", red = style::RED, reset = style::RESET, bold = style::BOLD, msg = msg);
    } else if msg.contains("Proof") || msg.contains("Fingerprint") || msg.contains("Hashed") || msg.contains("Certificate") {
        println!("  {cyan}▪{reset} {bold}{msg}{reset}", cyan = style::CYAN, reset = style::RESET, bold = style::BOLD, msg = msg);
    } else if msg.contains("Lattice") || msg.contains("Snapped") {
        println!("  {magenta}◆{reset} {bold}{msg}{reset}", magenta = style::MAGENTA, reset = style::RESET, bold = style::BOLD, msg = msg);
    } else if msg.contains("Conservation") {
        if msg.contains("conserved") || msg.contains("✓") || valid {
            println!("  {green}🔒 {bold}{msg}{reset}", green = style::GREEN, reset = style::RESET, bold = style::BOLD, msg = msg);
        } else {
            println!("  {red}🔒 {bold}{msg}{reset}", red = style::RED, reset = style::RESET, bold = style::BOLD, msg = msg);
        }
    } else {
        println!("  {msg}", msg = msg);
    }
}

/// Show the welcome banner.
fn print_welcome() {
    println!();
    println!("{cyan}╔══════════════════════════════════════════════════╗{reset}", cyan = style::CYAN, reset = style::RESET);
    println!("{cyan}║{reset}  {bold}FLUX Constraint Execution Engine {reset}{dim}v0.1.0{reset}        {cyan}║{reset}",
        cyan = style::CYAN, reset = style::RESET, bold = style::BOLD, dim = style::DIM);
    println!("{cyan}║{reset}  {dim}Warp Plugin Architecture — Proof of Concept{reset} {cyan}║{reset}",
        cyan = style::CYAN, reset = style::RESET, dim = style::DIM);
    println!("{cyan}╚══════════════════════════════════════════════════╝{reset}", cyan = style::CYAN, reset = style::RESET);
    println!();
    println!(" {bold}Commands:{reset}", bold = style::BOLD, reset = style::RESET);
    println!("  {green}lattice pythagorean <N>{reset}   Create Pythagorean lattice (N=precision)", green = style::GREEN, reset = style::RESET);
    println!("  {green}snap <x> <y>{reset}              Snap to nearest exact point", green = style::GREEN, reset = style::RESET);
    println!("  {green}push <val>{reset}                Push value onto stack", green = style::GREEN, reset = style::RESET);
    println!("  {green}check{reset}                     Verify constraint |v|² == 1.0", green = style::GREEN, reset = style::RESET);
    println!("  {green}conserve tension <t>{reset}      Set conservation threshold", green = style::GREEN, reset = style::RESET);
    println!("  {green}analyze <json>{reset}            Load & analyze transition graph", green = style::GREEN, reset = style::RESET);
    println!("  {green}fingerprint{reset}               Show spectral fingerprint", green = style::GREEN, reset = style::RESET);
    println!("  {green}verify{reset}                    Verify proof chain integrity", green = style::GREEN, reset = style::RESET);
    println!("  {green}halt{reset} / {green}exit{reset}              Exit REPL", green = style::GREEN, reset = style::RESET);
    println!("  {green}+, -, *, /{reset}               Arithmetic on stack", green = style::GREEN, reset = style::RESET);
    println!("  {green}pop, dup, swap{reset}            Stack manipulation", green = style::GREEN, reset = style::RESET);
    println!();
}

/// Run the demo script non-interactively.
fn run_demo_script() {
    let cmds = vec![
        "lattice pythagorean 200",
        "push 0.577",
        "snap 0.577 0.816",
        "push 0.707",
        "check",
        "push 0.6",
        "push 0.8",
        "check",
        "verify",
        "fingerprint",
        "conserve tension 0.001",
        "analyze {\"nodes\":[0.6,0.8,0.577,0.816],\"edges\":[[0,1],[1,2],[2,3]]}",
        "verify",
        "halt",
    ];

    let mut vm = vm::FluxVM::new();

    for cmd in &cmds {
        if cmd.trim().is_empty() {
            println!();
            continue;
        }

        print_prompt(vm.cycles());
        println!("{}", cmd);

        match vm.execute_command(cmd) {
            Ok(result) => {
                print_result(&result.message, result.valid);
                if !result.stack.is_empty() {
                    println!("  {dim}Stack:{reset} {count} items", dim = style::DIM, reset = style::RESET, count = result.stack.len());
                }
            }
            Err(e) => {
                print_result(&format!("Error: {}", e), false);
            }
        }
    }

    // Save proof certificate
    let proof_json = vm.proof_chain().to_json();
    std::fs::write("proof.cert", &proof_json).unwrap_or_else(|e| {
        eprintln!(
            "  {yellow}Warning:{reset} Could not save proof.cert: {e}",
            yellow = style::YELLOW,
            reset = style::RESET,
            e = e
        );
    });
    println!();
    println!(
        "  {cyan}└─ Proof certificate saved to proof.cert{reset}",
        cyan = style::CYAN,
        reset = style::RESET
    );
    println!();
}

/// Run the interactive REPL.
fn run_repl() {
    let mut vm = vm::FluxVM::new();
    let stdin = io::stdin();

    // Load proof.cert if it exists
    if let Ok(json) = std::fs::read_to_string("proof.cert") {
        if let Ok(chain) = proof::ProofChain::from_json(&json) {
            println!(
                "  {dim}Loaded proof certificate ({} operations){reset}",
                chain.len(),
                dim = style::DIM,
                reset = style::RESET
            );
        }
    }

    loop {
        print_prompt(vm.cycles());

        let mut line = String::new();
        stdin.lock().read_line(&mut line).unwrap_or(0);
        let line = line.trim().to_string();

        if line.is_empty() {
            continue;
        }

        match vm.execute_command(&line) {
            Ok(result) => {
                print_result(&result.message, result.valid);
                if !result.stack.is_empty() {
                    println!(
                        "  {dim}Stack:{reset} {} items",
                        result.stack.len(),
                        dim = style::DIM,
                        reset = style::RESET
                    );
                    for (i, val) in result.stack.iter().rev().take(5).enumerate() {
                        println!("    {}▶{} {}", style::DIM, style::RESET, val);
                    }
                    if result.stack.len() > 5 {
                        println!(
                            "    {}... {} more{}",
                            style::DIM,
                            result.stack.len() - 5,
                            style::RESET
                        );
                    }
                }

                if !vm.is_running() {
                    println!();
                    println!(
                        "  {yellow}VM halted. {cycles} cycles, proof hash: {hash}...{reset}",
                        yellow = style::YELLOW,
                        cycles = vm.cycles(),
                        hash = &result.proof_hash[..16],
                        reset = style::RESET
                    );
                    break;
                }
            }
            Err(e) => {
                if line == "halt" || line == "exit" || line == "quit" {
                    println!("  {yellow}Goodbye.{reset}", yellow = style::YELLOW, reset = style::RESET);
                    break;
                }
                print_result(&format!("Error: {}", e), false);
            }
        }
    }
}

fn main() {
    print_welcome();

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--demo" {
        run_demo_script();
    } else {
        run_repl();
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_demo_flow() {
        let mut vm = vm::FluxVM::new();

        assert!(vm.execute_command("lattice pythagorean 200").unwrap().valid);
        assert!(vm.execute_command("push 0.577").unwrap().valid);

        let r = vm.execute_command("snap 0.577 0.816").unwrap();
        assert!(r.valid, "Snap should succeed");

        let r = vm.execute_command("verify").unwrap();
        assert!(r.valid, "Verify should pass: {}", r.message);

        let r = vm.execute_command("fingerprint").unwrap();
        assert!(r.valid);
    }
}
