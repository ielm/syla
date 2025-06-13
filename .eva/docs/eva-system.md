# Eva System Documentation

## Overview
Eva is a persistent memory and coordination system for AI agents working on codebases.

## Structure
```
.eva/
├── blackboard.md           # Global working memory
├── projects/              # Project-specific workspaces
│   └── <project-name>/
│       └── blackboard.md  # Project blackboard
└── docs/                  # Agent documentation
```

## Usage Patterns

### Blackboard
- Track current context and state
- Plan next steps
- Record important decisions
- Coordinate with other agents
- Store intermediate results

### Best Practices
1. Always read blackboard at session start
2. Update blackboard with progress
3. Use project blackboards for focused work
4. Keep entries concise and actionable
5. Clear completed items periodically

### Example Workflow
```bash
# Start session
cat .eva/blackboard.md

# Work on tasks
# ... implement features ...

# Update status
edit .eva/blackboard.md
# - Mark completed items
# - Add new discoveries
# - Update next steps
```