# Syla Cursor Rules

Hyper-minimal rules for AI-assisted development on the Syla platform.

## Rules Overview

- **project-context.mdc** - Always applied, provides platform context
- **eva-system.mdc** - Always applied, enforces Eva system usage
- **service-patterns.mdc** - Auto-attached for service development
- **rust-standards.mdc** - Auto-attached for Rust files
- **cli-patterns.mdc** - Auto-attached for CLI development
- **testing-patterns.mdc** - Auto-attached for test files
- **workspace-concepts.mdc** - Auto-attached for workspace code

## Usage

These rules automatically provide context to Cursor AI based on what you're working on. They ensure consistent patterns across the codebase and remind about Eva system usage.

To manually reference a rule in chat: `@rule-name`