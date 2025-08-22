#!/bin/bash

cp scripts/pre_commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
echo "Pre-commit hook installed"

