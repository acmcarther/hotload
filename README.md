# Hotload

A small rust library to wrap a hotloadable dylib.

It comes with "Hotloader", a struct that auto hotloads a dylib, and a basic method proxy that can load, unload, and run a dylib.

Users can define their own method proxies, provided that they still support loading and unloading internal state.

## NOTE

This is actually *not* the authoritative source for this. The "in use" version of this lives in [next_space_coop](https://github.com/acmcarther/next_space_coop/tree/master/common/runtime_loading/loader).
