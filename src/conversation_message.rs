use serde::Deserialize;

#[derive(Clone)]
pub enum Role {
    User,
    System,
    Assistant,
}

#[derive(Clone)]
pub struct ConversationMessage {
    pub role: Role,
    pub content: String,
    pub function_call: Option<FunctionCall>,
}

impl ConversationMessage {
    pub fn new_content(role: Role, content: String) -> Self {
        Self {
            role,
            content: content.to_string(),
            function_call: None,
        }
    }
    pub fn new_function_call(function_call: FunctionCall, content: String) -> Self {
        Self {
            role: Role::Assistant,
            content: content.to_string(),
            function_call: Some(function_call),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}
