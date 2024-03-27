<script>
import logo from '../images/icon.png';

let webViewTitle="Autonomi Browser Webview";
let webViewContent = "<HTML><HEAD></HEAD><BODY><h1>Default</h1> content</BODY></HTML>";
let webViewUrl = '';

let rowNavigationHeight = "20px";

function handleBack() {}
function handleForward() {}

let val = 'http://autonomi.com';

function handleSubmit() {
  webViewUrl = val;
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
    <button class="button" onclick="button1Function()">&lt;</button>
    <button class="button" onclick="button2Function()">&gt;</button>
<form class="input-form" on:submit|preventDefault={handleSubmit}>
  Type an xor:// or http:// URL and press enter:<input class="input" type=text bind:value={val} placeholder="autonomi address" />
</form>
</div>

<div class="row-view iframe-wrapper" >
  <iframe src={webViewUrl} name="webview" title={webViewTitle}></iframe>
</div>