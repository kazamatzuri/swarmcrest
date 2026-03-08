#!/bin/bash
# Configure git to use the shared hooks from scripts/
# Run once after cloning the repo.
git config core.hooksPath scripts
echo "Git hooks configured (core.hooksPath = scripts)"
