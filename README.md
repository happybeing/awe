# Autonomi Browser Demo
This is a demonstration of a web browser for Autonomi, an autonomous secure peer-to-peer network.

**Status:** exploratory - don't expect anything useful yet!

As a demonstration, this app will only provide minimal web browser functionality. It is part of a project to demonstrate how to publish and browse static websites using Autonomi.

## Aims:
- operation on Windows, MacOS, Linux and Android
- full feature operation of static websites built using any web framework and published on Autonomi
- ability to view historic versions of any website using Autonomi's perpetual, versioned storage

## Implementation
To deliver cross platform support along with the ability to access the Autonomi Rust APIs, this app is implemented using the Svelte web framework within a Tauri app. The design implements the application user interface in Svelte which loads web content into an `<iframe>` to display the static HTML stored on Autonomi. Website content is loaded using a Rust backend that accesses the Autonomi Rust API.

An accompanying command line app will be created to publish a static website on Autonomi in a format that supports the above aims.

The initial implementation will load website using a custom URL protocol to provide address the appropriate data using Autonomi's xor based content addressing. Later, support will be added for a naming system, similar to www domains, that will be resolved by the browser to obtain the xor address of the web content.

## Developing

This is a Tauri app built using the Svelte web framework. For more details visit [tauri.app](https://tauri.app) for more.
```bash
cargo tauri dev
```

## LICENSE

Everything is GPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also [./LICENSE](./LICENSE)
