<p align="left"><img align="left" src="meta/logo.png" width="240px"></p>

# Lighthouse

Control your Philips Hue lights with this API wrapper!

This wrapper is under active redevelopment, see the older commits in order to get
the previous iterations of the API.

Current goals is to slim this down, make this more async and to make a nicer command line
tool.

## Also see:

[lighthouse.el](https://github.com/finnkauski/lighthouse.el) - an Emacs package
wrapping the functionality of `lighthouse` (uses older version of the library)

[lightshow](https://github.com/finnkauski/lightshow) - A simple scripting language
allowing you to script your lights into lightshows (uses older version of the library)

## Usage

Simply add `lighthouse` to the `Cargo.toml` and simply go from there.

## Command line tool

The previous releases of this library came with a binary that allowed users to control their lights from the command line.
The crate has been refactored and simplified. The binary will have to be refactored as well. However the priority is to
finished a more sensible API wrapper before moving onto the binary.

## Contributions

I don't have the time to wrap absolutely all the endpoints and the data structures required for the API.

I would really love people to chip in over time and keep adding new functionality through extra endpoints wrapped.
