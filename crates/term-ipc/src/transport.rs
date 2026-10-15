pub const WINDOWS_PIPE_NAME: &str = r"\\.\pipe\term_browser_ipc";
pub const UNIX_SOCKET_DEFAULT: &str = "/tmp/term_browser.sock";

/// Returns the platform-appropriate IPC socket address or pipe name
pub fn get_socket_path() -> String {
    #[cfg(target_os = "windows")]
    {
        WINDOWS_PIPE_NAME.to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("XDG_RUNTIME_DIR")
            .map(|dir| format!("{}/term_browser.sock", dir))
            .unwrap_or_else(|_| UNIX_SOCKET_DEFAULT.to_string())
    }
}
