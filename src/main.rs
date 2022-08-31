use std::convert::TryInto;
use std::panic::PanicInfo;

use clap::{crate_version, Arg, Command};
use ko_backend::BackendImpl;
use ko_context::ContextMgr;
use ko_protocol::{secp256k1::SecretKey, tokio, KoResult, ProjectDeps};
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
    let project_deps: &ProjectDeps = &config.as_ref().try_into().expect("config");

    // initail CKB rcp client
    let rpc_client = RpcClient::new(&config.ckb_url, &config.ckb_indexer_url);

    // initail driver context manager
    let manager_private_key =
        SecretKey::from_slice(config.project_manager_privkey.as_bytes()).expect("private key");
    let mut context_mgr = ContextMgr::new(
        &rpc_client,
        &manager_private_key,
        project_deps,
        &config.drive_settings,
    );
    let config_type_args = ko_config::load_type_args_file(".project_type_args.toml")?;
    context_mgr.recover_contexts(config_type_args.into());

    // initail rpc backend
    let backend = BackendImpl::new(&rpc_client, context_mgr);

    // start rpc server
    RpcServerRuntime::run(&config.rpc_endpoint, backend, project_deps).await?;

    // wait abort signal to exit
    let ctrl_c_handler = tokio::spawn(async {
        #[cfg(windows)]
        let _ = tokio::signal::ctrl_c().await;
        #[cfg(unix)]
        {
            use tokio::signal::unix as os_impl;
            let mut sigtun_int = os_impl::signal(os_impl::SignalKind::interrupt()).unwrap();
            let mut sigtun_term = os_impl::signal(os_impl::SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sigtun_int.recv() => {}
                _ = sigtun_term.recv() => {}
            };
        }
    });

    // register channel of panic
    let (panic_sender, mut panic_receiver) = tokio::sync::mpsc::channel(1);

    std::panic::set_hook(Box::new(move |info: &PanicInfo| {
        panic_sender
            .try_send(info.to_string())
            .expect("panic_receiver is droped");
    }));

    tokio::select! {
        _ = ctrl_c_handler => {
            println!(" [INFO] ctrl-c is pressed, quit knside-out")
        }
        Some(panic_info) = panic_receiver.recv() => {
            println!(" [INFO] child thread paniced: {}", panic_info)
        }
    }

    Ok(())
}
