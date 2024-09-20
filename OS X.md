# Building and Running Hydroxite on macOS

## Prerequisites

- Rust (latest stable version)
- Xcode Command Line Tools

## Installation

1. Install Rust:
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Install Xcode Command Line Tools:
   ```
   xcode-select --install
   ```

3. Clone the repository:
   ```
   git clone https://github.com/yourusername/hydroxite.git
   cd hydroxite
   ```

4. Build the project:
   ```
   cargo build --release
   ```

5. Run Hydroxite:
   ```
   cargo run --release
   ```

## Troubleshooting

If you encounter any issues, please check the following:

- Ensure you have the latest version of Rust installed.
- Ensure you have the Xcode Command Line Tools installed.
- Check the project's GitHub issues page for any known issues or solutions.
