# awe - Autonomi Website Publisher/Browser
This is a demonstration of website publishing and browsing on Autonomi, an autonomous secure peer-to-peer network.

IMPORTANT: `awe` has been discontinued in favour of `dweb` (see https://github.com/happybeing/dweb), which achieves everything demonstrated with awe, but using a standard browser.

One day, it may make sense to have a custom browser and some of the ideas demonstrated in awe which have to be done differently in a standard browser, may be resumed.

### awe demonstrates:
- *a perpetual world wide web* without gatekeepers, censorship or surveillance. Where every version of every website will be accesssible to everyone, forever.

- use of existing web frameworks and development techniques when building for the web on Autonomi. In fact you can publish the exact same content to the web, and to the web on Autonomi.

This project is being superceded by [dweb](https://github.com/happybeing/dweb) which is both a Rust library and command line app, including everything demonstrated here. However, `dweb` uses a standard browser to view decentralised websites on Autonomi rather than a built in GUI.

## Aims
- operation on Windows, MacOS, Linux and Android
- full feature operation of static websites built using any web framework and published on Autonomi
- ability to view historic versions of any website using Autonomi's perpetual, versioned storage

## Status

`awe` has served its purpose with most aims achieved and some learnings which have lead me to move away from a custom browser solution, to one based on using standard browsers (see next).

- features demonstrated on Linux & MacOS:
  - use the `awe` command line to publish and update static HTML websites, either to Autonomi or a local test network
  - use the `awe` demo browser to view published websites using their content address on the network, including every published version of a website
  - command line features to inspect data structures on Autonomi

- a web naming system was being considered but hasn't yet been designed for Autonomi. This would be similar to web DNS but decentralised and perpetual, and without renewal fees.

See also [Implementation & Learnings](https://github.com/happybeing/awe#implementation--learnings).

### Taking awe to the next level with dweb
All the features demonstrated in `awe` are already available in the [dweb project](https://github.com/happybeing/dweb), but using a standard web browser rather than a separate app with a webview.

`dweb` includes a command line program supporting the capabilities of `awe` and a library for developers wanting to re-use these capapabilities.

The ambitions for `dweb` go far beyond publishing and browsing websites on Autonomi. That is the only the first goal, and wasn't even part of the plan when I first began think about what to build following my learnings from `awe`.

Initially I had abandoned the idea of publishing and browsing websites and planned instead to create a backup application by providing a back-end for rclone. When I decided on an approach for doing that I realised it would allow many more capabilities, including those demonstrated by `awe`, but using a standard browser. I have so many ideas for `dweb` because the approach allows far more web capabilities to exist using secure, decentralised, pay-once storage of Autonomi.

## Capabilities Explained

Using `awe` you can publish static HTML websites to the public Autonomi network. The approach used allows you to use any web building framework without modification, and shows access to websites built with a range of tooling including Svelte, React Static and Publii. Literally any approach that generates static HTML will work, and you can view and debug your websites locally before publishing, just as you would when building websites at the moment.

Then `awe` can be used as a browser allowing anyone to view your websites on Autonomi. The browser functionality is very basic, but the data structures provide a template for others wanting to use or implement similar features.

### The Perpetual Web
If you publish updates to your sites, earlier versions remain available to be viewed by telling `awe` which version you want to access. This demonstrates a unique capability of Autonomi called *the perpetual web* where all public data, including public websites will remain accessible so long as you know where it resides. No more link rot!


## Implementation

### Tauri + Svelte + Rust
As well as demonstrating website publishing and viewing, `awe` is a showcase for a cross platform, desktop and mobile application written using Tauri with a Svelte frontend and Rust backend.

### dweb Rust Crate

`awe` relies on [dweb](https://github.com/happybeing/dweb), a Rust library that is being built using code tested first in `awe`. `dweb` aims to go far beyond `awe`, both as a library for developers and as a command line tool.


## Usage
### Pre-requisites
To publish a website on the public Autonomi network you will first need to have some Autonomi tokens available in a local wallet. Visit the Autonomi [community forum](https://forum.autonomi.community) for help with this and other issues.

### Publish a website
`awe publish-new --name <DWEB-NAME> --files-root <PATH-TO-CONTENT>`

The above will attempt to upload and pay for storage of your data and print the <HISTORY-ADDRESS> of the website on completion. If you keep that address you and anyone you share it with can view it by entering it in the `awe` browser address bar.

To open the `awe` browser, type `awe` without any subcommands. Or you can type `awe` followed by the address of the website you wish to view. The following displays the most recent version of the site:

`awe <HISTORY-ADDRESS>`

Or you can specify the version explicitly:

`awe <HISTORY-ADDRESS> --website-version <INTEGER-VERSION>`

Note:
- type `awe --help` for the full list of commands and
- type `awe <COMMAND> --help` for details of a command.

## Installation
**Linux:** for Linux, `.deb` and `.rpm` packages and an `.AppImage` are available at https://downloads.happybeing.com. The Debian image is built on Ubuntu 24.04 LTS and has been shown to work on Ubuntu 22, 23 and 24, and Mint. Please report any attempts to run other distros in an issue.

**MacOS:** MacOS builds will occasionally be uploaded to https://downloads.happybeing.com. You will also find instructions on how to build on MacOS on the Autonomi forum: [MacOS build of awe](https://forum.autonomi.community/t/macos-build-of-awe-web-publisher-browser-demo-app/40202?u=happybeing)

**Other:** I had planned support for Android and Windows but those are no longer planned with the move to `dweb` which is based on `awe`. That project aims to add support for other platforms.

## Implementation & Learnings
To deliver cross platform support along with the ability to access the Autonomi Rust APIs, this app is implemented using the Svelte web framework within a Tauri app. The design implements the application user interface in Svelte which loads web content into an `<iframe>` to display the static HTML stored on Autonomi. Website content is loaded using a Rust backend that accesses the Autonomi Rust API.

`awe` is both a web browser and a command line app, enabling it to publish and update websites as well as view them.

The learnings of the project include:
- it is feasible to use standard web tooling to create and view decentralised websites on Autonomi
- websites can be updated and every version remains available for viewing
- an `<iframe>` is a very limited viewer but adequate to demostrate basic browsing
- different platforms handle URL schemes differently, making it impractical to use a custom scheme across platforms
- a custom scheme could be supported in a custom browser (e.g. by a fork of Verso/Servo or Chromium)
- letting go of a custom scheme and using a localhost server (as in `dweb`) gives extra flexibility and capabilities

## Developing

This is a Tauri app built using the Svelte web framework and uses Rust for the backend.

Visit the Tauri v2 section of ([tauri.app](https://tauri.app)) for details of pre-requisites you may need to install. Then clone this repository and run the following command in the cloned directory.
```bash
cargo tauri dev
```

### Build on MacOS
There following forum topic shows how to build on MacOS, but note that the public key values mentioned there change when the Autonomi beta network is reset, so they may be out of date.

- [MacOS build of awe](https://forum.autonomi.community/t/macos-build-of-awe-web-publisher-browser-demo-app/40202?u=happybeing)

## LICENSE

Everything is AGPL3.0 unless otherwise stated. Any contributions are accepted on the condition they conform to this license.

See also [./LICENSE](./LICENSE)
