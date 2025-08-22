#!/bin/bash
set -eu

echo "Running pre-commit checks..."

echo "Running formatting check"
if ! cargo fmt --all -- --check; then
  echo "X Code formatting check failed!"
  echo "Try running 'cargo fmt' to fix formatting issues."
  exit 1
fi

echo " - Code formatting check successfull"

echo "Running linting check"
if ! cargo clippy --all-targets --all-features -- -D warnings; then
  echo "X clippy found issues!"
  echo "Please fix above warnings"
  exit 1
fi

echo " - Linting check successfull"
