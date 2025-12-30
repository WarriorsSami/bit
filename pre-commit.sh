#!/bin/sh
set -eu

# -------- ensure cargo is in PATH --------
# Git hooks run with minimal environment, so we need to source the cargo env
if [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi

# If cargo is still not in PATH, try common installation locations
if ! command -v cargo >/dev/null 2>&1; then
  # Try adding common cargo bin paths
  if [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
fi

# Final check - if cargo is still not found, print helpful error and exit
if ! command -v cargo >/dev/null 2>&1; then
  printf "Error: cargo command not found in PATH.\n"
  printf "Please ensure Rust/Cargo is installed and accessible.\n"
  printf "Current PATH: %s\n" "$PATH"
  exit 1
fi

# -------- safe color setup (no errors when TERM/TTY missing) --------
is_tty=0
# stdout is a terminal?
if [ -t 1 ]; then
  is_tty=1
fi

# Only enable colors if: stdout is a tty AND TERM is set AND tput works
if [ "$is_tty" -eq 1 ] && [ -n "${TERM:-}" ] && command -v tput >/dev/null 2>&1 && tput colors >/dev/null 2>&1; then
  GREEN="$(tput setaf 2)"
  RED="$(tput setaf 1)"
  NC="$(tput sgr0)"
  BOLD="$(tput bold)"
  NORM="$(tput sgr0)"
else
  GREEN=""; RED=""; NC=""; BOLD=""; NORM=""
fi
# --------------------------------------------------------------------

printf "Running pre-commit checks...\n"

if ! cargo check --workspace; then
  printf "cargo check: ......... %s%s%s\n" "$RED" "nok" "$NC"
  printf "%sPre-commit: Issues detected when calling 'cargo check'.%s\n" "$RED" "$NC"
  exit 1
fi
printf "cargo check: ......... %sok%s\n" "$GREEN" "$NC"

if ! cargo fmt -- --check; then
  printf "cargo rustfmt: ....... %s%s%s\n" "$RED" "nok" "$NC"
  printf "%sPre-commit: Code style issues detected with rustfmt.%s\n" "$RED" "$NC"
  exit 1
fi
printf "cargo rustfmt: ....... %sok%s\n" "$GREEN" "$NC"

if ! cargo clippy --all-targets -- -D warnings; then
  printf "cargo clippy: ........ %s%s%s\n" "$RED" "nok" "$NC"
  printf "%sPre-commit: Issues detected by clippy.%s\n" "$RED" "$NC"
  exit 1
fi
printf "cargo clippy: ........ %sok%s\n" "$GREEN" "$NC"

if ! cargo test; then
  printf "cargo test: .......... %s%s%s\n" "$RED" "nok" "$NC"
  printf "%sPre-commit: Issues were detected when running the test suite.%s\n" "$RED" "$NC"
  exit 1
fi
printf "cargo test: .......... %sok%s\n" "$GREEN" "$NC"

printf "\n%s%sSuccess: %s%sAll pre-commit checks passed ✅%s\n\n" "$GREEN" "$BOLD" "$NC" "$NORM" "$NC"

exit 0