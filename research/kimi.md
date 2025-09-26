# ACP Todo List Implementation Research & Porting Plan

## Current Architecture Analysis

### ACP Thread View UI
**Location**: `crates/agent_ui/src/acp/thread_view.rs`
- Main UI component for agent conversations (line 320+)
- Renders tool calls with cards, headers, and content via `render_tool_call()` (line 2072)
- Supports expandable/collapsible tool content via `expanded_tool_calls` HashSet
- Handles different tool statuses: Pending, InProgress, Completed, Failed, Canceled, Rejected, WaitingForConfirmation
- Tool rendering includes card layouts, borders, and action buttons

### Tool System Framework
**Location**: `crates/agent2/src/tools/` directory
- All tools implement `AgentTool` trait (defined in `crates/agent2/src/thread.rs:2119`)
- Required trait methods: `name()`, `kind()`, `run()`, `initial_title()`, `description()`
- Tools registered in `add_default_tools()` (`crates/agent2/src/thread.rs:1038`)
- Input/output types use JSON Schema validation via `JsonSchema` derive
- Tools can stream updates via `ToolCallEventStream`

### Existing Tool Implementations
**Core Tools** (all in `crates/agent2/src/tools/`):
- `diagnostics_tool.rs` - Project diagnostics
- `read_file_tool.rs` - File reading
- `edit_file_tool.rs` - File editing
- `terminal_tool.rs` - Command execution
- `thinking_tool.rs` - Simple thinking tool (good reference)
- `list_directory_tool.rs` - Directory listing
- `grep_tool.rs` - Text searching
- And 10+ more tools...

### Tool Registration Pattern
```rust
// In crates/agent2/src/thread.rs:1038
pub fn add_default_tools(&mut self, environment: Rc<dyn ThreadEnvironment>, cx: &mut Context<Self>) {
    self.add_tool(DiagnosticsTool::new(self.project.clone()));
    self.add_tool(ReadFileTool::new(self.project.clone(), self.action_log.clone()));
    // ... add new tool here
    self.add_tool(TodoListTool::new(self.project.clone()));
}
```

## Key Findings

### No Existing Todo List Tool
- **No dedicated todo/task management tool** exists in the ACP system
- Existing "task" system (`crates/task/src/`) is for build/development tasks (tests, builds, etc.)
- Markdown task lists are supported (`- [ ]`, `- [x]`) for checkbox rendering
- ACP tools framework is mature and ready for new tool implementation

### Tool UI Rendering Capabilities
- `render_tool_call()` method handles all tool UI presentation
- Supports card vs inline layouts based on tool kind
- Collapsible content with expand/collapse functionality
- Status-based rendering (different UI for pending vs completed)
- Markdown content support for rich text display
- Action buttons for user interaction (confirm, reject, etc.)

### Storage & Persistence Options
- File-based storage using existing file system tools
- Project root or `.zed/` directory for todo data
- JSON format for structured todo data
- Thread-specific vs global todo lists possible

## Implementation Plan

### Phase 1: Basic Todo List Tool
**File**: `crates/agent2/src/tools/todo_list_tool.rs`

```rust
use crate::{AgentTool, ToolCallEventStream};
use agent_client_protocol as acp;
use anyhow::Result;
use gpui::{App, SharedString, Task};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TodoListToolInput {
    /// Action to perform: "add", "list", "complete", "remove"
    action: String,
    /// Task description (required for "add" action)
    task: Option<String>,
    /// Task ID (required for "complete" and "remove" actions)
    task_id: Option<usize>,
}

pub struct TodoListTool {
    project: Entity<Project>,
}

impl AgentTool for TodoListTool {
    type Input = TodoListToolInput;
    type Output = String;
    
    fn name() -> &'static str {
        "todo_list"
    }
    
    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit // or Read depending on implementation
    }
    
    fn run(
        self: Arc<Self>,
        input: Self::Input,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output>> {
        // Implementation for CRUD operations
        // Store todos in .zed/todos.json or similar
    }
}
```

### Phase 2: Tool Registration
**File**: `crates/agent2/src/thread.rs:1038`
- Add `self.add_tool(TodoListTool::new(self.project.clone()));` to `add_default_tools()`

### Phase 3: Enhanced UI (Optional)
**File**: `crates/agent_ui/src/acp/thread_view.rs`
- Extend `render_tool_call_content()` for interactive checkboxes
- Add todo-specific rendering logic
- Support inline editing of todo items

### Phase 4: Advanced Features
- Due dates and priorities
- Project/context categorization
- Search and filtering
- Export/import functionality

## Technical Considerations

### Tool Kind Selection
- `acp::ToolKind::Edit` - For modifying todos
- `acp::ToolKind::Read` - For listing todos
- Consider if single tool handles both or split into separate tools

### Storage Strategy
- **Location**: `.zed/todos.json` in project root
- **Format**: JSON array of todo objects
- **Persistence**: Use existing file system tools for read/write
- **Concurrency**: Handle multiple simultaneous edits

### UI Integration
- **Default rendering**: Markdown list with checkboxes
- **Enhanced rendering**: Interactive UI with checkboxes, editing
- **Status display**: Completed vs pending visual distinction
- **Collapsible content**: For long todo lists

### Error Handling
- File system errors (permission, disk space)
- JSON parsing errors
- Invalid input validation
- Concurrent access conflicts

## Reference Implementations

### Simple Tool Example
See `crates/agent2/src/tools/thinking_tool.rs` for minimal tool implementation.

### Complex Tool Example  
See `crates/agent2/src/tools/diagnostics_tool.rs` for tool with project integration.

### UI Rendering Example
See `render_tool_call()` in `crates/agent_ui/src/acp/thread_view.rs:2072` for UI patterns.

## Next Steps

1. **Create basic tool structure** following `thinking_tool.rs` pattern
2. **Implement CRUD operations** with file-based storage
3. **Register tool** in the agent system
4. **Test basic functionality** with simple markdown output
5. **Enhance UI** with interactive elements if needed
6. **Add advanced features** like due dates, priorities, etc.

The existing framework provides excellent foundation for a todo list tool. The tool system is well-designed and the UI rendering is sophisticated enough to handle both basic markdown lists and enhanced interactive interfaces.