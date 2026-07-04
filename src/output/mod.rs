#[cfg(target_os = "linux")]
pub mod mouse_linux;
#[cfg(target_os = "linux")]
pub mod keyboard_linux;
#[cfg(target_os = "linux")]
pub use mouse_linux as mouse;
#[cfg(target_os = "linux")]
pub use keyboard_linux as keyboard;

#[cfg(target_os = "windows")]
pub mod mouse_windows;
#[cfg(target_os = "windows")]
pub mod keyboard_windows;
#[cfg(target_os = "windows")]
pub use mouse_windows as mouse;
#[cfg(target_os = "windows")]
pub use keyboard_windows as keyboard;
