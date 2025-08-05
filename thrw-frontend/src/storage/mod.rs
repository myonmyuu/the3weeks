use std::{rc::Rc, sync::Arc, time::Duration};

use gloo_utils::format::JsValueSerdeExt;
use idb::{request::OpenDatabaseRequest, Database, DatabaseEvent, Factory, IndexParams, KeyRange, ObjectStoreParams};
use js_sys::{Array, JsString};
use leptos::{prelude::{provide_context, use_context}, reactive::spawn_local};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_wasm_bindgen::Serializer;
use tokio::sync::{mpsc::{error::SendError, UnboundedReceiver, UnboundedSender}, oneshot::{self, error::RecvError, Sender}};
use wasm_bindgen::JsValue;

pub mod names {
	pub const DB_NAME: &str = "thrw_data";
	pub const LOG_STORE: &str = "logs";
}

pub mod logs;

#[derive(Debug)]
pub enum StorageError {
	DbUnavailable,
	SendErr(SendError<ClientDBMessage>),
	Json(serde_json::Error),
	Recv(RecvError),
	Path,
	Key,
	Transaction,
	Store,
	Stringify,
	JsonParse,
	Query,
	ReturnValue,
	GetField,
	GetValues,
}

#[derive(Debug, Clone)]
pub enum IDBKey {
	String(String),
	Int(u32),
}
impl IDBKey {
	fn to_key(&self) -> Result<JsValue, serde_wasm_bindgen::Error> {
		let serializer = Serializer::json_compatible();
		match self {
			IDBKey::String(s) => s.serialize(&serializer),
			IDBKey::Int(i) => i.serialize(&serializer),
		}
	}
}
impl From<u32> for IDBKey {
	fn from(value: u32) -> Self {
		Self::Int(value)
	}
}
impl From<String> for IDBKey {
	fn from(value: String) -> Self {
		Self::String(value)
	}
}
impl From<JsValue> for IDBKey {
	fn from(value: JsValue) -> Self {
		if value.is_string() {
			Self::String(value.as_string().unwrap())
		} else if let Some(num) = value.as_f64() {
			Self::Int(num as u32)
		} else {
			panic!("JsValue type unsupported: {value:?}");
		}
	}
}

pub enum IDBQueryArgs {
	LowerBound(
		String,					//key range jsvalue
		Option<bool>,			//open?
	),
	UpperBound(
		String,					//key range jsvalue
		Option<bool>,			//open?
	),
	Bound(
		(String, String),		//lower/upper range
		Option<(bool, bool)>	//lower/upper open
	)
}
#[allow(unused)]
impl IDBQueryArgs {
	fn js_slice_to_string(list: &[JsValue]) -> Result<String, StorageError> {
		let range_array: Array = list.iter().collect();
		js_sys::JSON::stringify(&range_array.into())
			.map(String::from)
			.map_err(|_| StorageError::Stringify)
	}
	fn ul_bound_from_list(upper: bool, list: &[JsValue], open: Option<bool>) -> Result<Self, StorageError> {
		let key_range_str = Self::js_slice_to_string(list)?;

		if upper {
			Ok(Self::UpperBound(key_range_str, open))
		} else {
			Ok(Self::LowerBound(key_range_str, open))
		}
	}
	pub fn upper_bound(list: &[JsValue], open: Option<bool>) -> Result<Self, StorageError> {
		Self::ul_bound_from_list(true, list, open)
	}
	pub fn lower_bound(list: &[JsValue], open: Option<bool>) -> Result<Self, StorageError> {
		Self::ul_bound_from_list(false, list, open)
	}
	pub fn bound(lower: &[JsValue], upper: &[JsValue], open: Option<(bool, bool)>) -> Result<Self, StorageError> {
		Ok(Self::Bound((
			Self::js_slice_to_string(lower)?,
			Self::js_slice_to_string(upper)?
		), open))
	}
}
impl From<IDBQueryArgs> for idb::Query {
	fn from(value: IDBQueryArgs) -> Self {
		match value {
			IDBQueryArgs::LowerBound(list_str, open) => idb::Query::KeyRange(
				KeyRange::lower_bound(
					&js_sys::JSON::parse(&list_str).unwrap(),
					open
				).unwrap()
			),
			IDBQueryArgs::UpperBound(list_str, open) => idb::Query::KeyRange(
				KeyRange::upper_bound(
					&js_sys::JSON::parse(&list_str).unwrap(),
					open
				).unwrap()
			),
			IDBQueryArgs::Bound(bounds, open) => idb::Query::KeyRange(
				KeyRange::bound(
					&js_sys::JSON::parse(&bounds.0).unwrap(),
					&js_sys::JSON::parse(&bounds.1).unwrap(),
					open.map(|v| v.0),
					open.map(|v| v.1)
				).unwrap()
			),
		}
	}
}

pub enum ClientDBMessage {
	Store(
		String,					//store
		Option<IDBKey>,			//key
		String,					//json
		oneshot::Sender<Result<IDBKey, StorageError>>
	),
	Retrieve(
		String,					//store
		IDBKey,					//key
		oneshot::Sender<Result<String, StorageError>>
	),
	RetrieveMany(
		String,					//store
		Option<IDBQueryArgs>,	//query
		Option<u32>,			//limit
		Option<String>,			//index
		oneshot::Sender<Result<Vec<String>, StorageError>>
	),
}

