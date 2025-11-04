use thrw_shared::app::state::{client::LoginContext, shared::{AccountLevel, LoginState}};

use crate::prelude::*;

pub mod consts {
	pub const ACC_IDS: i32 = 20000;
	pub const ADMIN_IDS: i32 = 30000;
	pub const VFS_IDS: i32 = 40000;
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewEvent<const ID: i32> {
	signal: RwSignal<i32>,
}
impl<const ID: i32> ReviewEvent<ID> {
	pub fn new() -> Self {
		Self {
			signal: RwSignal::new(0),
		}
	}

	pub fn invalidate(&self) {
		self.signal.set(self.signal.get_untracked() + 1);
	}

	pub fn subscribe(self) -> impl Fn() -> i32 {
		let s = self.signal;
		move || { s.get() }
	}

	pub fn provide_new() {
		provide_context(Self::new());
	}

	pub fn use_provided() -> Self {
		use_context::<Self>().unwrap_or_else(|| panic!("review event {ID} not registered"))
	}
}
impl<const ID: i32> Default for ReviewEvent<ID> {
	fn default() -> Self {
		Self::new()
	}
}


pub fn check_login(login_ctx: Option<LoginContext>) -> Option<bool> {
	let login_ctx = login_ctx.unwrap_or_else(|| use_context::<LoginContext>().expect("login context missing"));
	let login_state = login_ctx.login_state.get();
	match login_state {
		LoginState::Unverified => None,
		LoginState::LoggedOut => Some(false),
		LoginState::LoggedIn(_, _, _) => Some(true),
	}
}

pub fn check_login_raw() -> Option<bool> {
	check_login(None)
}

pub fn check_admin() -> Option<bool> {
	let ctx = use_context::<LoginContext>().expect("login context missing");
	match ctx.login_state.get() {
		LoginState::Unverified => None,
		LoginState::LoggedOut => Some(false),
		LoginState::LoggedIn(_, _, account_level) => Some(matches!(account_level, AccountLevel::Admin)),
	}
}