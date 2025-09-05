mod deploy_coprocessor_app;
mod instantiate_contracts;
mod read_input;
mod setup_authorizations;
mod write_output;

pub use deploy_coprocessor_app::deploy_coprocessor_app;
pub use instantiate_contracts::instantiate_contracts;
pub use read_input::*;
pub use setup_authorizations::setup_authorizations;
pub use write_output::write_setup_artifacts;
