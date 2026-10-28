pub mod api;
#[cfg(feature = "flutter")]
mod frb_generated;
pub mod icons;
pub mod model;
pub mod platform;
pub mod reconcile;
pub mod search;
pub mod settings;
pub mod store;
pub mod usage;

// Keeps the JNI entry points in the cdylib so the JVM can resolve them.
#[cfg(target_os = "android")]
#[allow(unused_imports)]
pub use crate::platform::android::jni_host::{
    JNI_OnLoad, Java_bitshift_qlaunch_MainActivity_onBackPressedEvent,
    Java_bitshift_qlaunch_MainActivity_onHomePressed,
    Java_bitshift_qlaunch_MainActivity_onHomePressedFromOverlay,
};
