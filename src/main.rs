use clap::{crate_version, Arg, Command};
use ko_context::ContextImpl;
use ko_protocol::{secp256k1::SecretKey, tokio, traits::Context, KoResult, ProjectDeps};
use ko_rpc::RpcServerRuntime;
use ko_rpc_client::RpcClient;

#[tokio::main]
async fn main() -> KoResult<()> {
    let matches = Command::new("knside-out")
        .version(crate_version!())
        .arg(
            Arg::new("config_path")
                .short('c')
                .long("config")
                .help("Knside-out config path")
                .required(true)
                .takes_value(true),
        )
        .subcommand(Command::new("run").about("Run knside-out process"))
        .get_matches();

    let config_path = matches.value_of("config_path").unwrap();
    let config = ko_config::load_file(config_path)?;
    let project_deps: &ProjectDeps = &config.as_ref().into();

    // initail CKB rcp client
    let rpc_client = RpcClient::new(&config.ckb_url, &config.ckb_indexer_url);

    // initail drive context
    let privkey =
        SecretKey::from_slice(config.project_owner_privkey.as_bytes()).expect("private key");
    let (mut driver, context_sender) =
        ContextImpl::new(&rpc_client, &privkey, &config.as_ref().into());
    driver.set_drive_interval(config.drive_settings.drive_interval_sec);
    driver.set_max_requests_count(config.drive_settings.max_reqeusts_count);
    driver.set_confirms_count(config.drive_settings.block_confirms_count);

    // start rpc server
    RpcServerRuntime::run(
        &config.rpc_endpoint,
        &rpc_client,
        context_sender,
        project_deps,
    )
    .await?;

    // start drive loop
    driver.run(&project_deps.project_cell_deps).await;
    Ok(())
}
