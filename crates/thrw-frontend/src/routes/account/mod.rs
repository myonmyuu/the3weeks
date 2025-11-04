
use crate::{prelude::*};

pub(self) mod consts {
	pub const CHAR_LIST_ID: i32 = crate::prelude::ACC_IDS + 1;
}

#[component]
pub fn Account() -> impl IntoView {
	view! {
		Accout stuff
	}
}

#[component(transparent)]
pub fn AccountRoutes() -> impl MatchNestedRoutes + Clone {
	ReviewEvent::<{consts::CHAR_LIST_ID}>::provide_new();

	view! {
		<ProtectedParentRoute
			path=path!("/account")
			view=EmptyParent
			condition=check_login_raw
			redirect_path=||"/"
		>
			<Route path=path!("/") view=Account />

		</ProtectedParentRoute>
	}
	.into_inner()
}