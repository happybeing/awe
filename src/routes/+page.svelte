<script>
import {onMount} from 'svelte';
import logo from '../images/icon.png';
import { invoke } from '@tauri-apps/api/tauri'; // For calling Rust


import {builtinsPublic} from '../generated/builtins-public.js';
import {builtinsLocal} from '../generated/builtins-local.js';

let webViewTitle="Autonomi Browser Webview";
let webViewContent = "<HTML><HEAD></HEAD><BODY><h1>Default</h1> content</BODY></HTML>";
let webViewUrl = '';

let rowNavigationHeight = "20px";

const DEFAULT_PROTOCOL = "awx://";

let addressBar = '';
let versionInput = 0;  // 0 implies load the default/most recent version
let firstVersion = 1;
let maxVersion = 1;
let viewLoaded = false;
let builtins;

// $maxVersion: if (versionInput > maxVersion) { versionInput = maxVersion};

onMount(() => {
  console.log("onMount()");

  load_builtins().then((result) => {
    console.log("load_builtins() returned");
    console.log(result);
    builtins = result;
    if (builtins.awe == null) {
      builtinsVisibility = 'hidden';
    } else {
      builtinsVisibility = 'visible';
    }
  });

  // Before website loaded, this will be the CLI provided version if present
  invoke('on_get_version_requested').then((version_requested) => {
    console.log('on_get_version_requested() returned ' + version_requested);
    if (version_requested > 0) {
      versionInput = version_requested;
    }
  });

  invoke('on_start_get_cli_url').then((url) => {
    console.log('on_start_get_cli_url() returned ' + url);
    if (url.length > 0) {
      addressBar = url;
      loadNewPage();
    }
  });

});

/**
 * @param {KeyboardEvent} e
 */
function onAddressBarKeypress(e) {
  console.log("onUrlOrVersionKeypress " + e.code);
  if (e.code == "Enter") {
    let v = getVersionFromAddressBar();
    if (v > 0) {
      versionInput = v;
    }

    loadNewPage();
  }
}

/**
 * @param {KeyboardEvent} e
 */
function onVersionNumberKeypress(e) {
  console.log("onVersionNumberKeypress " + e.code);
  if (e.code == "Enter") {
    loadNewPage();
  }
}

/**
 * // @param {{ code: string; }} e
 */
function onVersionChange(e) {
  console.log("onVersionChange()");
  if (e.code == "Enter") {
    loadNewPage();
  }
}

/**
 * Set/modify the version parameter string in the addressBar
 *
 * @param {string} v
 */
function setUrlVersion(v) {
  console.log("setUrlVersion() v = " + v);
  console.log("    BEFORE: " + addressBar);

  try {
    let url = new URL(addressBar);
    url.searchParams.set('v', v);
    addressBar = url.toString();
  } catch (error) {console.log(error);}

  console.log("    AFTER: " + addressBar);
}

/**
 * Get a version value the 'v=' parameter in the addressBar
 *
 * @returns {Number} or 0 if parameter not present
 */
function getVersionFromAddressBar() {
  console.log("getUrlVersion()");
  console.log("    addressBar: " + addressBar);

  let value = 0;
  try {
    let url = new URL(addressBar);
    let vParam = url.searchParams.get('v');
    if (vParam != null) {
      value = Number.parseInt(vParam);
      console.log
    }
  } catch (error) {console.log(error);}

  console.log("    got version: " + value);
  return value;
}

// To be called from onUrlOrVersionKeypress() when Enter is pressed
function loadNewPage() {
  console.log('loadNewPage()')
  viewLoaded = false;

  // pass website_version to Rust
  let newVersion = versionInput;
  console.log('    versionInput: ' + versionInput);
  console.log('    newVersion: ' + newVersion);
  webViewUrl = '';
  invoke('on_frontend_set_version', { frontendVersion: newVersion }).then((versionApplied) => {
    console.log('    on_frontend_set_version() returned ' + versionApplied)
    if (versionInput != versionApplied) {versionInput = versionApplied; }
    if (addressBar.indexOf("://") == -1 ){
      addressBar = DEFAULT_PROTOCOL + addressBar;
    }
    setUrlVersion(versionInput.toString());
    webframe.src = '/';
    console.log("    loading... " + addressBar);
    webframe.src = addressBar;
  });
}

function onViewLoaded() {
  console.log("onViewLoaded()");
  console.log("    webViewUrl: " + webViewUrl);
  console.log("    addressBar: " + addressBar);
  invoke('on_get_version_max').then((result) => {
    console.log('    on_get_version_max() returned ' + result);
    maxVersion = result;
  });

  invoke('on_get_version_loaded').then((result) => {
    console.log('    on_get_version_loaded() returned ' + result);
    versionInput = result;
    viewLoaded = true;
    setUrlVersion(versionInput.toString());
  });
}

// TODO fix cross-origin block using CSP: SecurityError: Blocked a frame with origin "http://localhost:5173" from accessing a cross-origin frame. Protocols, domains, and ports must match.
/** @type {HTMLIFrameElement} */
	let webframe;
function handleBackButton() {
  webframe.contentDocument?.defaultView?.history.back();
  // document.getElementById('webframe').back();
//  webframe.contentWindow?.history.back();
}

function handleForwardButton() {
  webframe.contentWindow?.history.forward();
}

async function load_builtins() {
  console.log("load_builtins()");
  if (await invoke('on_is_local_network')) {
    console.log("on_is_local_network() returned true");
    return builtinsLocal;
  } else {
    console.log("on_is_local_network() returned false");
    return builtinsPublic;
  }
}

let builtinsVisibility = 'visible';

function handleBuiltinsButton() {
  console.log("handleBuiltinsButton()")
  if (builtins != null) {
    let url = builtins.aweSomeSites.url;
    console.log("   loading url:", url);
    addressBar = url;
    loadNewPage();
  }
}

</script>

<style>
.container {
  height: vh;
  width: 100%;
  white-space: nowrap;
  overflow: hidden;
}

.builtins-button {
  height: 4ch;
  /* width: 4ch; */
}
.button {
  visibility: hidden; /* TODO implement tracking of history and use to set iframe.src */
  height: 4ch;
  width: 4ch;
}
.button,
.input-form {
  display: inline-block;
  height: 32px;
  margin-top: 4px;
  margin-bottom: 4px;
  margin-left: 4px;
  line-height: 3ch;
}

.input-form {
  position: relative;
}

input[type="text"] {
  height: 34px;
  font-size: 1.2em;
  padding-left: 8px;
  margin-left: 8px;
}

.iframe-wrapper iframe,
.iframe-wrapper object,
.iframe-wrapper embed {
  height: calc(100vh - 50px); /* Prevent vertical scrolling of everything inside body */
  width: 100%;
  border: 0;
}

</style>

<div class="container">
  <button class="builtins-button" style="visibility: {builtinsVisibility}" on:click={handleBuiltinsButton}>Click Me!</button>
  <button class="button" on:click={handleBackButton}>&lt;</button>
  <button class="button" on:click={handleBackButton}>&gt;</button>
    Type awx:// URL and press enter:<input class="input" type=text bind:value={addressBar} placeholder="autonomi address"  on:keypress={onAddressBarKeypress} />
  Site version:<input disabled={!viewLoaded} class="" type=number bind:value={versionInput} on:change={onVersionChange} on:keypress={onVersionNumberKeypress}/>
</div>

<div class="row-view iframe-wrapper" >
  <iframe id="webframe" bind:this={webframe} on:load={onViewLoaded} src={webViewUrl} name="webview" title={webViewTitle}></iframe>
</div>