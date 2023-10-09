use ethers::{
    abi::{Detokenize, Tokenize},
    solc::{remappings::Remapping, Project, ProjectCompileOutput, ProjectPathsConfig},
    types::{Log, U256},
};
use forge::{
    executor::{
        inspector::CheatsConfig,
        opts::{Env, EvmOpts},
    },
    result::TestSetup,
    ContractRunner, MultiContractRunner, MultiContractRunnerBuilder,
};
use foundry_config::{fs_permissions::PathPermission, Config, FsPermissions};
use foundry_evm::{
    decode::decode_console_logs,
    executor::{Backend, EvmError, ExecutorBuilder},
    Address,
};
use once_cell::sync::Lazy;
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

static PROJECT: Lazy<Project> = Lazy::new(|| {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root = PathBuf::from(root.parent().unwrap());
    let mut paths = ProjectPathsConfig::builder().root(root.clone()).build().unwrap();
    {
        // manually insert openzeppelin to remmapings. forge isn't autodetecting.
        let mut path = root.clone();
        path.push("lib/openzeppelin-contracts/contracts");
        paths.remappings.push(Remapping {
            context: None,
            name: "openzeppelin/".to_string(),
            path: path.to_str().unwrap().to_string(),
        });
    }
    {
        // manually insert openzeppelin to remmapings. forge isn't autodetecting.
        let mut path = root.clone();
        path.push("lib/ismp-solidity/src");
        paths.remappings.push(Remapping {
            context: None,
            name: "ismp/".to_string(),
            path: path.to_str().unwrap().to_string(),
        });
    }
    Project::builder().paths(paths).ephemeral().no_artifacts().build().unwrap()
});

static EVM_OPTS: Lazy<EvmOpts> = Lazy::new(|| EvmOpts {
    env: Env {
        gas_limit: 18446744073709551615,
        chain_id: Some(foundry_common::DEV_CHAIN_ID),
        tx_origin: Config::DEFAULT_SENDER,
        block_number: 1,
        block_timestamp: 1,
        code_size_limit: Some(usize::MAX),
        ..Default::default()
    },
    sender: Config::DEFAULT_SENDER,
    initial_balance: U256::MAX,
    ffi: true,
    memory_limit: 2u64.pow(24),
    ..Default::default()
});

static LOL: Lazy<()> = Lazy::new(|| {
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    let subscriber = FmtSubscriber::builder().with_max_level(Level::TRACE).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
});

static COMPILED: Lazy<ProjectCompileOutput> = Lazy::new(|| {
    let out = (*PROJECT).compile().unwrap();
    if out.has_compiler_errors() {
        eprintln!("{out}");
        panic!("Compiled with errors");
    }
    out
});

/// Builds a base runner
fn base_runner() -> MultiContractRunnerBuilder {
    MultiContractRunnerBuilder::default().sender(EVM_OPTS.sender)
}

fn manifest_root() -> PathBuf {
    let mut root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // need to check here where we're executing the test from, if in `forge` we need to also allow
    // `testdata`
    if root.ends_with("forge") {
        root = root.parent().unwrap();
    }
    root.to_path_buf()
}

/// Builds a non-tracing runner
fn runner_with_config(mut config: Config) -> MultiContractRunner {
    use foundry_evm::executor::SpecId;

    config.allow_paths.push(manifest_root());

    base_runner()
        .with_cheats_config(CheatsConfig::new(&config, &EVM_OPTS))
        .evm_spec(SpecId::MERGE)
        .sender(config.sender)
        .build(&PROJECT.paths.root, (*COMPILED).clone(), EVM_OPTS.local_evm_env(), EVM_OPTS.clone())
        .unwrap()
}

/// Builds a non-tracing runner
pub fn runner() -> MultiContractRunner {
    let mut config = Config::with_root(PROJECT.root());
    config.fs_permissions = FsPermissions::new(vec![PathPermission::read_write(manifest_root())]);
    runner_with_config(config)
}

