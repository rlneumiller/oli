use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LspServerType {
    Python,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbolParams {
    pub file_path: String,
    pub server_type: LspServerType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokensParams {
    pub file_path: String,
    pub server_type: LspServerType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLensParams {
    pub file_path: String,
    pub server_type: LspServerType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionParams {
    pub file_path: String,
    pub position: Position,
    pub server_type: LspServerType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: SymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub children: Option<Vec<DocumentSymbol>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

impl SymbolKind {
    pub fn to_string(&self) -> Option<String> {
        match self {
            SymbolKind::File => Some("File".to_string()),
            SymbolKind::Module => Some("Module".to_string()),
            SymbolKind::Namespace => Some("Namespace".to_string()),
            SymbolKind::Package => Some("Package".to_string()),
            SymbolKind::Class => Some("Class".to_string()),
            SymbolKind::Method => Some("Method".to_string()),
            SymbolKind::Property => Some("Property".to_string()),
            SymbolKind::Field => Some("Field".to_string()),
            SymbolKind::Constructor => Some("Constructor".to_string()),
            SymbolKind::Enum => Some("Enum".to_string()),
            SymbolKind::Interface => Some("Interface".to_string()),
            SymbolKind::Function => Some("Function".to_string()),
            SymbolKind::Variable => Some("Variable".to_string()),
            SymbolKind::Constant => Some("Constant".to_string()),
            SymbolKind::String => Some("String".to_string()),
            SymbolKind::Number => Some("Number".to_string()),
            SymbolKind::Boolean => Some("Boolean".to_string()),
            SymbolKind::Array => Some("Array".to_string()),
            SymbolKind::Object => Some("Object".to_string()),
            SymbolKind::Key => Some("Key".to_string()),
            SymbolKind::Null => Some("Null".to_string()),
            SymbolKind::EnumMember => Some("EnumMember".to_string()),
            SymbolKind::Struct => Some("Struct".to_string()),
            SymbolKind::Event => Some("Event".to_string()),
            SymbolKind::Operator => Some("Operator".to_string()),
            SymbolKind::TypeParameter => Some("TypeParameter".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokens {
    pub data: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLens {
    pub range: Range,
    pub command: Option<Command>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub title: String,
    pub command: String,
    pub arguments: Option<Vec<serde_json::Value>>,
}
