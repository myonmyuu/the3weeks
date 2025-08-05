use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientLog {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<i32>,
	pub room: String,
	pub time: f64,
}