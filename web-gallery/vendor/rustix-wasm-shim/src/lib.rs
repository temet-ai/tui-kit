//! A stub `rustix` for `wasm32-unknown-unknown`.
//!
//! # Why this exists
//!
//! `ratatui-image` declares `rustix` unconditionally for every non-Windows
//! target:
//!
//! ```toml
//! [target."cfg(not(windows))".dependencies.rustix]
//! version = "^0.38.4"
//! features = ["stdio", "termios", "fs"]
//! ```
//!
//! `wasm32-unknown-unknown` matches `cfg(not(windows))`, but `rustix` 0.38 has
//! no backend for it: it falls through to the libc backend, whose `errno`
//! dependency answers with `compile_error!("The target OS is \"unknown\" or
//! \"none\", so it's unsupported by the errno crate.")`. That happens before
//! any of our code is reached, so no amount of feature selection in
//! `web-gallery` or `tui-kit` avoids it.
//!
//! # What it replaces
//!
//! `ratatui-image` 11.0.6 touches exactly four rustix items, all in code paths
//! that talk to a tty:
//!
//! - `termios::tcgetattr` / `termios::tcsetattr` (`picker.rs`, raw mode for the
//!   capability query)
//! - `termios::tcgetwinsize` (`picker.rs`, the font-size fallback that reads
//!   pixel dimensions off the terminal)
//! - `io::Errno` (`errors.rs`, one `From` impl)
//!
//! A browser has no tty, so every one of those is meaningless here. The stubs
//! below return [`io::Errno::NOTTY`], which is the honest answer and the same
//! answer the real calls would give on a detached process. `web-gallery` never
//! reaches them anyway: it builds its `Picker` with `Picker::from_fontsize`,
//! which states the cell size instead of querying for it.
//!
//! # What would remove it
//!
//! An upstream `ratatui-image` change narrowing that dependency to
//! `cfg(all(not(windows), not(target_arch = "wasm32")))`, together with the
//! matching `#[cfg]` on its four call sites. Until then this patch is the only
//! way to build anything depending on `ratatui-image` for the web.
//!
//! Scope: the `[patch.crates-io]` entry that installs this applies only to the
//! standalone `web-gallery` workspace. The repo-root workspace never sees it,
//! so the native `tui-kit` build keeps the real `rustix`.

#![no_std]

/// The subset of `rustix::io` that `ratatui-image` names.
pub mod io {
    /// A stub error code. The real type wraps a raw OS errno; here it carries
    /// one so the `std::io::Error` conversion stays faithful.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Errno(pub i32);

    impl Errno {
        /// `ENOTTY` — "inappropriate ioctl for device", the errno a terminal
        /// call gets when the file descriptor is not a terminal. On wasm
        /// nothing is.
        pub const NOTTY: Self = Self(25);
    }

    impl core::fmt::Display for Errno {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "os error {}", self.0)
        }
    }

    // `ratatui-image`'s `errors.rs` does `std::io::Error::from(errno)`, an impl
    // the real rustix provides. `Errno` is local to this crate, so defining it
    // for the foreign `std::io::Error` is allowed.
    impl From<Errno> for std::io::Error {
        fn from(errno: Errno) -> Self {
            std::io::Error::from_raw_os_error(errno.0)
        }
    }
}

/// The subset of `rustix::termios` that `ratatui-image` names.
pub mod termios {
    use crate::io::Errno;

    /// Terminal local-mode flags. Only the bit positions matter to callers
    /// (they clear `ICANON`/`ECHO` on a value we produced), so plain bitflags
    /// over a `u32` suffice.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct LocalModes(pub u32);

    impl LocalModes {
        /// Canonical (line-buffered) input.
        pub const ICANON: Self = Self(0x0000_0100);
        /// Echo input characters.
        pub const ECHO: Self = Self(0x0000_0008);
    }

    impl core::ops::Not for LocalModes {
        type Output = Self;
        fn not(self) -> Self {
            Self(!self.0)
        }
    }

    impl core::ops::BitAnd for LocalModes {
        type Output = Self;
        fn bitand(self, rhs: Self) -> Self {
            Self(self.0 & rhs.0)
        }
    }

    impl core::ops::BitAndAssign for LocalModes {
        fn bitand_assign(&mut self, rhs: Self) {
            self.0 &= rhs.0;
        }
    }

    impl core::ops::BitOr for LocalModes {
        type Output = Self;
        fn bitor(self, rhs: Self) -> Self {
            Self(self.0 | rhs.0)
        }
    }

    impl core::ops::BitOrAssign for LocalModes {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0;
        }
    }

    /// Terminal attributes. Only `local_modes` is read or written by
    /// `ratatui-image`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Termios {
        /// The `c_lflag` field of the real `struct termios`.
        pub local_modes: LocalModes,
    }

    /// When a `tcsetattr` takes effect.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OptionalActions {
        /// Apply immediately.
        Now,
        /// Apply after pending output drains.
        Drain,
        /// Apply after draining, discarding pending input.
        Flush,
    }

    /// Terminal window size, in both cells and pixels.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Winsize {
        /// Height in cells.
        pub ws_row: u16,
        /// Width in cells.
        pub ws_col: u16,
        /// Width in pixels (0 when the terminal does not report it).
        pub ws_xpixel: u16,
        /// Height in pixels (0 when the terminal does not report it).
        pub ws_ypixel: u16,
    }

    /// Always `Err(Errno::NOTTY)`: there is no terminal behind a wasm fd.
    ///
    /// Generic over the argument rather than `AsFd`-bound, because the fd
    /// traits are the part of `std::os` that `wasm32-unknown-unknown` does not
    /// have — and the value is discarded regardless.
    pub fn tcgetattr<Fd>(_fd: Fd) -> Result<Termios, Errno> {
        Err(Errno::NOTTY)
    }

    /// Always `Err(Errno::NOTTY)`. See [`tcgetattr`].
    pub fn tcsetattr<Fd>(_fd: Fd, _actions: OptionalActions, _termios: &Termios) -> Result<(), Errno> {
        Err(Errno::NOTTY)
    }

    /// Always `Err(Errno::NOTTY)`. `ratatui-image` calls this only as a
    /// font-size fallback and handles the error by returning `None`.
    pub fn tcgetwinsize<Fd>(_fd: Fd) -> Result<Winsize, Errno> {
        Err(Errno::NOTTY)
    }
}

// `std` is needed for the one `std::io::Error` conversion above; the rest of
// the crate is `no_std`.
extern crate std;
