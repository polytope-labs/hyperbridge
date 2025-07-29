mod hyperbridge_client;
mod pallet_ismp;
mod pallet_mmr;
mod token_allocation;

pub fn setup_logging()  {
    env_logger::builder()
        .format_module_path(false)
        .init();
}
