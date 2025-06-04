# Soroban Timelock Claimable Balance Contract

A secure, efficient smart contract implementation for Stellar's Soroban platform that enables time-bound token deposits with multi-party claiming functionality.

## Core Concept
This contract allows users to lock tokens under customizable time conditions, granting specified claimants access based on:
- **Preemptive claims** (before a set timestamp)
- **Future claims** (after a set timestamp)

## Key Features
- **Flexible Time Constraints**  
  Define `Before` or `After` timestamps for claim eligibility
- **Multi-Claimant Support**  
  Authorize up to 10 distinct claimant addresses
- **Secure Initialization**  
  One-time setup prevents reinitialization attacks
- **Minimalist Design**  
  Optimized for Soroban's resource-constrained environment
- **Authorization Enforcement**  
  Strict claimant verification at claim execution

## Smart Contract Architecture

### Data Model
```rust
// Time constraint type
pub enum TimeBoundKind {
    Before, // Claimable ONLY before timestamp
    After   // Claimable ONLY after timestamp
}

// Time condition specification
pub struct TimeBound {
    pub kind: TimeBoundKind,
    pub timestamp: u64,
}

// Claimable balance container
pub struct ClaimableBalance {
    pub token: Address,          // Token address
    pub amount: i128,            // Token amount
    pub claimants: Vec<Address>, // Eligible claimants (max 10)
    pub time_bound: TimeBound,   // Time restriction
}

## Storage Architecture

### Core Storage Keys
- **`Init`**  
  Initialization status flag (`bool`) ensuring one-time contract setup
- **`Balance`**  
  Primary storage slot for `ClaimableBalance` struct

## Technical Implementation

### Development Specifications
- **Language:** Rust (no_std compatibility)
- **SDK:** Soroban 0.9.4
- **Memory Model:** Zero-copy storage design
- **Compiler:** Stable Rust 1.70+

```rust
// Storage key declaration example
pub enum DataKey {
    Init,    // Initialization flag
    Balance  // ClaimableBalance storage
}

