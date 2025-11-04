use crate::{prelude::*, routes::{admin::{consts::KEY_LIST_ID, keys::KeyManager}, EmptyParent, EmptyView}, util::check_login_raw};

mod keys;

pub(self) mod consts {
	pub const KEY_LIST_ID: i32 = crate::prelude::ADMIN_IDS + 1;
}

#[component(transparent)]
pub fn AdminRoutes() -> impl MatchNestedRoutes + Clone {
	ReviewEvent::<{KEY_LIST_ID}>::provide_new();

	view! {
		<ProtectedParentRoute
			path=path!("/admin")
			view=EmptyParent
			condition=check_admin
			redirect_path=||"/"
		>
			<Route path=path!("/") view=EmptyView />
			<Route path=path!("/keys") view=KeyManager />
		</ProtectedParentRoute>
	}
	.into_inner()
}