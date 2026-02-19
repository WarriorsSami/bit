# Git Commit Message Instructions

## Mandatory Format: Conventional Commits

All commit messages MUST follow the [Conventional Commits](https://www.conventionalcommits.org/) specification.

### Format Structure

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Rules

1. **Type** (required): Must be one of:
   - `feat`: A new feature
   - `fix`: A bug fix
   - `docs`: Documentation only changes
   - `style`: Changes that don't affect code meaning (whitespace, formatting, etc.)
   - `refactor`: Code change that neither fixes a bug nor adds a feature
   - `perf`: Performance improvement
   - `test`: Adding missing tests or correcting existing tests
   - `build`: Changes to build system or external dependencies
   - `ci`: Changes to CI configuration files and scripts
   - `chore`: Other changes that don't modify src or test files
   - `revert`: Reverts a previous commit

2. **Scope** (optional): A noun describing the section of codebase affected, in parentheses:
   - Examples: `(index)`, `(merge)`, `(checkout)`, `(diff)`, `(log)`, `(objects)`, `(refs)`, `(cli)`

3. **Description** (required):
   - Use imperative, present tense: "change" not "changed" nor "changes"
   - Don't capitalize first letter
   - No period (.) at the end
   - Keep under 72 characters

4. **Body** (optional):
   - Separate from subject with a blank line
   - Use imperative, present tense
   - Explain what and why, not how
   - Wrap at 72 characters

5. **Footer** (optional):
   - Use for breaking changes: `BREAKING CHANGE: <description>`
   - Use for issue references: `Fixes #123` or `Closes #456`

### Examples

#### Simple feature:
```
feat(merge): add three-way merge algorithm
```

#### Bug fix with scope:
```
fix(index): prevent file/dir path conflicts during add
```

#### Feature with body:
```
feat(checkout): detect uncommitted changes before switching branches

Add validation to check for uncommitted modifications in the workspace
that would be overwritten by the checkout operation. This prevents
data loss when switching branches.
```

#### Breaking change:
```
feat(refs)!: change revision parser to support ranges

BREAKING CHANGE: Revision::parse now returns a structured enum instead
of a simple string, requiring callers to pattern match on the result.
```

#### Multiple components:
```
refactor(objects): extract blob serialization into dedicated module

Move blob encoding/decoding logic from database module into objects
module for better separation of concerns.

Refs #45
```

### What NOT to do

❌ `Fixed a bug` (not imperative, no type)
❌ `feat: Added new feature.` (not imperative, has period)
❌ `update stuff` (no type, vague description)
❌ `FEAT: New feature` (type should be lowercase)
❌ `feat : add feature` (space before colon)

### Additional Guidelines

- Keep commits atomic (one logical change per commit)
- If you can't describe the change without using "and", split it into multiple commits
- Test-only changes should use `test:` type
- Documentation-only changes should use `docs:` type
- When in doubt, prefer `feat:` for new functionality and `fix:` for corrections

### Integration with Bit Project

For this Git implementation project, common scopes include:
- `(index)` - staging area/index operations
- `(objects)` - object database and serialization
- `(refs)` - reference handling and branch management
- `(merge)` - merge operations and conflict resolution
- `(diff)` - diff generation and comparison
- `(log)` - commit history traversal
- `(checkout)` - working tree updates
- `(status)` - status calculation
- `(cli)` - command-line interface
- `(tests)` - test infrastructure

Common types for this project:
- Use `feat:` when implementing new Git commands or features
- Use `fix:` when correcting behavior to match Git semantics
- Use `test:` when adding test coverage
- Use `refactor:` when improving code structure without changing behavior
- Use `docs:` when updating README or inline documentation

