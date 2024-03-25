// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::str::FromStr;

use cumulus_primitives_core::ParaId;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use gargantua_runtime::Block;
use log::info;
use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, Result, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
use sp_runtime::traits::AccountIdConversion;
use std::net::SocketAddr;

use crate::{
    chain_spec,
    cli::{Cli, RelayChainCli, Subcommand},
    service::{new_partial, GargantuanExecutor, MessierExecutor, NexusExecutor},
};

fn load_spec(id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
    Ok(match id {
        name if name.starts_with("gargantua-") => {
            let id = name.split('-').last().expect("dev chainspec should have chain id");
            let id = u32::from_str(id).expect("can't parse Id into u32");
            Box::new(chain_spec::gargantua_development_config(id))
        },
        name if name.starts_with("messier-") => {
            let id = name.split('-').last().expect("dev chainspec should have chain id");
            let id = u32::from_str(id).expect("can't parse Id into u32");
            Box::new(chain_spec::messier_development_config(id))
        },
        name if name.starts_with("nexus-") => {
            let id = name.split('-').last().expect("dev chainspec should have chain id");
            let id = u32::from_str(id).expect("can't parse Id into u32");
            Box::new(chain_spec::nexus_development_config(id))
        },

        "gargantua" => Box::new(
            chain_spec::ChainSpec::<gargantua_runtime::RuntimeGenesisConfig>::from_json_bytes(
                include_bytes!("../../chainspec/gargantua.json").to_vec(),
            )?,
        ),
        "messier" => Box::new(
            chain_spec::ChainSpec::<messier_runtime::RuntimeGenesisConfig>::from_json_bytes(
                include_bytes!("../../chainspec/messier.json").to_vec(),
            )?,
        ),
        "nexus" =>
            Box::new(chain_spec::ChainSpec::<nexus_runtime::RuntimeGenesisConfig>::from_json_bytes(
                include_bytes!("../../chainspec/nexus.json").to_vec(),
            )?),
        path => Box::new(
            chain_spec::ChainSpec::<gargantua_runtime::RuntimeGenesisConfig>::from_json_file(
                std::path::PathBuf::from(path),
            )?,
        ),
    })
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Hyperbridge".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Hyperbridge by Polytope Labs\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relay chain node.\n\n\
		{} <parachain-args> -- <relay-chain-args>",
            Self::executable_name()
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/paritytech/cumulus/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2023
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        load_spec(id)
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "Hyperbridge".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Hyperbridge by Polytope Labs\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relay chain node.\n\n\
		{} <parachain-args> -- <relay-chain-args>",
            Self::executable_name()
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/paritytech/cumulus/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2020
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
    }
}

