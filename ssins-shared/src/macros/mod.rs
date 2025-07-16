#[macro_export]
macro_rules! make_error_type {
	($v:vis $ername:ident { $($specname:ident($spectype:ty)),+$(,)? }) => {
		$v enum $ername {
			$(
				$specname($spectype),
			)+
		}
		$(
		impl From<$spectype> for $ername {
			fn from(value: $spectype) -> Self {
				Self::$specname(value)
			}
		}
		)+
		impl std::fmt::Debug for $ername {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				match self {
					$(
						Self::$specname(spec) => f.debug_tuple(stringify!($specname)).field(spec).finish(),
					)+
				}
			}
		}
	};
}