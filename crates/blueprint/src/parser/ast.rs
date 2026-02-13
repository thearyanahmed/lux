/// untyped AST — the parser produces this tree from .bp text.
/// the transpiler converts it into typed IR.

#[derive(Debug, Clone, PartialEq)]
pub struct Ast {
    pub blueprint: BlueprintBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintBlock {
    pub name: String,
    pub items: Vec<AstItem>,
}

/// a top-level item inside a blueprint, phase, step, or nested block
#[derive(Debug, Clone, PartialEq)]
pub enum AstItem {
    /// key: value (including multi-line with |)
    Property(Property),
    /// named block: `block_type "name" { ... }` or `block_type { ... }`
    Block(Block),
    /// raw line that doesn't fit key:value pattern (e.g. probe lines, list items)
    Line(RawLine),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub key: String,
    pub value: PropertyValue,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// simple string value (unquoted or quoted)
    String(String),
    /// integer
    Int(i64),
    /// float
    Float(f64),
    /// boolean
    Bool(bool),
    /// multi-line string (from | syntax)
    MultiLine(String),
}

impl PropertyValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PropertyValue::String(s) | PropertyValue::MultiLine(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            PropertyValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PropertyValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub block_type: String,
    pub name: Option<String>,
    pub items: Vec<AstItem>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawLine {
    pub content: String,
    pub line: usize,
}