#[derive(Clone)]
struct StorageData {
	db_send: UnboundedSender<ClientDBMessage>,
}

fn get_storage_and_id_from_path(path: &str) -> (String, Option<String>) {
	let id;
	let mut store = path.to_string();
	match &path.split_terminator(".").collect::<Vec<&str>>()[0..] {
		&[a, b] => {
			store = a.to_string();
			id = Some(b.to_string());
		},
		_ => {
			id = None;
		}
	};

	(store, id)
}

fn json_str_to_object(json: &str) -> Result<JsValue, serde_json::Error> {
	let parsed: serde_json::Value = serde_json::from_str(json)?;

	JsValue::from_serde(&parsed)
}

fn js_value_to_string(value: &JsValue) -> Option<String> {
	if value.is_string() {
        value.as_string()
    } else if let Some(num) = value.as_f64() {
		if (num.fract() - 0.0).abs() < f64::EPSILON {
        	Some((num as i64).to_string())
		} else {
			Some(num.to_string())
		}
    } else if let Some(b) = value.as_bool() {
        Some(b.to_string())
    } else if value.is_null() || value.is_undefined() {
        Some("null".to_string())
    } else {
        // Fallback
        js_sys::JSON::stringify(value).map(String::from).ok()
    }
}

fn get_field_as_string(obj: &JsValue, field: &str) -> Option<String> {
    let value = js_sys::Reflect::get(obj, &JsValue::from_str(field)).ok()?;

    js_value_to_string(&value)
}

async fn handle_db_requests(
	mut receive: UnboundedReceiver<ClientDBMessage>,
	db_request: OpenDatabaseRequest,
) {
	let db_connection = 
		db_request
		.await
		.unwrap();

	while let Some(message) = receive.recv().await {
		match message {
			ClientDBMessage::Store(store, key, value, res_channel) => {
				let Ok(transaction) = db_connection.transaction(std::slice::from_ref(&store), idb::TransactionMode::ReadWrite) else {
					// TODO: log error
					let _ = res_channel.send(Err(StorageError::Transaction));
					continue;
				};
				let store = transaction.object_store(&store).unwrap();
				
				let Ok(value) = json_str_to_object(&value) else {
					let _ = res_channel.send(Err(StorageError::JsonParse));
					continue;
				};
				let id = key.and_then(|k| k.to_key().ok());
				let res = match store.add(&value, id.as_ref()) {
					Ok(req) => req.await,
					Err(err) => {
						log::debug!("error creating store request: {err}");
						let _ = res_channel.send(Err(StorageError::Store));
						continue;
					},
				};

				let added_key = match res {
					Ok(value) => value,
					Err(err) => {
						log::debug!("error adding value to store: {err}");
						let _ = res_channel.send(Err(StorageError::Store));
						continue;
					},
				};

				if let Err(err) = transaction.commit() {
					log::debug!("error committing add transaction: {err}");
					let _ = res_channel.send(Err(StorageError::Transaction));
					continue;
				}

				let _ = res_channel.send(Ok(added_key.into()));
			},
			ClientDBMessage::Retrieve(store, key, res_channel) => {
				let Ok(transaction) = db_connection.transaction(std::slice::from_ref(&store), idb::TransactionMode::ReadOnly) else {
					// TODO: log error
					let _ = res_channel.send(Err(StorageError::Transaction));
					continue;
				};

				let store_name = store.clone();
				let Ok(store) = transaction.object_store(&store) else {
					log::debug!("error getting idb store '{store}'");
					let _ = res_channel.send(Err(StorageError::Store));
					continue;
				};

				let Ok(id) = key.clone().to_key() else {
					// log::debug!("error getting idb store '{store}'");
					let _ = res_channel.send(Err(StorageError::Key));
					continue;
				};

				let Ok(value) = store.get(id) else {
					log::debug!("error getting value with key '{key:?}' from '{store_name}'");
					let _ = res_channel.send(Err(StorageError::Store));
					continue;
				};

				let Ok(Some(value)) = value.await else {
					log::debug!("error getting value for entry with key '{key:?}'");
					let _ = res_channel.send(Err(StorageError::Store));
					continue;
				};

				let Ok(value) = js_sys::JSON::stringify(&value)
					.map(String::from) else
				{
					let _ = res_channel.send(Err(StorageError::Stringify));
					continue;
				};

				let _ = res_channel.send(Ok(value));
			},
			ClientDBMessage::RetrieveMany(store, query, limit, index, res_channel) => {
				let Ok(transaction) = db_connection.transaction(std::slice::from_ref(&store), idb::TransactionMode::ReadOnly) else {
					// TODO: log error
					let _ = res_channel.send(Err(StorageError::Transaction));
					continue;
				};

				let Ok(store) = transaction.object_store(&store) else {
					let _ = res_channel.send(Err(StorageError::Store));
					// TODO: log error
					continue;
				};

				let query = query
					.map(|query| query.into())
				;

				let res = match index {
					Some(index) => {
						let Ok(index) = store.index(&index) else {
							continue;
						};
						index.get_all(query, limit)
					},
					None => {
						store.get_all(query, limit)
					},
				};

				let res = match res {
					Ok(vals) => vals.await,
					Err(err) => {
						log::debug!("error getting values: {err}");
						let _ = res_channel.send(Err(StorageError::GetValues));
						continue;
					},
				};

				let Ok(res) = res else {
					let _ = res_channel.send(Err(StorageError::Store));
					continue;
				};
				
				let res: Result<Vec<JsString>, JsValue> = res.iter()
						.map(js_sys::JSON::stringify)
						.collect();
				let Ok(values) = res else {
					let _ = res_channel.send(Err(StorageError::Stringify));
					continue;
				};
				let _ = res_channel.send(Ok(values.iter().map(String::from).collect()));
			},
		}
	}
}

