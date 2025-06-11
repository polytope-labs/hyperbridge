use super::*;
use sp_runtime::str_array as s;

const fn percent(x: i32) -> sp_runtime::FixedI64 {
	sp_runtime::FixedI64::from_rational(x as u128, 100)
}
const fn permill(x: i32) -> sp_runtime::FixedI64 {
	sp_runtime::FixedI64::from_rational(x as u128, 1000)
}

use pallet_referenda::Curve;
const TRACKS_DATA: [pallet_referenda::Track<u16, Balance, BlockNumber>; 6] = [
	pallet_referenda::Track {
		id: 0,
		info: pallet_referenda::TrackInfo {
			name: s("root"),
			max_deciding: 1,
			decision_deposit: 100 * UNIT,
			prepare_period: 8 * MINUTES,
			decision_period: 20 * MINUTES,
			confirm_period: 12 * MINUTES,
			min_enactment_period: 5 * MINUTES,
			min_approval: Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100)),
			min_support: Curve::make_linear(28, 28, permill(0), percent(50)),
		},
	},
	pallet_referenda::Track {
		id: 1,
		info: pallet_referenda::TrackInfo {
			name: s("whitelisted_caller"),
			max_deciding: 100,
			decision_deposit: 10 * UNIT,
			prepare_period: 6 * MINUTES,
			decision_period: 20 * MINUTES,
			confirm_period: 4 * MINUTES,
			min_enactment_period: 3 * MINUTES,
			min_approval: Curve::make_reciprocal(
				16,
				28 * 24,
				percent(96),
				percent(50),
				percent(100),
			),
			min_support: Curve::make_reciprocal(1, 1792, percent(3), percent(2), percent(50)),
		},
	},
	pallet_referenda::Track {
		id: 2,
		info: pallet_referenda::TrackInfo {
			name: s("fellowship_admin"),
			max_deciding: 10,
			decision_deposit: 5 * UNIT,
			prepare_period: 8 * MINUTES,
			decision_period: 20 * MINUTES,
			confirm_period: 8 * MINUTES,
			min_enactment_period: 3 * MINUTES,
			min_approval: Curve::make_linear(17, 28, percent(50), percent(100)),
			min_support: Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50)),
		},
	},
	pallet_referenda::Track {
		id: 3,
		info: pallet_referenda::TrackInfo {
			name: s("referendum_canceller"),
			max_deciding: 1_000,
			decision_deposit: 10 * UNIT,
			prepare_period: 8 * MINUTES,
			decision_period: 14 * MINUTES,
			confirm_period: 8 * MINUTES,
			min_enactment_period: 3 * MINUTES,
			min_approval: Curve::make_linear(17, 28, percent(50), percent(100)),
			min_support: Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50)),
		},
	},
	pallet_referenda::Track {
		id: 4,
		info: pallet_referenda::TrackInfo {
			name: s("referendum_killer"),
			max_deciding: 1_000,
			decision_deposit: 50 * UNIT,
			prepare_period: 8 * MINUTES,
			decision_period: 20 * MINUTES,
			confirm_period: 8 * MINUTES,
			min_enactment_period: 3 * MINUTES,
			min_approval: Curve::make_linear(17, 28, percent(50), percent(100)),
			min_support: Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50)),
		},
	},
	pallet_referenda::Track {
		id: 5,
		info: pallet_referenda::TrackInfo {
			name: s("treasury_spend"),
			max_deciding: 200,
			decision_deposit: 1 * 3 * UNIT,
			prepare_period: 1 * MINUTES,
			decision_period: 14 * MINUTES,
			confirm_period: 4 * MINUTES,
			min_enactment_period: 1 * MINUTES,
			min_approval: Curve::make_linear(23, 28, percent(50), percent(100)),
			min_support: Curve::make_reciprocal(16, 28, percent(1), percent(0), percent(50)),
		},
	},
];

pub struct TracksInfo;
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = u16;
	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;
	fn tracks(
	) -> impl Iterator<Item = Cow<'static, pallet_referenda::Track<Self::Id, Balance, BlockNumber>>>
	{
		TRACKS_DATA.iter().map(Cow::Borrowed)
	}
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		if let Ok(system_origin) = frame_system::RawOrigin::try_from(id.clone()) {
			match system_origin {
				frame_system::RawOrigin::Root => Ok(0),
				_ => Err(()),
			}
		} else if let Ok(custom_origin) = origins::Origin::try_from(id.clone()) {
			match custom_origin {
				origins::Origin::WhitelistedCaller => Ok(1),
				origins::Origin::FellowshipAdmin => Ok(2),
				origins::Origin::ReferendumCanceller => Ok(3),
				origins::Origin::ReferendumKiller => Ok(4),
				origins::Origin::TreasurySpend => Ok(5),
				_ => Err(()),
			}
		} else {
			Err(())
		}
	}
}
