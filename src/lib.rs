// Valence co-processor application main library file
// Acts as a central point for re-exporting functionality from the core crate

pub use valence_coprocessor_app_core as core;
pub use valence_coprocessor_app_lib as lib;

// Export commonly used types and functions
pub mod prelude {
    pub use crate::core::*;
} 