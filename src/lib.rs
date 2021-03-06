extern crate libc;
extern crate libloading;

#[cfg(unix)]
use libloading::os::unix::Symbol;
#[cfg(windows)]
use libloading::os::windows::Symbol;

use libloading::Library;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time;
use std::fs;

/** A user defined type that wraps a dynamically loaded library. */
pub trait HotloadProxy {
  fn initialize_fresh(lib: Library) -> Self;
  fn initialize(lib: Library, state: *mut libc::c_void) -> Self;
  fn dump_state(self) -> *mut libc::c_void;
}

/** A basic proxy proxying "initialize", "run", and "dump_state" to a dylib */
pub struct BasicProxy {
  lib: Library,
  run_fn: Option<Symbol<fn()>>,
}

impl BasicProxy {
  /** Set the state of the internal dylib */
  fn initialize_internal(&mut self, state: *mut libc::c_void) {
    unsafe {
      self.lib.get::<fn(*mut libc::c_void)>("initialize".as_bytes()).unwrap()(state)
    }
  }

  /** Run the dylib's run function using the cached symbol */
  pub fn run(&mut self) {
    self.run_fn.as_mut().unwrap()();
  }
}

impl HotloadProxy for BasicProxy {
  /** Build a brand new proxy from a library */
  fn initialize_fresh(lib: Library) -> BasicProxy {
    BasicProxy {
      run_fn: Some(unsafe { lib.get::<fn()>("run".as_bytes()).unwrap().into_raw() }),
      lib: lib
    }
  }

  /** Build a brand proxy from a prior opaque state */
  fn initialize(lib: Library, state: *mut libc::c_void) -> BasicProxy{
    let mut proxy = BasicProxy::initialize_fresh(lib);
    proxy.initialize_internal(state);
    proxy
  }

  /** Dump the opaque state from the contained dylib */
  fn dump_state(mut self) -> *mut libc::c_void {
    drop(self.run_fn.take()); // ensure run_fn is dropped before contained lib
    unsafe {
      self.lib.get::<fn() -> *mut libc::c_void>("dump_state".as_bytes()).unwrap()()
    }
  }
}

/** A struct containing a hotloadable library object, and a user defined method proxy */
pub struct Hotloader<T: HotloadProxy> {
  proxy: Option<T>,
  watched_path: PathBuf,
  last_modified: SystemTime,
}

impl <T: HotloadProxy> Hotloader<T> {
  /** Build a fresh hotloader from a path to a dylib */
  pub fn new(dylib_path: PathBuf) -> Hotloader<T> {
    let mut hotloader = Hotloader {
      proxy: None,
      watched_path: dylib_path,
      last_modified: time::UNIX_EPOCH
    };

    hotloader.try_hotload();
    hotloader
  }

  /** Get the *optional* method proxy. It may not be present if hotloading fails */
  pub fn get_proxy(&mut self) -> Option<&mut T> {
    self.try_hotload();
    self.proxy.as_mut()
  }

  /** Attempt a hotload or do nothing */
  fn try_hotload(&mut self) {
    let new_last_modified = fs::metadata(self.watched_path.clone()).and_then(|v| v.modified())
      .ok()
      .unwrap_or(self.last_modified);
    if new_last_modified <= self.last_modified {
      return;
    }

    // Take state from proxy and drop dylib (so we can reload it)
    // see docs on dlopen for why dropping is necessary
    let last_state = self.proxy.take().map(|c| c.dump_state());

    let library = Library::new(self.watched_path.clone()).ok();
    let new_proxy = match (last_state, library) {
      (_, None) => None,
      (None, Some(lib)) => Some(T::initialize_fresh(lib)),
      (Some(state), Some(lib)) => Some(T::initialize(lib, state))
    };

    self.proxy = new_proxy;
    self.last_modified = new_last_modified;
  }
}
