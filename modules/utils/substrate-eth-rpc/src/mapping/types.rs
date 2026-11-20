use alloy_dyn_abi::DynSolType;
use alloy_primitives::{keccak256, B256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MappingError {
	#[error("unknown pallet key `{0}`")]
	UnknownPallet(String),
	#[error("missing field `{0}`")]
	MissingField(String),
	#[error("expected {expected}, got {got}")]
	TypeMismatch { expected: String, got: String },
	#[error("{event}.{field}: {source}")]
	Field {
		event: String,
		field: String,
		#[source]
		source: Box<MappingError>,
	},
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiType {
	Bytes32,
	Bytes4,
	Bytes,
	String,
	Bool,
	Uint(usize),
	Address,
	Tuple(Vec<FieldSpec>),
	StateMachine,
	Option(Box<AbiType>),
	Enum(&'static [(&'static str, u8)]),
}

impl AbiType {
	pub fn sol_type(&self) -> String {
		match self {
			AbiType::Bytes32 => "bytes32".into(),
			AbiType::Bytes4 => "bytes4".into(),
			AbiType::Bytes => "bytes".into(),
			AbiType::String | AbiType::StateMachine => "string".into(),
			AbiType::Bool => "bool".into(),
			AbiType::Uint(bits) => format!("uint{bits}"),
			AbiType::Address => "address".into(),
			AbiType::Enum(_) => "uint8".into(),
			AbiType::Tuple(fields) => {
				let inner = fields.iter().map(|f| f.ty.sol_type()).collect::<Vec<_>>();
				format!("({})", inner.join(","))
			},
			AbiType::Option(inner) => format!("(bool,{})", inner.sol_type()),
		}
	}

	pub fn dyn_type(&self) -> DynSolType {
		match self {
			AbiType::Bytes32 => DynSolType::FixedBytes(32),
			AbiType::Bytes4 => DynSolType::FixedBytes(4),
			AbiType::Bytes => DynSolType::Bytes,
			AbiType::String | AbiType::StateMachine => DynSolType::String,
			AbiType::Bool => DynSolType::Bool,
			AbiType::Uint(bits) => DynSolType::Uint(*bits),
			AbiType::Address => DynSolType::Address,
			AbiType::Enum(_) => DynSolType::Uint(8),
			AbiType::Tuple(fields) =>
				DynSolType::Tuple(fields.iter().map(|f| f.ty.dyn_type()).collect()),
			AbiType::Option(inner) => DynSolType::Tuple(vec![DynSolType::Bool, inner.dyn_type()]),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpec {
	pub name: &'static str,
	pub source: &'static str,
	pub ty: AbiType,
	pub indexed: bool,
}

impl FieldSpec {
	pub fn new(name: &'static str, source: &'static str, ty: AbiType) -> Self {
		Self { name, source, ty, indexed: false }
	}

	pub fn indexed(name: &'static str, source: &'static str, ty: AbiType) -> Self {
		Self { name, source, ty, indexed: true }
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSpec {
	pub name: &'static str,
	pub fields: Vec<FieldSpec>,
}

impl EventSpec {
	pub fn new(name: &'static str, fields: Vec<FieldSpec>) -> Self {
		Self { name, fields }
	}

	pub fn signature(&self) -> String {
		let params = self.fields.iter().map(|f| f.ty.sol_type()).collect::<Vec<_>>();
		format!("{}({})", self.name, params.join(","))
	}

	pub fn topic0(&self) -> B256 {
		keccak256(self.signature().as_bytes())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PalletSpec {
	pub key: &'static str,
	pub metadata_name: &'static str,
	pub events: Vec<EventSpec>,
}

pub fn state_machine_id() -> AbiType {
	AbiType::Tuple(vec![
		FieldSpec::new("stateId", "state_id", AbiType::StateMachine),
		FieldSpec::new("consensusStateId", "consensus_state_id", AbiType::Bytes4),
	])
}