pub fn init_storage() {
	let (send, receive) = tokio::sync::mpsc::unbounded_channel::<ClientDBMessage>();

	spawn_local(async move {
		async fn delete_db() -> Result<(), idb::Error> {
			log::debug!("deleting IndexedDB...");
			Factory::new()?
				.delete(names::DB_NAME)?
				.await?
			;
			log::debug!("DB deleted");
			Ok(())
		}

		#[cfg(debug_assertions)]
		let _ = delete_db().await;

		let factory = Factory::new().expect("unable to init idb factory");
		let mut db_request = factory
			.open(names::DB_NAME, Some(1))
			.unwrap();
		
		db_request.on_upgrade_needed(|event| {
			match event.new_version().unwrap() {
				Some(1) => {
					let db = event.database().unwrap();
					let mut log_params = ObjectStoreParams::new();
					log_params.auto_increment(true);
					log_params.key_path(Some(idb::KeyPath::new_single("id")));
					let log_store = db
						.create_object_store(
							names::LOG_STORE,
							log_params
						)
						.unwrap()
					;
					
					let mut log_room_index = IndexParams::new();
					// log_room_index.multi_entry(true);

					log_store
						.create_index(
							"room",
							idb::KeyPath::Single("room".to_string()),
							Some(log_room_index)
						)
						.unwrap()
					;

					// let mut log_room_date_index = IndexParams::new();
					// log_room_index.
					log_store
						.create_index(
							"roomtime",
							idb::KeyPath::Array(vec![
								"room".to_string(),
								"time".to_string()
							]),
							None
						)
						.unwrap()
					;
				},
				_ => {}
			}
		});

		spawn_local(async {
			handle_db_requests(receive, db_request).await;
		});

		provide_context(StorageData { db_send: send });
		log::debug!("provided storage data");

	});


}

pub async fn store_value<T: Serialize>(store: &str, key: Option<IDBKey>, value: T) -> Result<IDBKey, StorageError> {
	let json_res = serde_json::ser::to_string(&value);
	let Ok(serialized) = json_res else {
		return Err(StorageError::Json(json_res.unwrap_err()));
	};

	let Some(data) = use_context::<StorageData>() else {
		return Err(StorageError::DbUnavailable);
	};

	let (ch_send, ch_receive) = oneshot::channel();

	if let Err(err) = data.db_send.send(ClientDBMessage::Store(store.to_string(), key, serialized, ch_send)) {
		return Err(StorageError::SendErr(err));
	}

	ch_receive.await.map_err(StorageError::Recv)?
}

async fn get_json_from_path(store: &str, key: IDBKey) -> Result<String, StorageError> {
	let send = use_context::<StorageData>()
		.unwrap()
		.db_send
	;

	let (ch_send, ch_receive) = oneshot::channel();
	send.send(ClientDBMessage::Retrieve(store.to_string(), key, ch_send))
		.map_err(StorageError::SendErr)?
	;

	ch_receive.await
		.map_err(StorageError::Recv)?
}

pub async fn get_value<T>(path: &str, key: IDBKey) -> Result<T, StorageError>
where
	for<'de> T: Deserialize<'de>
{
	let value = get_json_from_path(path, key).await?;

	serde_json::de::from_str::<T>(&value)
		.map_err(StorageError::Json)
}

pub async fn get_values<T>(
	store: &str,
	query: Option<IDBQueryArgs>,
	limit: Option<u32>,
	index: Option<&str>
) -> Result<Vec<T>, StorageError>
where
	for<'de> T: Deserialize<'de>,
{
	let send = use_context::<StorageData>()
		.unwrap()
		.db_send
	;

	let (
		ch_send,
		ch_receive
	) = oneshot::channel();

	send.send(ClientDBMessage::RetrieveMany(
		store.to_string(),
		query,
		limit,
		index.map(String::from),
		ch_send
	))
		.map_err(StorageError::SendErr)?
	;

	let jsons = ch_receive.await
		.map_err(StorageError::Recv)??
	;

	let results: Result<Vec<T>, serde_json::Error> = jsons
		.iter()
		.map(|json| serde_json::de::from_str::<T>(json))
		.collect()
	;
	results.map_err(StorageError::Json)
}