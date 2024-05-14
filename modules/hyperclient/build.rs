use anyhow::anyhow;
use graphql_client_codegen::{
	generate_module_token_stream_from_string, CodegenMode, GraphQLClientCodegenOptions,
};
use proc_macro2::Span;
use std::{env, io::Write, path::Path};
use syn::{token::Pub, VisPublic, Visibility};

pub static SCHEMA: &'static str = include_str!("../../indexers/schema.graphql");

pub static QUERY_STRING: &'static str = include_str!("./graphql/queries.graphql");

fn main() -> anyhow::Result<()> {
	let manifest_dir = env!("CARGO_MANIFEST_DIR");
	let path = format!("{manifest_dir}/graphql/schema.graphql");
	let custom_schema = std::fs::read_to_string(format!("{manifest_dir}/graphql/customs.graphql"))?;
	let mut file = std::fs::File::create(path.clone())?;
	file.write_all(SCHEMA.as_bytes())?;
	writeln!(&mut file, "{custom_schema}")?;

	let mut options = GraphQLClientCodegenOptions::new(CodegenMode::Cli);

	options.set_response_derives("Debug, PartialEq, Eq, Clone".to_string());
	options.set_module_visibility(Visibility::Public(VisPublic {
		pub_token: Pub { span: Span::call_site() },
	}));

	let generated_tokens =
		generate_module_token_stream_from_string(QUERY_STRING, &Path::new(&path), options)
			.map_err(|e| anyhow!("Codegen Failed with {e:?}"))?;

	let generated_code = generated_tokens.to_string();
	let output_path = format!("{manifest_dir}/src/indexing/graphql.rs");

	let mut f = std::fs::File::create(output_path.clone())?;
	writeln!(&mut f, "#![allow(non_camel_case_types)]")?;
	writeln!(&mut f, "#![allow(unused_imports)]")?;
	writeln!(&mut f, "use crate::indexing::BigInt;")?;
	f.write_all(generated_code.as_bytes())?;
	std::fs::remove_file(path)?;

	println!("cargo:rerun-if-changed={manifest_dir}/../../indexers/schema.graphql");
	Ok(())
}
