use crate::{AgentTool, ToolCallEventStream};
use agent_client_protocol as acp;
use anyhow::{Context as _, Result, anyhow};
use gpui::{App, Entity, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use util::{markdown::MarkdownCodeBlock, uuid::Uuid};

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, PartialEq, Eq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct TodoItem {
    /// Stable identifier for referencing the todo item across tool invocations.
    pub id: Uuid,
    /// Human friendly description of the todo item.
    pub content: String,
    /// Current completion state of the todo item.
    pub status: TodoStatus,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy)]
pub enum TodoAction {
    Create,
    Update,
    List,
    Clear,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct TodoListToolInput {
    /// Operation to perform on the todo list.
    pub action: TodoAction,
    /// Optional todo item content, used when creating or updating.
    #[serde(default)]
    pub content: Option<String>,
    /// Optional todo identifier for targeted operations.
    #[serde(default)]
    pub todo_id: Option<Uuid>,
    /// Optional status update when modifying an existing todo.
    #[serde(default)]
    pub status: Option<TodoStatus>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TodoListToolOutput {
    pub todos: Vec<TodoItem>,
}

pub struct TodoListTool {
    project: Entity<Project>,
}

impl TodoListTool {
    fn get_todos_path(&self, cx: &App) -> Result<PathBuf> {
        let project = self.project.read(cx);
        let worktree = project
            .worktrees()
            .next()
            .ok_or_else(|| anyhow!("No worktree found"))?;

        let worktree_path = worktree.read(cx).abs_path();
        Ok(worktree_path.join(".zed").join("todos.json"))
    }

    fn load_todos(&self, cx: &App) -> Result<Vec<TodoItem>> {
        let todos_path = self.get_todos_path(cx)?;

        if !todos_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&todos_path)
            .with_context(|| format!("Failed to read todos file: {}", todos_path.display()))?;

        serde_json::from_str(&content)
            .with_context(|| "Failed to parse todos file")
    }

    fn save_todos(&self, todos: &[TodoItem], cx: &App) -> Result<()> {
        let todos_path = self.get_todos_path(cx)?;

        // Create .zed directory if it doesn't exist
        if let Some(parent) = todos_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let content = serde_json::to_string_pretty(todos)
            .with_context(|| "Failed to serialize todos")?;

        std::fs::write(&todos_path, content)
            .with_context(|| format!("Failed to write todos file: {}", todos_path.display()))
    }
}

impl TodoListTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for TodoListTool {
    type Input = TodoListToolInput;
    type Output = TodoListToolOutput;

