use clap::{crate_version, Arg, Command};
use ko_core::Context;
use ko_protocol::{secp256k1::SecretKey, tokio, KoResult};
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

    // initail CKB rcp client
    let rpc_client = RpcClient::new(&config.ckb_url, &config.ckb_indexer_url);

    // start rpc server
    RpcServerRuntime::run(&config.rpc_endpoint, &rpc_client, &config.as_ref().into()).await?;

    // initail drive context
    let privkey =
        SecretKey::from_slice(config.project_owner_privkey.as_bytes()).expect("private key");
    let context = Context::new(&rpc_client, &privkey, &config.as_ref().into());

    // handle exception operation
    let ctrl_c_handler = tokio::spawn(async {
        #[cfg(windows)]
        let _ = tokio::signal::ctrl_c().await;
        #[cfg(unix)]
        {
            use tokio::signal::unix;
            let mut sigtun_int = unix::signal(unix::SignalKind::interrupt()).unwrap();
            let mut sigtun_term = unix::signal(unix::SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sigtun_int.recv() => {}
                _ = sigtun_term.recv() => {}
            };
        }
    });

    // enter drive loop, will stop when any type of error throwed out
    tokio::select! {
        _ = ctrl_c_handler => {
            println!("<Ctrl-C> is on call, quit knside-out drive loop");
        },
        Err(error) = context.start(&config.project_cell_deps) => {
            println!("[ERROR] {}", error);
        }
    }

    Ok(())
}
