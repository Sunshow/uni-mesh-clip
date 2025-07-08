### 🔄 Project Awareness & Context
- **Always read `PLANNING.md`** at the start of a new conversation to understand the project's architecture, goals, style, and constraints.
- **Check `TASK.md`** before starting a new task. If the task isn't listed, add it with a brief description and today's date.
- **Use consistent naming conventions, file structure, and architecture patterns** as described in `PLANNING.md`.

### 🧱 Code Structure & Modularity
- **Never create a file longer than 500 lines of code.** If a file approaches this limit, refactor by splitting it into modules or helper files.
- **Organize code into clearly separated modules**, grouped by feature or responsibility.
- **Use clear, consistent imports** (prefer relative imports within packages, ES6 imports for frontend).

### 🏗️ Tauri 2.0 Architecture
- **Follow Tauri 2.0 conventions**: Use `src-tauri/` for Rust backend, `src/` for frontend.
- **Use the new Tauri 2.0 API patterns**: Leverage the updated command system, events, and plugin architecture.
- **Separate concerns**: Keep business logic in Rust commands, UI logic in frontend, and shared types in both.
- **Use Tauri's built-in security features**: Validate all inputs, use CSP headers, and follow the principle of least privilege.

### 🦀 Rust Backend Standards
- **Use `#[tauri::command]` for all backend functions** exposed to the frontend.
- **Follow Rust conventions**: Use `snake_case` for functions/variables, `PascalCase` for types/structs.
- **Use `serde` for serialization** with proper derive macros.
- **Handle errors properly** using `Result<T, E>` and custom error types.
- **Use `tokio` for async operations** when needed.
- Write **documentation comments** using `///`:
  ```rust
  /// Brief summary of the function.
  ///
  /// # Arguments
  /// * `param1` - Description of parameter
  ///
  /// # Returns
  /// Description of return value
  ///
  /// # Errors
  /// When this function might return an error
  #[tauri::command]
  async fn example_command(param1: String) -> Result<String, String> {
      // Implementation
  }
  ```

### 🌐 Frontend Standards
- **Use TypeScript** for type safety and better development experience.
- **Use the Tauri 2.0 API**: Import from `@tauri-apps/api/core` and other v2 modules.
- **Create type definitions** that match Rust structs for seamless communication.
- **Use modern JavaScript/TypeScript patterns**: async/await, destructuring, optional chaining.
- **Follow consistent naming**: `camelCase` for JavaScript/TypeScript.
- Write **JSDoc comments** for complex functions:
  ```typescript
  /**
   * Brief summary of the function.
   * @param param1 - Description of parameter
   * @returns Description of return value
   */
  async function exampleFunction(param1: string): Promise<string> {
      // Implementation
  }
  ```

### 🧪 Testing & Reliability
- **Create unit tests for Rust commands** using `#[cfg(test)]` and `#[tokio::test]`.
- **Test frontend functionality** using your preferred testing framework (Jest, Vitest, etc.).
- **Test Tauri integration** with end-to-end tests when possible.
- **Tests should live in appropriate locations**:
  - Rust: `tests/` folder or inline with `#[cfg(test)]`
  - Frontend: `tests/` or `__tests__/` folders
  - Include at least:
    - 1 test for expected use
    - 1 edge case  
    - 1 failure case

### 🔧 Configuration & Dependencies
- **Keep `Cargo.toml` organized** with clear feature flags and dependencies.
- **Use `tauri.conf.json` properly** for app configuration, permissions, and security settings.
- **Pin important dependency versions** to ensure reproducible builds.
- **Use Tauri plugins** when available instead of reinventing functionality.

### ✅ Task Completion
- **Mark completed tasks in `TASK.md`** immediately after finishing them.
- Add new sub-tasks or TODOs discovered during development to `TASK.md` under a "Discovered During Work" section.
- **Test both frontend and backend** before marking tasks complete.

### 📎 Style & Conventions
- **Use `rustfmt`** for Rust code formatting and **`prettier`** for frontend code.
- **Use `clippy`** for Rust linting and **`eslint`** for TypeScript/JavaScript.
- **Follow Tauri's security best practices**: validate inputs, use proper permissions, sanitize data.
- **Use consistent error handling patterns** across both frontend and backend.

### 📚 Documentation & Explainability  
- **Update `README.md`** when new features are added, dependencies change, or setup steps are modified.
- **Document Tauri commands and their usage** in both Rust and TypeScript.
- **Comment non-obvious code** and ensure everything is understandable to a mid-level developer.
- When writing complex logic, **add inline comments** explaining the why, not just the what.
- **Document any custom Tauri configurations or plugins** used.

### 🧠 AI Behavior Rules
- **Never assume missing context. Ask questions if uncertain.**
- **Never hallucinate Tauri APIs or Rust crates** – only use verified Tauri 2.0 APIs and established crates.
- **Always confirm file paths and module names** exist before referencing them in code or tests.
- **Never delete or overwrite existing code** unless explicitly instructed to or if part of a task from `TASK.md`.
- **Check Tauri 2.0 documentation** for breaking changes from v1 when working with existing projects.
- **Verify frontend-backend type consistency** when creating or modifying data structures.