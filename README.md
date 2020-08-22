[![Build Status](https://travis-ci.com/finnkauski/lighthouse.svg?branch=main)](https://travis-ci.com/finnkauski/lighthouse)
<p align="left"><img align="left" src="meta/logo.png" width="240px"></p>

# Lighthouse

Control your Philips Hue lights with this API wrapper! 

**NOTE:**
This wrapper is under active redevelopment, see the older commits in order to get
the previous iterations of the API. Also this is not a complete API wrapper for the HUE API as I do not have the time to expand the wrapper. If you would like to contribute please consider making a PR.

## Also see:

[lighthouse.el](https://github.com/finnkauski/lighthouse.el) - an Emacs package
wrapping the functionality of `lighthouse` (uses older version of the library)

[lightshow](https://github.com/finnkauski/lightshow) - A simple scripting language
allowing you to script your lights into lightshows (uses older version of the library)

## Usage

Adding the dependency:

```toml
[dependencies]
lighthouse = "0.2"
```

And then in your application:

```rust
# Acquire your IP address and substitute here
let ip_addr = std::net::Ipv4Addr::new(192.168.1.10);
# Get an API token from your bridge, requires proof of physical access
let bridge_token = String::new("my-example-token");
let bridge = Bridge::new(ip_addr, bridge_token).unwrap();
let lights = bridge.scan();
```

**NOTE:**
The features for color conversion and serialisation to and from files are now behind 
feature flags. Available flags are:
- color - adds the color conversion module
- persist - adds the ability to serialise to and from files and also to create bridges from environment variables

## Command line tool

The previous releases of this library came with a binary that allowed users to control their lights from the command line.
The crate has been refactored and simplified. The binary will have to be refactored as well. However the priority is to
finished a more sensible API wrapper before moving onto the binary.

## Contributions

I don't have the time to wrap absolutely all the endpoints and the data structures required for the API.

I would really love people to chip in over time and keep adding new functionality through extra endpoints wrapped.
