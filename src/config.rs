//! The port's config file, `config/industrialforegoingsouls/config.json`.
//!
//! Upstream keeps its numbers in Titanium's config (`ConfigSoulLaserBase`, `ConfigSoulSurge`,
//! `IFSoulsMachines`, `IFSoulsClient`), which NeoForge loads through `ModContainer.registerConfig`
//! and announces with `ModConfigEvent.Loading`. The pin has the same pair — `register_configs`
//! writes the default file when it is missing and hands back the text the game read, and
//! `config_loading`/`config_reloading` carry it — so this module holds only the value in force.
//!
//! **It is read once, at load, and never from a tick.** The engine re-reads a file that changes on
//! disk and announces it through `config_reloading`; a block entity's tick only copies this value
//! out of the lock, so a machine costs one uncontended read and no file I/O.
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;

use crate::runtime;

/// The file's path, relative to the game's `config/` folder: a folder of this mod's own, which is
/// what the pin lets a mod own (`ConfigPath::owned_by`) and the shape this port settled on, since
/// the jar's own file names are Titanium's (`industrialforegoing-souls-machines`), which a mod id
/// of `industrialforegoingsouls` cannot claim.
pub const CONFIG_FILE: &str = "industrialforegoingsouls/config.json";

/// The values in force: the jar's defaults until the game reads the file.
static CONFIG: RwLock<runtime::Settings> = RwLock::new(runtime::Settings::DEFAULT);

fn reading() -> RwLockReadGuard<'static, runtime::Settings> {
    CONFIG
        .read()
        .expect("the config lock is not poisoned: nothing panics while holding it")
}

fn writing() -> RwLockWriteGuard<'static, runtime::Settings> {
    CONFIG
        .write()
        .expect("the config lock is not poisoned: nothing panics while holding it")
}

/// The values in force, as the rules take them.
///
/// [`runtime::Settings`] is `Copy`, so this is one short read lock and a copy — safe to call from a
/// block entity's tick.
#[must_use]
pub fn current() -> runtime::Settings {
    *reading()
}

/// Takes the text the game loaded for this mod (`config_loading`, `config_reloading`).
pub fn loaded(text: &str) {
    *writing() = runtime::parse_config(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file the game is asked to write is the one this port parses, and a text that is not this
    /// file's shape leaves the jar's defaults in force rather than clearing them.
    #[test]
    fn a_loaded_text_becomes_the_value_in_force() {
        loaded(&runtime::format_config(&runtime::Settings::DEFAULT));
        assert_eq!(current(), runtime::Settings::DEFAULT);
        loaded(r#"{"soul_storage": 500, "souls_per_operation": 5}"#);
        assert_eq!(current().soul_storage, 500);
        assert_eq!(current().souls_per_operation, 5);
        assert_eq!(current().max_progress, runtime::MAX_PROGRESS, "the rest");
        loaded("not a config file");
        assert_eq!(
            current(),
            runtime::Settings::DEFAULT,
            "garbage is the defaults"
        );
        // Leave the process's shared value at the defaults for any other test that reads it.
        loaded(&runtime::format_config(&runtime::Settings::DEFAULT));
    }

    /// The file is the folder form the pin owns (`<mod id>/…`), not the jar's Titanium file name,
    /// which a mod id of `industrialforegoingsouls` could not claim.
    #[test]
    fn the_file_lives_in_this_mods_own_folder() {
        assert_eq!(CONFIG_FILE, "industrialforegoingsouls/config.json");
        assert!(CONFIG_FILE.starts_with("industrialforegoingsouls/"));
    }
}