macro_rules! construct_async_run {
	(|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
		let runner = $cli.create_runner($cmd)?;
			match runner.config().chain_spec.id() {
                chain if chain.contains("gargantua") || chain.contains("dev") => {
                    runner.async_run(|$config| {
                        let $components = new_partial::<gargantua_runtime::RuntimeApi, GargantuanExecutor>(&$config)?;
                        Ok::<_, sc_cli::Error>(( { $( $code )* }, $components.task_manager))
		            })
                }
                chain if chain.contains("messier") => {
                    runner.async_run(|$config| {
                        let $components = new_partial::<messier_runtime::RuntimeApi, MessierExecutor>(&$config)?;
                        Ok::<_, sc_cli::Error>(( { $( $code )* }, $components.task_manager))
		            })
                }
                chain => panic!("Unknown chain with id: {}", chain),
            }
	}}
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
    let mut cli = Cli::from_args();

    // all full nodes should store request/responses, otherwise they'd basically be useless without
    // it.
    cli.run.base.offchain_worker_params.indexing_enabled = true;

    match &cli.subcommand {
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        },
        Some(Subcommand::CheckBlock(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                cmd.run(components.client, components.import_queue)
            })
        },
        Some(Subcommand::ExportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                cmd.run(components.client, config.database)
            })
        },
        Some(Subcommand::ExportState(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                cmd.run(components.client, config.chain_spec)
            })
        },
        Some(Subcommand::ImportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                cmd.run(components.client, components.import_queue)
            })
        },
        Some(Subcommand::Revert(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                cmd.run(components.client, components.backend, None)
            })
        },
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.sync_run(|config| {
                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name()].iter().chain(cli.relay_chain_args.iter()),
                );

                let polkadot_config = SubstrateCli::create_configuration(
                    &polkadot_cli,
                    &polkadot_cli,
                    config.tokio_handle.clone(),
                )
                .map_err(|err| format!("Relay chain argument error: {}", err))?;

                cmd.run(config, polkadot_config)
            })
        },
        Some(Subcommand::ExportGenesisState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| match config.chain_spec.id() {
                chain if chain.contains("gargantua") || chain.contains("dev") => {
                    let components =
                        new_partial::<gargantua_runtime::RuntimeApi, GargantuanExecutor>(&config)?;

                    cmd.run(components.client.clone())
                },
                chain if chain.contains("messier") => {
                    let components =
                        new_partial::<messier_runtime::RuntimeApi, MessierExecutor>(&config)?;

                    cmd.run(components.client.clone())
                },
                chain if chain.contains("nexus") => {
                    let components =
                        new_partial::<nexus_runtime::RuntimeApi, NexusExecutor>(&config)?;

                    cmd.run(components.client.clone())
                },
                chain => panic!("Unknown chain with id: {}", chain),
            })
        },
        Some(Subcommand::ExportGenesisWasm(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|_config| {
                let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
                cmd.run(&*spec)
            })
        },
        Some(Subcommand::Benchmark(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            // Switch on the concrete benchmark sub-command-
            match cmd {
                BenchmarkCmd::Pallet(cmd) =>
                    if cfg!(feature = "runtime-benchmarks") {
                        runner.sync_run(|config| cmd.run::<Block, ()>(config))
                    } else {
                        Err("Benchmarking wasn't enabled when building the node. \
					You can enable it with `--features runtime-benchmarks`."
                            .into())
                    },
                BenchmarkCmd::Block(cmd) =>
                    runner.sync_run(|config| match config.chain_spec.id() {
                        chain if chain.contains("gargantua") || chain.contains("dev") => {
                            let components = new_partial::<
                                gargantua_runtime::RuntimeApi,
                                GargantuanExecutor,
                            >(&config)?;
                            cmd.run(components.client)
                        },
                        chain if chain.contains("messier") => {
                            let components = new_partial::<
                                messier_runtime::RuntimeApi,
                                MessierExecutor,
                            >(&config)?;
                            cmd.run(components.client)
                        },
                        chain => panic!("Unknown chain with id: {}", chain),
                    }),
                #[cfg(not(feature = "runtime-benchmarks"))]
                BenchmarkCmd::Storage(_) =>
                    return Err(sc_cli::Error::Input(
                        "Compile with --features=runtime-benchmarks \
						to enable storage benchmarks."
                            .into(),
                    )
                    .into()),
                #[cfg(feature = "runtime-benchmarks")]
                BenchmarkCmd::Storage(cmd) =>
                    runner.sync_run(|config| match config.chain_spec.id() {
                        chain if chain.contains("gargantua") || chain.contains("dev") => {
                            let components = new_partial::<
                                gargantua_runtime::RuntimeApi,
                                GargantuanExecutor,
                            >(&config)?;
                            let db = components.backend.expose_db();
                            let storage = components.backend.expose_storage();
                            cmd.run(config, components.client.clone(), db, storage)
                        },
                        chain if chain.contains("messier") => {
                            let components = new_partial::<
                                messier_runtime::RuntimeApi,
                                MessierExecutor,
                            >(&config)?;
                            let db = components.backend.expose_db();
                            let storage = components.backend.expose_storage();
                            cmd.run(config, components.client.clone(), db, storage)
                        },
                        chain => panic!("Unknown chain with id: {}", chain),
                    }),
                BenchmarkCmd::Machine(cmd) =>
                    runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())),
                // NOTE: this allows the Client to leniently implement
                // new benchmark commands without requiring a companion MR.
                #[allow(unreachable_patterns)]
                _ => Err("Benchmarking sub-command unsupported".into()),
            }
        },
        #[cfg(feature = "try-runtime")]
        Some(Subcommand::TryRuntime(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            use sc_executor::{sp_wasm_interface::ExtendedHostFunctions, NativeExecutionDispatch};
            type HostFunctionsOf<E> = ExtendedHostFunctions<
                sp_io::SubstrateHostFunctions,
                <E as NativeExecutionDispatch>::ExtendHostFunctions,
            >;

            // grab the task manager.
            let registry = &runner.config().prometheus_config.as_ref().map(|cfg| &cfg.registry);
            let task_manager =
                sc_service::TaskManager::new(runner.config().tokio_handle.clone(), *registry)
                    .map_err(|e| format!("Error: {:?}", e))?;

            runner.async_run(|_| {
                Ok((cmd.run::<Block, HostFunctionsOf<GargantuanExecutor>>(), task_manager))
            })
        },
        #[cfg(not(feature = "try-runtime"))]
        Some(Subcommand::TryRuntime) => Err("Try-runtime was not enabled when building the node. \
			You can enable it with `--features try-runtime`."
            .into()),
        None => {
            let runner = cli.create_runner(&cli.run.normalize())?;
            let collator_options = cli.run.collator_options();

            runner.run_node_until_exit(|config| async move {
                let hwbench = if !cli.no_hardware_benchmarks {
                    config.database.path().map(|database_path| {
                        let _ = std::fs::create_dir_all(&database_path);
                        sc_sysinfo::gather_hwbench(Some(database_path))
                    })
                } else {
                    None
                };

                let para_id = chain_spec::Extensions::try_get(&*config.chain_spec)
                    .map(|e| e.para_id)
                    .ok_or_else(|| "Could not find parachain ID in chain-spec.")?;

                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name()].iter().chain(cli.relay_chain_args.iter()),
                );

                let id = ParaId::from(para_id);

                let parachain_account =
                    AccountIdConversion::<polkadot_primitives::AccountId>::into_account_truncating(
                        &id,
                    );

                let tokio_handle = config.tokio_handle.clone();
                let polkadot_config =
                    SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, tokio_handle)
                        .map_err(|err| format!("Relay chain argument error: {}", err))?;

                info!("Parachain Account: {}", parachain_account);
                info!("Is collating: {}", if config.role.is_authority() { "yes" } else { "no" });

                crate::service::start_parachain_node(
                    config,
                    polkadot_config,
                    collator_options,
                    id,
                    hwbench,
                )
                .await
                .map_err(Into::into)
            })
        },
    }
}

