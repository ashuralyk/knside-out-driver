use clap::{crate_version, Arg, Command};
use ko_core::Context;
use ko_core_assembler::AssemblerImpl;
use ko_core_driver::DriverImpl;
use ko_core_executor::ExecutorImpl;
use ko_protocol::secp256k1::SecretKey;
use ko_protocol::tokio;
use ko_rpc_client::RpcClient;

#[tokio::main]
async fn main() {
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
    let config = ko_config::load_file(config_path).expect("load config");

    // make instances of assembler, executor and driver
    let rpc_client = RpcClient::new(&config.ckb_url, &config.ckb_indexer_url);
    let assembler = AssemblerImpl::new(
        &rpc_client,
        &config.project_type_args,
        &config.project_code_hash,
    );
    let driver = DriverImpl::new(
        &rpc_client,
        &SecretKey::from_slice(&config.project_owner_privkey.0).unwrap(),
    );
    let executor = ExecutorImpl {};

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

    // running context
    let ctx = Context::new(assembler, executor, driver);
    tokio::select! {
        _ = ctrl_c_handler => {
            println!("<Ctrl-C> is on the call, quit knside-out drive loop");
        },
        Err(error) = ctx.start(&config.project_type_args, &config.project_cell_deps) => {
            println!("[Error] {}", error);
        }
    }
}
