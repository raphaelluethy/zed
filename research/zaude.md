# Porting ACP Todo List to Normal Zed Agent

## Research Notes

### Current Architecture Analysis

#### ACP Todo List Implementation
The ACP (Agent Control Protocol) Todo List is implemented in the `agent_ui` crate with these key components:

**Location**: `crates/agent_ui/src/acp/thread_view.rs`
- **Plan Structure** (`acp_thread/src/acp_thread.rs`):
  - `Plan` struct containing a `Vec<PlanEntry>`
  - `PlanEntry` with content (Markdown), priority, and status
  - `PlanEntryStatus` enum: `Pending`, `InProgress`, `Completed`

- **UI Components**:
  - Visual representation with icons for each status
  - Icons: `TodoPending`, `TodoProgress`, `TodoComplete`
  - Markdown rendering with strikethrough for completed items
  - Status management through ACP protocol

**Key Files**:
- `crates/agent_ui/src/acp/thread_view.rs` - UI rendering
- `crates/acp_thread/src/acp_thread.rs` - Core data structures
- `crates/agent_ui/src/acp/message_editor.rs` - Integration points

#### Normal Zed Agent Implementation
The normal Zed agent (`agent2` crate) uses a tool-based architecture:

**Location**: `crates/agent2/src/`
- **Tool System** (`tools.rs`):
  - Tools implement the `AgentTool` trait
  - Tools are registered via `add_default_tools()` method in `thread.rs`
  - Each tool has input/output types and async execution

- **Thread Management** (`thread.rs`):
  - `Thread` struct manages conversation state
  - Tools are stored in a `BTreeMap<SharedString, Arc<dyn AnyAgentTool>>`
  - No built-in plan/todo management system

**Key Files**:
- `crates/agent2/src/tools.rs` - Tool registry and exports
- `crates/agent2/src/thread.rs` - AgentTool trait and tool management
- `crates/agent2/src/tools/diagnostics_tool.rs` - Example tool implementation

### Implementation Requirements

To port the ACP Todo List to the normal Zed agent, the following components need to be created:

#### 1. Todo List Tool Implementation
Create a new tool that follows the `AgentTool` trait pattern:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TodoListToolInput {
    pub action: TodoAction, // "create", "update", "list", "clear"
    pub todos: Option<Vec<TodoItem>>,
    pub todo_id: Option<String>,
    pub status: Option<TodoStatus>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
    pub priority: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

pub struct TodoListTool {
    project: Entity<Project>,
    // Store todo state
    todos: RwLock<Vec<TodoItem>>,
}
```

#### 2. AgentTool Trait Implementation
```rust
impl AgentTool for TodoListTool {
    type Input = TodoListToolInput;
    type Output = String;

    fn name() -> &'static str {
        "todo_list"
    }

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Write // Or appropriate kind
    }

    fn run(/* ... */) -> Task<Result<Self::Output>> {
        // Handle different actions: create, update, list, clear
    }
}
```

#### 3. Tool Registration
Add the tool to `agent2/src/thread.rs` in the `add_default_tools()` method:

```rust
self.add_tool(TodoListTool::new(self.project.clone()));
```

#### 4. Tool Registry Update
Add the tool to the list in `agent2/src/tools.rs`:

```rust
pub fn default_tool_names() -> impl Iterator<Item = &'static str> {
    [
        // ... existing tools
        TodoListTool::name(),
    ].into_iter()
}
```

#### 5. (Optional) UI Components
Create UI components similar to the ACP implementation:
- Todo list view component
- Individual todo item components
- Status indicators with icons

### Key Differences to Address

1. **State Management**:
   - ACP todo lists are managed by the external agent through the protocol
   - Normal agent tool would need to maintain its own state
   - Consider persistence using project settings or user preferences

2. **UI Integration**:
   - ACP has dedicated UI components in `agent_ui/src/acp/`
   - Normal agent tools don't have built-in UI components
   - Would need to create custom UI or integrate with existing agent panel

3. **Protocol Integration**:
   - ACP uses agent control protocol for real-time updates
   - Normal agent uses tool calls with request/response pattern
   - Less real-time capability, more state-based

### Recommended Implementation Approach

1. **Phase 1: Basic Tool Functionality**
   - Create `TodoListTool` implementing `AgentTool` trait
   - Support basic CRUD operations (Create, Read, Update, Delete)
   - Use in-memory storage initially

2. **Phase 2: Persistence**
   - Add persistence using project or user settings
   - Implement save/load functionality
   - Consider using the existing database patterns from `agent2/src/db.rs`

3. **Phase 3: Enhanced Features**
   - Add priority support
   - Implement filtering and sorting
   - Add due dates or reminders

4. **Phase 4: UI Integration (Optional)**
   - Create dedicated UI components
   - Integrate with agent panel
   - Add visual indicators and icons

### Implementation Considerations

#### Error Handling
- Follow the pattern from `DiagnosticsTool` for proper error propagation
- Use `anyhow::Result` for consistent error handling
- Provide meaningful error messages for users

#### Tool Design Patterns
- Study existing tools like `DiagnosticsTool`, `EditFileTool`, etc.
- Follow the same input validation patterns
- Use appropriate `ToolKind` classification

#### State Management Options
1. **In-memory**: Simple but not persistent
2. **Project-based**: Store in project settings
3. **User-based**: Store in user preferences
4. **Database**: Use existing SQLite patterns

#### Integration Points
- `agent2/src/tools.rs` - Tool registration
- `agent2/src/thread.rs` - Tool management
- `crates/settings/` - For persistence options
- `crates/agent_ui/` - For UI components (if needed)

### Files to Modify/Created

**New Files:**
- `crates/agent2/src/tools/todo_list_tool.rs` - Main tool implementation
- `crates/agent2/src/tools/todo_types.rs` - Type definitions (optional)

**Modified Files:**
- `crates/agent2/src/tools.rs` - Add tool to module exports
- `crates/agent2/src/tools/mod.rs` - Add module declaration
- `crates/agent2/src/thread.rs` - Register tool in `add_default_tools()`

### Testing Strategy

1. **Unit Tests**: Test individual tool functions
2. **Integration Tests**: Test tool integration with agent system
3. **UI Tests**: Test UI components (if implemented)
4. **E2E Tests**: Test complete workflow

Follow existing test patterns from `agent2/src/tests/` and other tool implementations.

## Next Steps

1. Create basic tool structure following `DiagnosticsTool` pattern
2. Implement core CRUD functionality
3. Add persistence mechanism
4. Consider UI integration requirements
5. Write comprehensive tests

---

*Document created: 2025-09-26*
*Research based on Zed codebase analysis*