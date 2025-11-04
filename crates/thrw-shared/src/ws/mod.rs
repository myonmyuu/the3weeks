use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThrwSocketMessage {
	Intoduce(i32),
	String(String),
}