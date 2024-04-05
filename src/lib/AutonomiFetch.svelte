<script>
import { invoke } from '@tauri-apps/api/tauri'

let xorAddress = ''
let fetchBody = ''

async function greet() {
  fetchBody = await invoke('autonomi_fetch', { xorAddress })
}
</script>

<div>
<form class="row" id="automoni-fetch-form">
  <input id="autonomi-fetch-input" on:submit="{greet}" placeholder="autonomi website address" bind:value="{xorAddress}" />
  <p>{fetchBody}</p>
</form>
</div>