<script>
import { event } from "@tauri-apps/api";
import WebsiteView from "./WebsiteView.svelte";

import {webAddressStore} from '$lib/stores';

let val = 'http://autonomi.com';

function handleSubmit() {
  webAddressStore.set(val);
  console.log('submitting: ' + val);
}

</script>

<div>
<form class="row" id="address-bar" on:submit|preventDefault={handleSubmit}>
  Type an xor:// or http:// URL and press enter: <input bind:value={val} placeholder="autonomi website address" />
</form>
</div>
