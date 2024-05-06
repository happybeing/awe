# awe - Autonomi Website Publisher/Browser
This is a demonstration of website publishing and browsing on Autonomi, an autonomous secure peer-to-peer network.

It demonstrates:
- *a perpetual world wide web* without gatekeepers, censorship or surveillance. Where every version of every website will be accesssible to everyone, forever.

- use of existing web frameworks and development techniques when building for the web on Autonomi. In fact you can publish the exact same content to the web, and to the web on Autonomi.

- cross platform universal access to the web on Autonomi, including from desktop and mobile devices.

## Aims
- operation on Windows, MacOS, Linux and Android
- full feature operation of static websites built using any web framework and published on Autonomi
- ability to view historic versions of any website using Autonomi's perpetual, versioned storage

## Status
- early alpha on Linux - core features so far:
  - use the `awe` command line to publish and update static HTML websites, either to Autonomi or a local test network
  - use the `awe` demo browser to view published websites using their content address on the network, including every published version of a website


- you can build for other platforms yourself, or await:
  - Windows (coming soon)
  - MacOS (I need someone else to build this)
  - Android (coming later)
- a web naming system is being looked into for Autonomi (similar to web DNS but decentralised and perpetual, without renewal fees)

## Capabilities Explained

Using `awe` you can publish static HTML websites to the public Autonomi network. The approach used allows you to use any web building framework without modification, and shows access to websites built with a range of tooling including Svelte, React Static and Publii. Literally any approach that generates static HTML will work, and you can view and debug your websites locally before publishing, just as you would when building websites at the moment.

Then `awe` can be used as a browser allowing anyone to view your websites on Autonomi. Also, if you publish updates to your sites, earlier versions remain available to be viewed by telling `awe` which version you want to access.

This demonstrates a unique capability of Autonomi called *the perpetual web* where all public data, including public websites will remain accessible so long as you know where it resides. No more link rot!

The browser functionality is very basic, but the data structures provide a template for others wanting to use or implement similar features.

## Implementation

### Tauri + Svelte + Rust
As well as demonstrating website publishing and viewing, `awe` is a showcase for a cross platform, desktop and mobile application written using Tauri with a Svelte frontend and Rust backend.

### awe Rust Crate

In due course `awe` will be published as a Rust library on `crates.io` for those wanting to build versioned website publishing and viewing applications the same approach.

## Usage
### Pre-requisites
To publish a website on the public Autonomi network you will first need to have some Autonomi tokens available in a local wallet. These are available free for public testnets details of which are available from the Autonomi [community forum](https://forum.autonomi.community), along with a friendly and helpful community to get you started.

### Publish a website
`awe publish --website-root <PATH-TO-CONTENT>`

The above will attempt to upload and pay for storage of your data and print the xor-url of the website on completion. If you keep that address you and anyone you share it with can view it by entering it in the `awe` browser address bar.

To open the `awe` browser, type `awe` without any subcommands. Or you can type `awe` followed by the address of the website you wish to view, and if you like you can also specify the version you want:

`awe <XOR-URL> --website-version <INTEGER-VERSION>`

Note:
- type `awe --help` for the full list of commands and
- type `awe <COMMAND> --help` for details of a command.

## Installation
**Linux:** for Linux, a `.deb` package and an `.AppImage` are available in the releases section. The Debian image is built on Ubuntu 22.04 LTS and so should support that and later version of Ubuntu. Please report any attempts to run other distros in an issue.

**Other:** I plan support for Android, Windows and MacOS so if you would like to test these please get in touch via github or the [community forum](https://forum.autonomi.community). You'll find me as `@happybeing` on both.

For Windows and MacOS you should be able to build and run from source, but this has not been tested yet.

I'll be working on Android in due course.

## Implementation
To deliver cross platform support along with the ability to access the Autonomi Rust APIs, this app is implemented using the Svelte web framework within a Tauri app. The design implements the application user interface in Svelte which loads web content into an `<iframe>` to display the static HTML stored on Autonomi. Website content is loaded using a Rust backend that accesses the Autonomi Rust API.

'awe' is both a web browser and a command line app, enabling it to publish and update websites as well as view them.

## Developing

This is a Tauri app built using the Svelte web framework and uses Rust for the backend.

Visit the Tauri v1 website ([tauri.app](https://tauri.app)) for details of pre-requisites you may beed to install. Then clone this repository and run the following command in the cloned directory.
```bash
cargo tauri dev
```

## LICENSE

Everything is GPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also [./LICENSE](./LICENSE)
