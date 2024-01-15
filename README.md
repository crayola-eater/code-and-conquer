# Purpose

- An in-progress Rust rewrite of https://github.com/mancjs/code-and-conquer, which is a strategy + coding game (already written by others), where teams compete to conquer squares on a finite grid by programmatically submitting commands to an authoritative TCP server from their respective TCP client. This project was written to get more practice in Rust and configuring a Rust project for CI (GitHub Actions).

# Roadmap
- Most of the work so far has gone into the `game_core` module, which by design is decoupled from the `server`. The decoupling should allow for experimenting with different protocols for exchange between client and server (e.g. WebSocket, TCP, QUIC, HTTP, GRPC) and picking a decent fit.

  - [x] Set up CI checks with GitHub Actions
    - [x] Configure integration tests
    - [x] Configure formatting checks
    - [x] Configure Clippy
    - [x] Configure unit tests
  - [x] Ability to attack a square
  - [x] Ability to defend a square
  - [ ] Implement database/persistence layer
  - [ ] Ability to use roles
    - [ ] Spy
    - [ ] Minelayer
    - [ ] Cloaker 
  - [ ] Ability to query grid
  - [ ] Ability to query specific grid
  - [ ] Introduce environment variables
  - [ ] Experiment with different client-server protocols
    - [ ] Try TCP listener from Tokio
    - [ ] Try Tungstenite (WebSocket)
    - [ ] Try Tonic (GRPC)
    - [ ] Try QUIC
  - [ ] Implement server

# How to run

- Install Rust, Cargo, LLVM
- Clone this repo
- `cd` into cloned directory and then `cargo run`

# Tools used

- Rust standard library only
- Rust crates (see `Cargo.toml` for details)

# Created

- Jul 2023