pub async fn execute<T, R>(
    runner: &mut MultiContractRunner,
    contract_name: &'static str,
    fn_name: &'static str,
    args: T,
) -> Result<R, EvmError>
where
    T: Tokenize,
    R: Detokenize + Debug,
{
    let db = Backend::spawn(runner.fork.take()).await;

    let (id, (abi, deploy_code, libs)) = runner
        .contracts
        .iter()
        .find(|(id, (abi, _, _))| id.name == contract_name && abi.functions.contains_key(fn_name))
        .unwrap();
    let identifier = id.identifier();

    let function = abi.functions.get(fn_name).unwrap().first().unwrap().clone();

    let executor = ExecutorBuilder::default()
        .with_cheatcodes(runner.cheats_config.clone())
        .with_config(runner.env.clone())
        .with_spec(runner.evm_spec)
        .with_gas_limit(runner.evm_opts.gas_limit())
        .set_tracing(true)
        .set_coverage(runner.coverage)
        .build(db.clone());

    let mut single_runner = ContractRunner::new(
        &identifier,
        executor,
        abi,
        deploy_code.clone(),
        runner.evm_opts.initial_balance,
        runner.sender,
        runner.errors.as_ref(),
        libs,
    );

    let setup = single_runner.setup(true);
    let TestSetup { address, .. } = setup;

    let result = single_runner.executor.execute_test::<R, _, _>(
        single_runner.sender,
        address,
        function,
        args,
        0.into(),
        single_runner.errors,
    );

    match &result {
        Ok(call) => print_logs(fn_name, call.gas_used, &call.logs),
        Err(EvmError::Execution(execution)) =>
            print_logs(fn_name, execution.gas_used, &execution.logs),
        _ => {},
    };

    Ok(result?.result)
}

pub async fn single_runner<'a>(
    runner: &'a mut MultiContractRunner,
    contract_name: &'static str,
) -> (ContractRunner<'a>, Address) {
    let db = Backend::spawn(runner.fork.take()).await;

    let (id, (abi, deploy_code, libs)) = runner
        .contracts
        .iter()
        .find(|(id, (_, _, _))| id.name == contract_name)
        .unwrap();

    // dbg!(deploy_code.len());
    // dbg!(2 * 0x6000); // max init codesize

    let executor = ExecutorBuilder::default()
        .with_cheatcodes(runner.cheats_config.clone())
        .with_config(runner.env.clone())
        .with_spec(runner.evm_spec)
        .with_gas_limit(runner.evm_opts.gas_limit())
        .set_tracing(true)
        .set_coverage(runner.coverage)
        .build(db.clone());

    let mut single_runner = ContractRunner::new(
        &id.name,
        executor,
        abi,
        deploy_code.clone(),
        runner.evm_opts.initial_balance,
        runner.sender,
        runner.errors.as_ref(),
        libs,
    );

    let setup = single_runner.setup(true);
    let TestSetup { address, .. } = setup;

    (single_runner, address)
}

/// Execute using the single [`ContractRunner`]
pub fn execute_single<R, T>(
    contract: &mut ContractRunner,
    address: Address,
    func: &str,
    args: T,
) -> Result<R, EvmError>
where
    T: Tokenize,
    R: Detokenize + Debug,
{
    let function = contract.contract.functions.get(func).unwrap().first().unwrap().clone();

    let result = contract.executor.execute_test::<R, _, _>(
        contract.sender,
        address,
        function,
        args,
        0.into(),
        contract.errors,
    );

    match &result {
        Ok(call) => print_logs(func, call.gas_used, &call.logs),
        Err(EvmError::Execution(execution)) =>
            print_logs(func, execution.gas_used, &execution.logs),
        _ => {},
    };

    Ok(result?.result)
}

fn print_logs(func: &str, gas_used: u64, logs: &Vec<Log>) {
    println!("Gas used {func}: {:#?}", gas_used);
    println!("=========== Start Logs {func} ===========");
    for log in decode_console_logs(logs) {
        println!("{}", log);
    }
    println!("=========== End Logs {func} ===========");
}