    fn name() -> &'static str {
        "todo_list"
    }

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Write
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(input) => match input.action {
                TodoAction::Create => "Create todo".into(),
                TodoAction::Update => "Update todo".into(),
                TodoAction::List => "List todos".into(),
                TodoAction::Clear => "Clear todos".into(),
            },
            Err(_) => "Todo List".into(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: Self::Input,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output>> {
        let load_result = self.load_todos(cx);
        let mut todos = match load_result {
            Ok(todos) => todos,
            Err(e) => {
                event_stream.update_fields(acp::ToolCallUpdateFields {
                    content: Some(vec![format!("Warning: Failed to load existing todos: {}. Starting with empty list.", e).into()]),
                    ..Default::default()
                });
                Vec::new()
            }
        };

        let output = match input.action {
            TodoAction::Create => {
                if let Some(content) = input.content {
                    let new_todo = TodoItem {
                        id: Uuid::new_v4(),
                        content,
                        status: TodoStatus::Pending,
                    };
                    todos.push(new_todo.clone());

                    match self.save_todos(&todos, cx) {
                        Ok(_) => {
                            event_stream.update_fields(acp::ToolCallUpdateFields {
                                content: Some(vec![format!("Created todo: {}", new_todo.content).into()]),
                                ..Default::default()
                            });
                            Ok(TodoListToolOutput { todos })
                        }
                        Err(e) => Err(anyhow!("Failed to save todos: {}", e)),
                    }
                } else {
                    Err(anyhow!("Content is required when creating a todo"))
                }
            }
            TodoAction::Update => {
                if let Some(todo_id) = input.todo_id {
                    let mut found = false;
                    let pre_update_content;
                    let post_update_content;

                    for todo in &mut todos {
                        if todo.id == todo_id {
                            pre_update_content = format!("{} ({:?})", todo.content, todo.status);

                            if let Some(content) = &input.content {
                                todo.content = content.clone();
                            }
                            if let Some(status) = input.status {
                                todo.status = status;
                            }

                            post_update_content = format!("{} ({:?})", todo.content, todo.status);
                            found = true;

                            event_stream.update_fields(acp::ToolCallUpdateFields {
                                content: Some(vec![format!("Updated todo from '{}' to '{}'", pre_update_content, post_update_content).into()]),
                                ..Default::default()
                            });
                            break;
                        }
                    }

                    if found {
                        match self.save_todos(&todos, cx) {
                            Ok(_) => Ok(TodoListToolOutput { todos }),
                            Err(e) => Err(anyhow!("Failed to save todos: {}", e)),
                        }
                    } else {
                        Err(anyhow!("Todo with ID {} not found", todo_id))
                    }
                } else {
                    Err(anyhow!("Todo ID is required for update action"))
                }
            }
            TodoAction::List => {
                if todos.is_empty() {
                    event_stream.update_fields(acp::ToolCallUpdateFields {
                        content: Some(vec!["No todos found. Create some todos to get started!".into()]),
                        ..Default::default()
                    });
                } else {
                    let mut markdown_output = String::from("Current todos:\n\n");
                    for todo in &todos {
                        let status_icon = match todo.status {
                            TodoStatus::Pending => "⏳",
                            TodoStatus::InProgress => "🔄",
                            TodoStatus::Completed => "✅",
                        };
                        markdown_output.push_str(&format!(
                            "{} {} (ID: {})\n\n",
                            status_icon, todo.content, todo.id
                        ));
                    }
                    event_stream.update_fields(acp::ToolCallUpdateFields {
                        content: Some(vec![markdown_output.into()]),
                        ..Default::default()
                    });
                }
                Ok(TodoListToolOutput { todos })
            }
            TodoAction::Clear => {
                let count = todos.len();
                todos.clear();
                match self.save_todos(&todos, cx) {
                    Ok(_) => {
                        event_stream.update_fields(acp::ToolCallUpdateFields {
                            content: Some(vec![format!("Cleared {} todo(s).", count).into()]),
                            ..Default::default()
                        });
                        Ok(TodoListToolOutput { todos })
                    }
                    Err(e) => Err(anyhow!("Failed to clear todos: {}", e)),
                }
            }
        };
+
+        Task::ready(output)
+    }
+
+    fn replay(
+        &self,
+        _input: Self::Input,
+        _output: Self::Output,
+        _event_stream: ToolCallEventStream,
+        _cx: &mut App,
+    ) -> Result<()> {
+        Ok(())
+    }
 }

 impl Into<LanguageModelToolResultContent> for TodoListToolOutput {
     fn into(self) -> LanguageModelToolResultContent {
         let mut markdown = String::new();

         if self.todos.is_empty() {
             markdown.push_str("No todos found.");
         } else {
             markdown.push_str("Current todos:\n\n");
             for todo in &self.todos {
                 let status_icon = match todo.status {
                     TodoStatus::Pending => "⏳",
                     TodoStatus::InProgress => "🔄",
                     TodoStatus::Completed => "✅",
                 };
                 markdown.push_str(&format!(
                     "{} {} (ID: {})\n\n",
                     status_icon, todo.content, todo.id
                 ));
             }
         }

         LanguageModelToolResultContent {
             text: Some(markdown),
             ..Default::default()
         }
     }
 }