impl DefaultConfigurationValues for RelayChainCli {
    fn p2p_listen_port() -> u16 {
        30334
    }

    fn rpc_listen_port() -> u16 {
        9945
    }

    fn prometheus_listen_port() -> u16 {
        9616
    }
}

impl CliConfiguration<Self> for RelayChainCli {
    fn shared_params(&self) -> &SharedParams {
        self.base.base.shared_params()
    }

    fn import_params(&self) -> Option<&ImportParams> {
        self.base.base.import_params()
    }

    fn network_params(&self) -> Option<&NetworkParams> {
        self.base.base.network_params()
    }

    fn keystore_params(&self) -> Option<&KeystoreParams> {
        self.base.base.keystore_params()
    }

    fn base_path(&self) -> Result<Option<BasePath>> {
        Ok(self
            .shared_params()
            .base_path()?
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_addr(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_addr(default_listen_port)
    }

    fn prometheus_config(
        &self,
        default_listen_port: u16,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<PrometheusConfig>> {
        self.base.base.prometheus_config(default_listen_port, chain_spec)
    }

    fn init<F>(
        &self,
        _support_url: &String,
        _impl_version: &String,
        _logger_hook: F,
        _config: &sc_service::Configuration,
    ) -> Result<()>
    where
        F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
    {
        unreachable!("PolkadotCli is never initialized; qed");
    }

    fn chain_id(&self, is_dev: bool) -> Result<String> {
        let chain_id = self.base.base.chain_id(is_dev)?;

        Ok(if chain_id.is_empty() { self.chain_id.clone().unwrap_or_default() } else { chain_id })
    }

    fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
        self.base.base.role(is_dev)
    }

    fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool(is_dev)
    }

    fn trie_cache_maximum_size(&self) -> Result<Option<usize>> {
        self.base.base.trie_cache_maximum_size()
    }

    fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
        self.base.base.rpc_methods()
    }

    fn rpc_max_connections(&self) -> Result<u32> {
        self.base.base.rpc_max_connections()
    }

    fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
        self.base.base.rpc_cors(is_dev)
    }

    fn default_heap_pages(&self) -> Result<Option<u64>> {
        self.base.base.default_heap_pages()
    }

    fn force_authoring(&self) -> Result<bool> {
        self.base.base.force_authoring()
    }

    fn disable_grandpa(&self) -> Result<bool> {
        self.base.base.disable_grandpa()
    }

    fn max_runtime_instances(&self) -> Result<Option<usize>> {
        self.base.base.max_runtime_instances()
    }

    fn announce_block(&self) -> Result<bool> {
        self.base.base.announce_block()
    }

    fn telemetry_endpoints(
        &self,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
        self.base.base.telemetry_endpoints(chain_spec)
    }

    fn node_name(&self) -> Result<String> {
        self.base.base.node_name()
    }
}
