<script>
import logo from '../images/icon.png';

let webViewTitle="Autonomi Browser Webview";
let webViewContent = "<HTML><HEAD></HEAD><BODY><h1>Default</h1> content</BODY></HTML>";
let webViewUrl = '';

let rowNavigationHeight = "20px";

// Test URLs
let val = 'http://autonomi.com';

// The following need re-uploading to each new testnet with "safe files upload -p"
// Don't forget '-p' to make public
val = 'axor://9ff3c8719f0e228f86621e3afa20cb533434d7f9f76ec2cbe8d3ab637757d483'; // Traktion's blog
val = 'axor://86f7f345ce751485e06ed42792b93794572eec71a2bde0ec284327ecc2e41f04'; // aweb-sites/site1/index.html

function handleSubmit() {
  webViewUrl = val;
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

</script>

<style>
.container {
  height: vh;
  width: 100%;
  white-space: nowrap;
  overflow: hidden;
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
    <button class="button" on:click={handleBackButton}>&lt;</button>
    <button class="button" on:click={handleBackButton}>&gt;</button>
<form class="input-form" on:submit|preventDefault={handleSubmit}>
  Type an axor:// URL and press enter:<input class="input" type=text bind:value={val} placeholder="autonomi address" />
</form>
</div>

<div class="row-view iframe-wrapper" >
  <iframe id="webframe" bind:this={webframe} src={webViewUrl} name="webview" title={webViewTitle}></iframe>
</div>